use crate::{
    cache::MessageCache, error::RecursorError, nsas::NSAddressStore, running_query::RunningQuery,
};
use failure;
use futures::{future, Future};
use r53::{name, Message, Name, RRType};
use server::Query;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::prelude::*;

pub trait Resolver: Clone + Send + 'static {
    type Query: Future<Item = Message, Error = failure::Error> + Send + 'static;
    fn new_query(&self, query: Message, depth: usize) -> Self::Query;
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
        recursor
    }

    pub fn handle_query(
        &self,
        query: Query,
    ) -> Box<Future<Item = Query, Error = failure::Error> + Send + 'static> {
        let client = query.client;
        Box::new(
            RunningQuery::new(query.message, self.clone(), 1).map(move |message| Query {
                client,
                message,
                done: true,
            }),
        )
    }
}

impl Resolver for Recursor {
    type Query = RunningQuery;
    fn new_query(&self, query: Message, depth: usize) -> RunningQuery {
        RunningQuery::new(query, self.clone(), depth)
    }
}
