use crate::{
    error::RecursorError,
    forwarder::Forwarder,
    nsas::{NSAddressStore, Nameserver},
};
use failure;
use futures::{prelude::*, Future};
use r53::{Message, MessageRender};
use std::{
    error::Error,
    mem,
    net::SocketAddr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::{
    net::{udp, UdpSocket},
    timer::Delay,
    util::FutureExt,
};

const DEFAULT_RECV_TIMEOUT: Duration = Duration::from_secs(1); //3 secs
const DEFAULT_RECV_BUF_SIZE: usize = 1024;

enum State {
    Init,
    Send(udp::SendDgram<Vec<u8>>),
    Recv(udp::RecvDgram<Vec<u8>>, Delay, Instant),
    Poisoned,
}

pub struct Sender {
    query: Message,
    nameserver: Nameserver,
    nsas: Arc<NSAddressStore<Forwarder>>,
    state: State,
}

impl Sender {
    pub fn new(
        query: Message,
        nameserver: Nameserver,
        nsas: Arc<NSAddressStore<Forwarder>>,
    ) -> Self {
        Sender {
            query,
            nameserver,
            nsas,
            state: State::Init,
        }
    }
}

impl Future for Sender {
    type Item = Message;
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match mem::replace(&mut self.state, State::Poisoned) {
                State::Init => {
                    let mut render = MessageRender::new();
                    self.query.rend(&mut render);
                    let socket =
                        UdpSocket::bind(&("0.0.0.0:0".parse::<SocketAddr>().unwrap())).unwrap();
                    let target = SocketAddr::new(self.nameserver.address, 53);
                    self.state = State::Send(socket.send_dgram(render.take_data(), &target));
                }
                State::Send(mut fut) => match fut.poll() {
                    Err(e) => {
                        self.nameserver.set_unreachable();
                        self.nsas.update_nameserver_rtt(&self.nameserver);
                        return Err(RecursorError::IoError(e).into());
                    }
                    Ok(Async::NotReady) => {
                        self.state = State::Send(fut);
                        return Ok(Async::NotReady);
                    }
                    Ok(Async::Ready((socket, _))) => {
                        self.state = State::Recv(
                            socket.recv_dgram(vec![0; DEFAULT_RECV_BUF_SIZE]),
                            Delay::new(Instant::now().checked_add(DEFAULT_RECV_TIMEOUT).unwrap()),
                            Instant::now(),
                        );
                    }
                },
                State::Recv(mut fut, mut delay, send_time) => match fut.poll() {
                    Err(e) => {
                        self.nameserver.set_unreachable();
                        self.nsas.update_nameserver_rtt(&self.nameserver);
                        return Err(RecursorError::IoError(e).into());
                    }
                    Ok(Async::NotReady) => match delay.poll() {
                        Err(e) => {
                            return Err(RecursorError::TimerErr(e.description().to_string()).into());
                        }
                        Ok(Async::Ready(_)) => {
                            self.nameserver.set_rtt(DEFAULT_RECV_TIMEOUT);
                            self.nsas.update_nameserver_rtt(&self.nameserver);
                            return Err(RecursorError::Timeout(self.nameserver.address).into());
                        }
                        Ok(Async::NotReady) => {
                            self.state = State::Recv(fut, delay, send_time);
                            return Ok(Async::NotReady);
                        }
                    },
                    Ok(Async::Ready((_, buf, size, _))) => {
                        self.nameserver.set_rtt(send_time.elapsed());
                        self.nsas.update_nameserver_rtt(&self.nameserver);
                        match Message::from_wire(&buf[..size]) {
                            Ok(resp) => {
                                return Ok(Async::Ready(resp));
                            }
                            Err(err) => {
                                return Err(err);
                            }
                        }
                    }
                },
                State::Poisoned => panic!("inside sender pool"),
            }
        }
    }
}
