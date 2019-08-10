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

    //#[test]
    fn test_fetch_all() {
        let mut resolver = DumbResolver::new(0);
        resolver.set_answer(vec![RData::from_str(RRType::A, "1.1.1.1").unwrap()]);

        let nameservers = Arc::new(Mutex::new(LruCache::new(100)));

        let mut fetcher = NameserverFetcher::new(
            FetchStyle::FetchAll,
            vec![
                Name::new("ns1.knet.cn").unwrap(),
                Name::new("ns2.knet.cn").unwrap(),
                Name::new("ns3.knet.cn").unwrap(),
            ],
            nameservers.clone(),
            resolver,
        );
        assert_eq!(nameservers.lock().unwrap().len(), 0);

        let mut rt = Runtime::new().unwrap();
        rt.block_on(fetcher.map(|_| ()));

        assert_eq!(nameservers.lock().unwrap().len(), 3);
    }

    //#[test]
    fn test_fetch_any() {
        let mut resolver = DumbResolver::new(0);
        resolver.set_answer(vec![RData::from_str(RRType::A, "1.1.1.1").unwrap()]);

        let nameservers = Arc::new(Mutex::new(LruCache::new(100)));

        let mut fetcher = NameserverFetcher::new(
            FetchStyle::FetchAny,
            vec![
                Name::new("ns1.knet.cn").unwrap(),
                Name::new("ns2.knet.cn").unwrap(),
                Name::new("ns3.knet.cn").unwrap(),
            ],
            nameservers.clone(),
            resolver,
        );
        assert_eq!(nameservers.lock().unwrap().len(), 0);

        let mut rt = Runtime::new().unwrap();
        rt.block_on(fetcher.map(|_| ()));

        assert_eq!(nameservers.lock().unwrap().len(), 1);
    }

    #[test]
    fn test_fetch_any_get_err() {
        let mut resolver = DumbResolver::new(1);
        resolver.set_answer(vec![RData::from_str(RRType::A, "1.1.1.1").unwrap()]);

        let nameservers = Arc::new(Mutex::new(LruCache::new(100)));

        let mut fetcher = NameserverFetcher::new(
            FetchStyle::FetchAny,
            vec![
                Name::new("ns1.knet.cn").unwrap(),
                Name::new("ns2.knet.cn").unwrap(),
                Name::new("ns3.knet.cn").unwrap(),
            ],
            nameservers.clone(),
            resolver,
        );
        assert_eq!(nameservers.lock().unwrap().len(), 0);

        let mut rt = Runtime::new().unwrap();
        rt.block_on(fetcher.map(|_| ()));

        let mut nameservers = nameservers.lock().unwrap();
        assert_eq!(nameservers.len(), 1);

        let nameserver = Name::new("ns2.knet.cn").unwrap();
        let key = EntryKey::from_name(&nameserver);
        let entry = nameservers.get(&key);
        assert!(entry.is_some());

        let entry = entry.unwrap();
        let addresses = entry.get_addresses();
        assert_eq!(addresses.len(), 1);
        assert_eq!(addresses[0].get_v4_addr(), Ipv4Addr::new(1, 1, 1, 1));
    }

}
