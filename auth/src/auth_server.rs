use crate::zones::AuthZone;
use futures::{future, Future};
use server::{Done, Failed, Query};
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

    pub fn handle_query(
        &mut self,
        mut query: Query,
    ) -> impl Future<Item = Done, Error = Failed> + Send + 'static {
        let zones = self.zones.clone();
        future::lazy(move || {
            let zones = zones.read().unwrap();
            if zones.handle_query(&mut query.message) {
                future::ok(Done(query))
            } else {
                future::err(Failed(query))
            }
        })
    }
}
