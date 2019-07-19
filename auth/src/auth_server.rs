use crate::zones::AuthZone;
use futures::{Async, Future, Poll};
use server::{Query, QueryService, ResponseSender, UdpStreamSender};
use std::sync::{Arc, RwLock};

pub struct AuthServer {
    zones: Arc<RwLock<AuthZone>>,
}

impl AuthServer {
    pub fn new() -> Self {
        AuthServer {
            zones: Arc::new(RwLock::new(AuthZone::new())),
        }
    }

    pub fn zones(&self) -> Arc<RwLock<AuthZone>> {
        self.zones.clone()
    }
}

impl QueryService for AuthServer {
    type ResponseSender = UdpStreamSender;
    fn is_capable(&self, query: &Query) -> bool {
        let zones = self.zones.read().unwrap();
        zones.get_zone(&query.message.question.name).is_some()
    }

    fn handle_query(
        &mut self,
        query: Query,
        sender: UdpStreamSender,
    ) -> Box<dyn Future<Item = (), Error = ()> + Send + 'static> {
        Box::new(LookupFuture {
            zones: self.zones.clone(),
            query: Some(query),
            sender: sender,
        })
    }
}

pub struct LookupFuture {
    zones: Arc<RwLock<AuthZone>>,
    query: Option<Query>,
    sender: UdpStreamSender,
}

impl Future for LookupFuture {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<(), ()> {
        let zones = self.zones.read().unwrap();
        let mut resp = self.query.take().unwrap();
        zones.handle_query(&mut resp.message);
        if self.sender.send_response(resp).is_err() {
            println!("send queue is full");
        }
        Ok(Async::Ready(()))
    }
}
