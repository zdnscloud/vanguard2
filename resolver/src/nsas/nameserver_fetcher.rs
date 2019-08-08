use crate::{
    nsas::{
        entry_key::EntryKey,
        message_util::{message_to_nameserver_entry, message_to_zone_entry},
        nameserver_entry::{NameserverCache, NameserverEntry},
    },
    Resolver,
};
use failure::Result;
use futures::{future, prelude::*, Future};
use lru::LruCache;
use r53::{Message, Name, RRType};
use std::{
    io,
    net::Ipv4Addr,
    sync::{Arc, Mutex},
};
use tokio::executor::spawn;

#[derive(PartialEq, Eq, Debug)]
pub enum FetchStyle {
    FetchAll,
    FetchAny,
}

pub struct NameserverFetcher<R: Resolver + Clone + 'static> {
    style: FetchStyle,
    names: Vec<Name>,
    nameservers: Arc<Mutex<NameserverCache>>,
    resolver: R,
    fut: Option<Box<Future<Item = Message, Error = io::Error> + Send>>,
    current_name: Option<Name>,
}

impl<R: Resolver + Clone + 'static> NameserverFetcher<R> {
    pub fn new(
        style: FetchStyle,
        names: Vec<Name>,
        nameservers: Arc<Mutex<NameserverCache>>,
        resolver: R,
    ) -> Self {
        NameserverFetcher {
            style,
            names,
            nameservers,
            resolver,
            fut: None,
            current_name: None,
        }
    }
}

impl<R: Resolver + Clone + 'static> Future for NameserverFetcher<R> {
    type Item = Ipv4Addr;
    type Error = ();

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            if self.fut.is_none() {
                let name = self.names.pop();
                if name.is_none() {
                    return Err(());
                } else {
                    let name = name.unwrap();
                    self.fut = Some(self.resolver.resolve(name.clone(), RRType::A));
                    self.current_name = Some(name);
                }
            }

            match self.fut.as_mut().unwrap().poll() {
                Err(_) => {}
                Ok(Async::NotReady) => {
                    return Ok(Async::NotReady);
                }
                Ok(Async::Ready(msg)) => {
                    if let Ok(entry) =
                        message_to_nameserver_entry(self.current_name.take().unwrap(), msg)
                    {
                        let addr = entry.select_address().unwrap().get_v4_addr();
                        self.nameservers.lock().unwrap().put(entry.get_key(), entry);
                        if self.style == FetchStyle::FetchAny {
                            return Ok(Async::Ready(addr));
                        }
                    }
                }
            }
        }
    }
}
