use std::{net::SocketAddr, sync::Arc};

use crate::handler::QueryHandler;
use crate::tcp_server::TcpServer;
use crate::udp_server::UdpServer;
use futures::{future, Future};
use tokio::executor::spawn;

pub struct Server<S: QueryHandler> {
    addr: SocketAddr,
    handler: S,
}

impl<S: QueryHandler + 'static> Server<S> {
    pub fn new(addr: SocketAddr, handler: S) -> Self {
        Server { addr, handler }
    }

    pub fn into_future(self) -> impl Future<Item = (), Error = ()> + Send + 'static {
        let handler = Arc::new(self.handler);
        let addr = self.addr;
        future::lazy(move || {
            spawn(UdpServer::new(addr, handler.clone()).map_err(|e| println!("udp errr {:?}", e)));
            TcpServer::new(addr, handler.clone()).into_future()
        })
    }
}
