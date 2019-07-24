use auth::AuthServer;
use cache::{MessageCache, MessageLruCache};
use forwarder::Forwarder;
use futures::{future, Future};
use server::{Done, Failed, Query, QueryHandler};
use std::sync::{Arc, Mutex};

pub struct Resolver {
    auth: AuthServer,
    forwarder: Forwarder,
    message_cache: Arc<Mutex<MessageLruCache>>,
}

impl Resolver {
    pub fn new(auth: AuthServer, forwarder: Forwarder) -> Self {
        Resolver {
            auth,
            forwarder,
            message_cache: Arc::new(Mutex::new(MessageLruCache::new(0))),
        }
    }
}

impl QueryHandler for Resolver {
    fn handle_query(
        &mut self,
        query: Query,
    ) -> Box<dyn Future<Item = Done, Error = Failed> + Send + 'static> {
        let read_cache = self.message_cache.clone();
        let write_cache = self.message_cache.clone();
        let mut forwarder = self.forwarder.clone();
        Box::new(
            self.auth
                .handle_query(query)
                .or_else(move |mut query| {
                    future::lazy(move || {
                        let mut read_cache = read_cache.lock().unwrap();
                        if read_cache.gen_response(&mut query.0.message) {
                            future::ok(Done(query.0))
                        } else {
                            future::err(Failed(query.0))
                        }
                    })
                })
                .or_else(move |query| forwarder.handle_query(query.0))
                .map(move |response| {
                    let mut write_cache = write_cache.lock().unwrap();
                    write_cache.add_message(response.0.message.clone());
                    response
                }),
        )
    }
}
