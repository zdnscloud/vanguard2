use crate::zones::AuthZone;
use futures::{future, Future};
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
        mut query: Query,
        mut sender: UdpStreamSender,
    ) -> Box<dyn Future<Item = (), Error = ()> + Send + 'static> {
        let zones = self.zones.clone();
        Box::new(future::lazy(move || {
            let zones = zones.read().unwrap();
            zones.handle_query(&mut query.message);
            if sender.send_response(query).is_err() {
                println!("send queue is full");
            }
            future::ok::<(), ()>(())
        }))
    }
}
