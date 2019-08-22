use crate::server::Query;
use failure;
use futures::{prelude::*, Future};
use r53::Message;
use std::net::SocketAddr;

pub struct MessageFutureAdaptor<T> {
    client: SocketAddr,
    inner: T,
}

impl<T> MessageFutureAdaptor<T> {
    pub fn new(client: SocketAddr, inner: T) -> Self {
        MessageFutureAdaptor { client, inner }
    }
}

impl<T: Future<Item = Message, Error = failure::Error>> Future for MessageFutureAdaptor<T> {
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
