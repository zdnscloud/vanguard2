use super::{
    cache::MessageCache, nsas::NSAddressStore, roothint::RootHint, running_query::RunningQuery,
};
use crate::{config::RecursorConfig, error::VgError, network::MessageFutureAdaptor, server::Query};
use failure;
use futures::Future;
use r53::{name, Message, Name, RRType};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::prelude::*;

const DEFAULT_MESSAGE_CACHE_SIZE: usize = 10000;

pub trait Resolver: Clone + Send + 'static {
    type Query: Future<Item = Message, Error = failure::Error> + Send + 'static;
    fn new_query(&self, query: Message, depth: usize) -> Self::Query;
}

#[derive(Clone)]
pub struct Recursor {
    pub(crate) cache: Arc<Mutex<MessageCache>>,
    pub(crate) nsas: Arc<NSAddressStore<Recursor>>,
    pub(crate) roothint: Arc<RootHint>,
}

pub type RecursorFuture = MessageFutureAdaptor<RunningQuery>;

impl Recursor {
    pub fn new(conf: &RecursorConfig) -> Self {
        let nsas = Arc::new(NSAddressStore::new());
        let mut recursor = Recursor {
            cache: Arc::new(Mutex::new(MessageCache::new(DEFAULT_MESSAGE_CACHE_SIZE))),
            nsas: Arc::clone(&nsas),
            roothint: Arc::new(RootHint::new()),
        };

        unsafe {
            let pointer = Arc::into_raw(nsas) as *mut NSAddressStore<Recursor>;
            (*pointer).set_resolver(recursor.clone());
        }
        recursor
    }

    pub fn handle_query(&self, query: Query) -> RecursorFuture {
        let client = query.client;
        let fut = RunningQuery::new(query.message, self.clone(), 1);
        MessageFutureAdaptor::new(client, fut)
    }
}

impl Resolver for Recursor {
    type Query = RunningQuery;
    fn new_query(&self, query: Message, depth: usize) -> RunningQuery {
        RunningQuery::new(query, self.clone(), depth)
    }
}
