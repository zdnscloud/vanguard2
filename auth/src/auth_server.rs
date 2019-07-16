use crate::zones::AuthZone;
use r53::{Message, MessageRender};
use std::{
    io,
    net::SocketAddr,
    sync::{Arc, RwLock},
};
use tokio::{net::UdpSocket, prelude::*};

pub struct AuthServer {
    socket: UdpSocket,
    zones: Arc<RwLock<AuthZone>>,
    current_user: Option<SocketAddr>,
    render: MessageRender,
    buf: Vec<u8>,
}

impl AuthServer {
    pub fn new(socket: UdpSocket, zones: Arc<RwLock<AuthZone>>) -> Self {
        AuthServer {
            socket: socket,
            zones: zones,
            current_user: None,
            render: MessageRender::new(),
            buf: vec![0; 1024],
        }
    }
}

impl Future for AuthServer {
    type Item = ();
    type Error = std::io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        loop {
            if let Some(peer) = self.current_user {
                let _amt = try_ready!(self.socket.poll_send_to(self.render.data(), &peer));
                self.current_user = None;
            }

            let (_, client) = try_ready!(self.socket.poll_recv_from(&mut self.buf));
            let message = Message::from_wire(self.buf.as_slice());
            if message.is_err() {
                continue;
            }
            let message = message.unwrap();
            let resp = {
                let zones = self.zones.read().unwrap();
                zones.handle_query(message)
            };
            self.render.clear();
            resp.rend(&mut self.render);
            self.current_user = Some(client);
        }
    }
}
