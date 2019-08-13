use crate::{
    cache::MessageLruCache, error::RecursorError, nsas::NSAddressStore, running_query::RunningQuery,
};
use failure;
use futures::{future, Future};
use r53::{name, Message, Name, RRType};
use std::sync::{Arc, Mutex};

pub trait Resolver {
    fn resolve(
        &self,
        name: Name,
        typ: RRType,
    ) -> Box<Future<Item = Message, Error = failure::Error> + Send>;
}

#[derive(Clone)]
pub struct Recursor {
    cache: Arc<Mutex<MessageLruCache>>,
    nsas: Arc<NSAddressStore>,
}

impl Recursor {
    pub fn new() -> Self {
        Recursor {
            cache: Arc::new(Mutex::new(MessageLruCache::new(0))),
            nsas: Arc::new(NSAddressStore::new()),
        }
    }
}

impl Resolver for Recursor {
    fn resolve(
        &self,
        name: Name,
        typ: RRType,
    ) -> Box<Future<Item = Message, Error = failure::Error> + Send> {
        let query = Message::with_query(name.clone(), typ);
        Box::new(future::ok(query))
        /*
        let cache = self.cache.lock().unwrap();
        if cache.gen_response(&mut query) {
            return Box::new(future::ok(query));
        }

        let current_zone = if let Some(ns) = cache.get_deepest_ns(&name) {
            ns
        } else {
            name::root()
        };
        let query = RunningQuery::new(query, current_zone, self.clone());
        self.nsas
            .get_nameserver(current_zone, self.clone())
            .and_then(|nameserver| {})
            */
    }
}
