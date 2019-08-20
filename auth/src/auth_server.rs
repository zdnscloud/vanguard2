use crate::zones::AuthZone;
use failure;
use futures::{future, Future};
use server::Query;
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
        &self,
        mut query: Query,
    ) -> impl Future<Item = Query, Error = failure::Error> + Send + 'static {
        let zones = self.zones.clone();
        future::lazy(move || {
            let zones = zones.read().unwrap();
            if zones.handle_query(&mut query.message) {
                query.done = true;
                future::ok(query)
            } else {
                query.done = false;
                future::ok(query)
            }
        })
    }
}
