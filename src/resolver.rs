use auth::AuthServer;
use cache::MessageCache;
use forwarder::Forwarder;
use futures::{future, Future};
use server::{Done, Failed, Query, QueryHandler};
use std::sync::{Arc, Mutex};

pub struct Resolver<T: MessageCache> {
    auth: AuthServer,
    forwarder: Forwarder,
    message_cache: Arc<Mutex<T>>,
}

impl<T: MessageCache> Resolver<T> {
    pub fn new(auth: AuthServer, forwarder: Forwarder, cache: T) -> Self {
        Resolver {
            auth,
            forwarder,
            message_cache: Arc::new(Mutex::new(cache)),
        }
    }
}

impl<T: MessageCache + Send + 'static> QueryHandler for Resolver<T> {
    fn handle_query(
        &self,
        query: Query,
    ) -> Box<dyn Future<Item = Done, Error = Failed> + Send + 'static> {
        let read_cache = self.message_cache.clone();
        let write_cache = self.message_cache.clone();
        let forwarder = self.forwarder.clone();
        Box::new(
            self.auth
                .handle_query(query)
                .or_else(move |mut query| {
                    let mut read_cache = read_cache.lock().unwrap();
                    if read_cache.gen_response(&mut query.0.message) {
                        future::ok(Done(query.0))
                    } else {
                        future::err(Failed(query.0))
                    }
                })
                .or_else(move |query| {
                    forwarder.handle_query(query.0).map(move |response| {
                        let mut write_cache = write_cache.lock().unwrap();
                        write_cache.add_message(response.0.message.clone());
                        response
                    })
                }),
        )
    }
}
