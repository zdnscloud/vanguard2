use std::{net::SocketAddr, time::Duration};

pub trait AbstractNameserver {
    fn get_addr(&self) -> SocketAddr;
    fn set_rtt(&mut self, rtt: Duration);
    fn set_unreachable(&mut self);
}

pub trait NameserverStore<S: AbstractNameserver> {
    fn update_nameserver_rtt(&self, nameserver: &S);
}
