use auth::AuthServer;
use forwarder::Forwarder;
use futures::{future, Future};
use server::{Done, Failed, Query, QueryHandler};
use std::sync::{Arc, Mutex};

pub struct Resolver {
    auth: AuthServer,
    forwarder: Forwarder,
}

impl Resolver {
    pub fn new(auth: AuthServer, forwarder: Forwarder) -> Self {
        Resolver { auth, forwarder }
    }
}

impl QueryHandler for Resolver {
    fn handle_query(
        &self,
        query: Query,
    ) -> Box<dyn Future<Item = Done, Error = Failed> + Send + 'static> {
        let forwarder = self.forwarder.clone();
        Box::new(
            self.auth
                .handle_query(query)
                .or_else(move |query| forwarder.handle_query(query.0)),
        )
    }
}
