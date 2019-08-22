use super::{
    cache::MessageCache, error::RecursorError, nsas::NSAddressStore, roothint::RootHint,
    running_query::RunningQuery,
};
use crate::server::Query;
use failure;
use futures::{future, Future};
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

pub type RecursorFuture = RunningQueryAdaptor<RunningQuery>;

impl Recursor {
    pub fn new() -> Self {
        let nsas = Arc::new(NSAddressStore::new());
        let recursor = Recursor {
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
        RunningQueryAdaptor::new(client, fut)
    }
}

impl Resolver for Recursor {
    type Query = RunningQuery;
    fn new_query(&self, query: Message, depth: usize) -> RunningQuery {
        RunningQuery::new(query, self.clone(), depth)
    }
}

pub struct RunningQueryAdaptor<T> {
    client: SocketAddr,
    inner: T,
}

impl<T> RunningQueryAdaptor<T> {
    pub fn new(client: SocketAddr, inner: T) -> Self {
        RunningQueryAdaptor { client, inner }
    }
}

impl<T: Future<Item = Message, Error = failure::Error>> Future for RunningQueryAdaptor<T> {
    type Item = Query;
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        match self.inner.poll() {
            Err(e) => {
                return Err(e);
            }
            Ok(Async::NotReady) => {
                return Ok(Async::NotReady);
            }
            Ok(Async::Ready(resp)) => {
                return Ok(Async::Ready(Query {
                    client: self.client,
                    message: resp,
                    done: true,
                }));
            }
        }
    }
}
