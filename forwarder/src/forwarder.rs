use futures::Future;
use r53::{Message, MessageRender};
use server::{Query, QueryService, ResponseSender, UdpStreamSender};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::{net::UdpSocket, util::FutureExt};

pub struct Forwarder {
    target: SocketAddr,
}

impl Forwarder {
    pub fn new(target: SocketAddr) -> Self {
        Forwarder { target: target }
    }
}

impl QueryService for Forwarder {
    type ResponseSender = UdpStreamSender;
    fn is_capable(&self, _query: &Query) -> bool {
        true
    }

    fn handle_query(
        &mut self,
        mut query: Query,
        mut sender: UdpStreamSender,
    ) -> Box<dyn Future<Item = (), Error = ()> + Send + 'static> {
        let mut render = MessageRender::new();
        query.message.rend(&mut render);
        let socket = UdpSocket::bind(&("0.0.0.0:0".parse::<SocketAddr>().unwrap())).unwrap();
        Box::new(
            socket
                .send_dgram(render.take_data(), &self.target)
                .and_then(|(socket, _)| socket.recv_dgram(vec![0; 1024]))
                .timeout(Duration::from_secs(3))
                .map_err(|_| println!("forward timedOut"))
                .map(move |(_, buf, size, _)| {
                    let resp = Message::from_wire(&buf[..size]).unwrap();
                    query.message = resp;
                    if sender.send_response(query).is_err() {
                        println!("send queue is full");
                    }
                }),
        )
    }
}
