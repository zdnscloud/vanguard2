use futures::Future;
use r53::{Message, MessageRender};
use server::{Done, Failed, Query};
use std::net::SocketAddr;
use std::time::Duration;
use tokio::{net::UdpSocket, util::FutureExt};

#[derive(Clone)]
pub struct Forwarder {
    target: SocketAddr,
}

impl Forwarder {
    pub fn new(target: SocketAddr) -> Self {
        Forwarder { target: target }
    }

    pub fn handle_query(
        &mut self,
        query: Query,
    ) -> impl Future<Item = Done, Error = Failed> + Send + 'static {
        let mut render = MessageRender::new();
        query.message.rend(&mut render);
        let sender = query.client;
        let socket = UdpSocket::bind(&("0.0.0.0:0".parse::<SocketAddr>().unwrap())).unwrap();
        socket
            .send_dgram(render.take_data(), &self.target)
            .and_then(|(socket, _)| socket.recv_dgram(vec![0; 1024]))
            .timeout(Duration::from_secs(3))
            .map_err(|_| {
                println!("forward timedOut");
                Failed(query)
            })
            .map(move |(_, buf, size, _)| {
                let resp = Message::from_wire(&buf[..size]).unwrap();
                Done(Query::new(resp, sender))
            })
    }
}
