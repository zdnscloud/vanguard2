use super::{
    forwarder::Forwarder,
    group::{ForwarderGroup, ForwarderPool},
};
use crate::{
    config::ForwarderConfig,
    network::{MessageFutureAdaptor, Nameserver, NameserverStore, Sender},
    server::Query,
};
use datasrc::RBTree;
use futures::{prelude::*, Future};
use r53::Name;
use std::{mem, net::SocketAddr, sync::Arc};

#[derive(Clone)]
pub struct ForwarderManager {
    forwarders: Arc<RBTree<ForwarderGroup>>,
    pool: Arc<ForwarderPool>,
}

impl ForwarderManager {
    pub fn new(conf: &ForwarderConfig) -> Self {
        let pool = ForwarderPool::new(conf);
        let mut groups = RBTree::new();
        pool.init_groups(&mut groups, conf);
        ForwarderManager {
            forwarders: Arc::new(groups),
            pool: Arc::new(pool),
        }
    }

    pub fn handle_query(&self, query: Query) -> ForwarderFuture {
        let question = query.message.question.as_ref().unwrap();
        if let Some(forwarder) = self.get_forwarder(&question.name) {
            return ForwarderFuture::new(query, Some(forwarder), self);
        } else {
            return ForwarderFuture::new(query, None, self);
        }
    }

    fn get_forwarder(&self, name: &Name) -> Option<Forwarder> {
        let result = self.forwarders.find(name);
        if let Some(selecotr) = result.get_value() {
            return Some(selecotr.select_forwarder(&*self.pool));
        } else {
            return None;
        }
    }
}

enum State {
    NoForwarder(Query),
    Send((SocketAddr, Sender<Forwarder, ForwarderPool>)),
    Poisoned,
}

pub struct ForwarderFuture {
    state: State,
}

impl ForwarderFuture {
    pub fn new(query: Query, forwarder: Option<Forwarder>, manager: &ForwarderManager) -> Self {
        let state = if let Some(forwarder) = forwarder {
            State::Send((
                query.client,
                Sender::new(query.message, forwarder, manager.pool.clone()),
            ))
        } else {
            State::NoForwarder(query)
        };
        ForwarderFuture { state }
    }
}

impl Future for ForwarderFuture {
    type Item = Query;
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match mem::replace(&mut self.state, State::Poisoned) {
                State::NoForwarder(query) => {
                    return Ok(Async::Ready(query));
                }
                State::Send((client, mut fut)) => match fut.poll() {
                    Err(e) => {
                        return Err(e);
                    }
                    Ok(Async::NotReady) => {
                        self.state = State::Send((client, fut));
                        return Ok(Async::NotReady);
                    }
                    Ok(Async::Ready(message)) => {
                        return Ok(Async::Ready(Query {
                            client,
                            message,
                            done: true,
                        }));
                    }
                },
                State::Poisoned => {
                    panic!("forwarder future crashed");
                }
            }
        }
    }
}
