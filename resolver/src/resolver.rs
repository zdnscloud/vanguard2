use crate::{
    cache::MessageCache, error::RecursorError, nsas::NSAddressStore, running_query::RunningQuery,
};
use failure;
use futures::{future, Future};
use r53::{name, Message, Name, RRType};
use server::{Done, Query};
use std::sync::{Arc, Mutex};

pub trait Resolver {
    fn resolve(&self, query: Message)
        -> Box<Future<Item = Message, Error = failure::Error> + Send>;
}

#[derive(Clone)]
pub struct Recursor {
    pub(crate) cache: Arc<Mutex<MessageCache>>,
    pub(crate) nsas: Arc<NSAddressStore<Recursor>>,
}

impl Recursor {
    pub fn new(cache: MessageCache) -> Self {
        let mut nsas = Arc::new(NSAddressStore::new());
        let recursor = Recursor {
            cache: Arc::new(Mutex::new(cache)),
            nsas: Arc::clone(&nsas),
        };
        unsafe {
            let pointer = Arc::into_raw(nsas) as *mut NSAddressStore<Recursor>;
            (*pointer).set_resolver(recursor.clone());
        }
        assert!(recursor.nsas.resolver.is_some());
        recursor
    }

    pub fn handle_query(
        &self,
        query: Query,
    ) -> Box<Future<Item = Done, Error = failure::Error> + Send + 'static> {
        let client = query.client;
        Box::new(
            RunningQuery::new(query.message, self.clone())
                .map(move |message| Done(Query { client, message })),
        )
    }
}

impl Resolver for Recursor {
    fn resolve(
        &self,
        query: Message,
    ) -> Box<Future<Item = Message, Error = failure::Error> + Send + 'static> {
        Box::new(RunningQuery::new(query, self.clone()))
    }
}
