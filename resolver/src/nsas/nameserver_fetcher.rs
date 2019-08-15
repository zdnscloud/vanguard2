use crate::{
    nsas::{
        entry_key::EntryKey,
        error::NSASError,
        message_util::{message_to_nameserver_entry, message_to_zone_entry},
        nameserver_entry::{Nameserver, NameserverCache, NameserverEntry},
    },
    Resolver,
};
use failure;
use futures::{future, prelude::*, Future};
use r53::{Message, Name, RRType};
use std::{
    io,
    sync::{Arc, Mutex},
};

pub struct NameserverFetcher<R> {
    names: Vec<Name>,
    nameservers: Arc<Mutex<NameserverCache>>,
    resolver: R,
    fut: Option<Box<Future<Item = Message, Error = failure::Error> + Send>>,
    current_name: Option<Name>,
    get_any: bool,
}

impl<R: Resolver> NameserverFetcher<R> {
    pub fn new(names: Vec<Name>, nameservers: Arc<Mutex<NameserverCache>>, resolver: R) -> Self {
        NameserverFetcher {
            names,
            nameservers,
            resolver,
            fut: None,
            current_name: None,
            get_any: false,
        }
    }
}

impl<R: Resolver> Future for NameserverFetcher<R> {
    type Item = ();
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            if self.fut.is_none() {
                let name = self.names.pop();
                if name.is_none() {
                    if self.get_any {
                        return Ok(Async::Ready(()));
                    } else {
                        return Err(NSASError::NoValidNameserver.into());
                    }
                } else {
                    let name = name.unwrap();
                    self.fut = Some(
                        self.resolver
                            .resolve(Message::with_query(name.clone(), RRType::A)),
                    );
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
                        self.nameservers.lock().unwrap().put(entry.get_key(), entry);
                        self.get_any = true;
                    }
                }
            }
            self.fut = None;
        }
    }
}

mod test {
    use super::*;
    use crate::nsas::test_helper::DumbResolver;
    use lru::LruCache;
    use r53::{util::hex::from_hex, RData, RRset};
    use std::net::Ipv4Addr;
    use tokio::runtime::Runtime;

    #[test]
    fn test_fetch_all() {
        let mut resolver = DumbResolver::new();
        let names = vec![
            Name::new("ns1.knet.cn").unwrap(),
            Name::new("ns2.knet.cn").unwrap(),
            Name::new("ns3.knet.cn").unwrap(),
        ];

        for name in names.iter() {
            resolver.set_answer(
                name.clone(),
                RRType::A,
                vec![RData::from_str(RRType::A, "1.1.1.1").unwrap()],
                Vec::new(),
            );
        }

        let nameservers = Arc::new(Mutex::new(LruCache::new(100)));

        let mut fetcher = NameserverFetcher::new(names, nameservers.clone(), resolver);
        assert_eq!(nameservers.lock().unwrap().len(), 0);

        let mut rt = Runtime::new().unwrap();
        rt.block_on(fetcher).unwrap();

        assert_eq!(nameservers.lock().unwrap().len(), 3);
    }
}
