use crate::handler::{Query, QueryService, ResponseSender};
use futures::{
    stream::{Fuse, Peekable, Stream},
    sync::mpsc::{channel, Receiver, Sender},
    Async, Future, Poll,
};
use r53::{Message, MessageRender};
use std::io;
use tokio::{executor::spawn, net::UdpSocket};

pub struct UdpStream {
    socket: UdpSocket,
    sender: Sender<Query>,
    services: Vec<Box<dyn QueryService<ResponseSender = UdpStreamSender>>>,
    response_ch: Peekable<Fuse<Receiver<Query>>>,
}

impl UdpStream {
    pub fn new(
        socket: UdpSocket,
        services: Vec<Box<dyn QueryService<ResponseSender = UdpStreamSender>>>,
    ) -> Self {
        let (sender, response_ch) = channel(1024);
        UdpStream {
            socket,
            sender,
            services,
            response_ch: response_ch.fuse().peekable(),
        }
    }

    fn send_all_response(&mut self, render: &mut MessageRender) -> Poll<(), io::Error> {
        loop {
            match self.response_ch.peek() {
                Ok(Async::Ready(Some(query))) => {
                    query.message.rend(render);
                    try_ready!(self.socket.poll_send_to(render.data(), &query.client));
                    render.clear();
                }
                Ok(Async::Ready(None)) | Ok(Async::NotReady) => return Ok(Async::Ready(())),
                Err(_) => panic!("get error form channel"),
            }

            match self.response_ch.poll() {
                Err(_) => panic!("get error when poll response"),
                Ok(Async::NotReady) => return Ok(Async::Ready(())),
                _ => (),
            }
        }
    }
}

impl Future for UdpStream {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), Self::Error> {
        let mut buf = [0u8; 1024];
        let mut render = MessageRender::new();
        loop {
            try_ready!(self.send_all_response(&mut render));
            let (size, src) = try_ready!(self.socket.poll_recv_from(&mut buf));
            let query = Query::new(Message::from_wire(&buf[..size]).unwrap(), src);
            for s in &mut self.services {
                if s.is_capable(&query) {
                    let sender = UdpStreamSender::new(self.sender.clone());
                    spawn(s.handle_query(query, sender));
                    break;
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct UdpStreamSender(Sender<Query>);

impl UdpStreamSender {
    fn new(sender: Sender<Query>) -> Self {
        UdpStreamSender(sender)
    }
}

impl ResponseSender for UdpStreamSender {
    fn send_response(&mut self, response: Query) -> io::Result<()> {
        self.0
            .try_send(response)
            .map_err(|_| io::Error::new(io::ErrorKind::Other, "full"))
    }
}
