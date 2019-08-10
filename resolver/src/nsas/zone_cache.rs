use crate::{
    message_classifier::{classify_response, ResponseCategory},
    nsas::{
        entry_key::EntryKey,
        error::NSASError,
        message_util::{message_to_nameserver_entry, message_to_zone_entry},
        nameserver_entry::{NameserverCache, NameserverEntry},
        nameserver_fetcher::{FetchStyle, NameserverFetcher},
        zone_entry::ZoneEntry,
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

const DEFAULT_ZONE_ENTRY_CACHE_SIZE: usize = 1009;
const DEFAULT_NAMESERVER_ENTRY_CACHE_SIZE: usize = 3001;

type ZoneLruCache = LruCache<EntryKey, ZoneEntry>;

pub struct ZoneCache<R: Resolver + Clone + 'static> {
    nameservers: Arc<Mutex<NameserverCache>>,
    zones: Arc<Mutex<ZoneLruCache>>,
    resolver: R,
}

impl<R: Resolver + Clone + 'static + Send> ZoneCache<R> {
    pub fn new(resolver: R) -> Self {
        ZoneCache {
            nameservers: Arc::new(Mutex::new(LruCache::new(
                DEFAULT_NAMESERVER_ENTRY_CACHE_SIZE,
            ))),
            zones: Arc::new(Mutex::new(LruCache::new(DEFAULT_ZONE_ENTRY_CACHE_SIZE))),
            resolver,
        }
    }

    pub fn get_nameserver(&mut self, zone: &Name) -> Box<Future<Item = Ipv4Addr, Error = ()>> {
        let (address, missing_nameserver) = {
            let key = &EntryKey::from_name(zone);
            let mut zones = self.zones.lock().unwrap();
            if let Some(entry) = zones.get(key) {
                entry.select_nameserver(&mut self.nameservers.lock().unwrap())
            } else {
                (None, Vec::new())
            }
        };

        if address.is_some() {
            if !missing_nameserver.is_empty() {
                spawn(
                    NameserverFetcher::new(
                        FetchStyle::FetchAll,
                        missing_nameserver,
                        self.nameservers.clone(),
                        self.resolver.clone(),
                    )
                    .map(|_| ()),
                );
            }
            return Box::new(future::ok(address.unwrap().get_v4_addr()));
        } else {
            return self.fetch_zone(zone);
        }
    }

    fn fetch_zone(&self, zone: &Name) -> Box<Future<Item = Ipv4Addr, Error = ()>> {
        let resolver = self.resolver.clone();
        let zone_name = zone.clone();
        let zones = self.zones.clone();
        let nameservers = self.nameservers.clone();
        let nameservers2 = self.nameservers.clone();
        Box::new(
            self.resolver
                .resolve(zone.clone(), RRType::NS)
                .map_err(|e| {
                    println!("get io err {:?}", e);
                    None
                })
                .and_then(move |mut msg| {
                    if let Ok((zone_entry, nameserver_entries)) =
                        message_to_zone_entry(&zone_name, msg)
                    {
                        if let Some(nameserver_entries) = nameserver_entries {
                            let mut nameservers = nameservers.lock().unwrap();
                            for nameserver_entry in nameserver_entries {
                                nameservers.put(nameserver_entry.get_key(), nameserver_entry);
                            }
                            let (address, _) = zone_entry.select_nameserver(&mut nameservers);
                            let mut zones = zones.lock().unwrap();
                            zones.put(zone_entry.get_key(), zone_entry);
                            return future::ok(address.unwrap().get_v4_addr());
                        }
                        return future::err(Some(zone_entry.get_nameservers()));
                    }
                    return future::err(None);
                })
                .or_else(move |left_servers| {
                    let nameservers = if let Some(servers) = left_servers {
                        servers
                    } else {
                        Vec::new()
                    };
                    return NameserverFetcher::new(
                        FetchStyle::FetchAny,
                        nameservers,
                        nameservers2,
                        resolver,
                    );
                }),
        )
    }
}
