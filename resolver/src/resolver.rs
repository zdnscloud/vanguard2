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
    pub(crate) nsas: Arc<NSAddressStore>,
}

impl Recursor {
    pub fn new(cache: MessageCache) -> Self {
        Recursor {
            cache: Arc::new(Mutex::new(cache)),
            nsas: Arc::new(NSAddressStore::new()),
        }
    }

    pub fn handle_query(
        &self,
        query: Query,
    ) -> Box<Future<Item = Done, Error = failure::Error> + Send + 'static> {
        let client = query.client;
        Box::new(
            self.resolve(query.message)
                .map(move |message| Done(Query { client, message })),
        )
    }
}

impl Resolver for Recursor {
    fn resolve(
        &self,
        query: Message,
    ) -> Box<Future<Item = Message, Error = failure::Error> + Send + 'static> {
        RunningQuery::new(query, self.clone()).resolve()
    }
}
