use futures::{future, Future};
use r53::{Message, MessageRender};
use server::{Done, Failed, Query};
use std::{io, net::SocketAddr, time::Duration};
use tokio::{net::UdpSocket, util::FutureExt};

const DEFAULT_RECV_TIMEOUT: Duration = Duration::from_secs(3); //3 secs
const DEFAULT_RECV_BUF_SIZE: usize = 1024;

#[derive(Clone)]
pub struct Forwarder {
    target: SocketAddr,
}

impl Forwarder {
    pub fn new(target: SocketAddr) -> Self {
        Forwarder { target: target }
    }

    pub fn handle_query(
        &self,
        query: Query,
    ) -> impl Future<Item = Done, Error = Failed> + Send + 'static {
        let mut render = MessageRender::new();
        query.message.rend(&mut render);
        let sender = query.client;
        let socket = UdpSocket::bind(&("0.0.0.0:0".parse::<SocketAddr>().unwrap())).unwrap();
        socket
            .send_dgram(render.take_data(), &self.target)
            .and_then(|(socket, _)| socket.recv_dgram(vec![0; DEFAULT_RECV_BUF_SIZE]))
            .and_then(move |(_, buf, size, _)| {
                if let Ok(resp) = Message::from_wire(&buf[..size]) {
                    future::ok(Done(Query::new(resp, sender)))
                } else {
                    future::err(io::Error::new(io::ErrorKind::Other, "invalid response"))
                }
            })
            .timeout(DEFAULT_RECV_TIMEOUT)
            .map_err(move |_| {
                println!("forward timedOut");
                Failed(query)
            })
    }
}
