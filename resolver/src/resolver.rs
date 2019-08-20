use crate::{
    cache::MessageCache, error::RecursorError, nsas::NSAddressStore, running_query::RunningQuery,
};
use failure;
use futures::{future, Future};
use r53::{name, Message, Name, RRType};
use server::{Done, Query};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::prelude::*;

#[derive(Clone)]
pub struct Recursor {
    pub(crate) cache: Arc<Mutex<MessageCache>>,
    pub(crate) nsas: Arc<NSAddressStore>,
}

impl Recursor {
    pub fn new(cache: MessageCache) -> Self {
        let mut nsas = Arc::new(NSAddressStore::new());
        let recursor = Recursor {
            cache: Arc::new(Mutex::new(cache)),
            nsas: Arc::clone(&nsas),
        };
        unsafe {
            let pointer = Arc::into_raw(nsas) as *mut NSAddressStore;
            (*pointer).set_resolver(recursor.clone());
        }
        assert!(recursor.nsas.recursor.is_some());
        recursor
    }

    pub fn handle_query(
        &self,
        query: Query,
    ) -> Box<Future<Item = Done, Error = failure::Error> + Send + 'static> {
        let client = query.client;
        Box::new(
            RunningQuery::new(query.message, self.clone(), 1)
                .map(move |message| Done(Query { client, message })),
        )
    }

    pub fn new_query(&self, query: Message, depth: usize) -> RunningQuery {
        RunningQuery::new(query, self.clone(), depth)
    }
}
