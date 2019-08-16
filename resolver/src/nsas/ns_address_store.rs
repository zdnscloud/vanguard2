use crate::{
    nsas::{
        address_entry,
        entry_key::EntryKey,
        nameserver_cache::{self, Nameserver, NameserverCache},
        nameserver_fetcher::NameserverFetcher,
        zone_cache::ZoneCache,
        zone_fetcher::ZoneFetcher,
    },
    Resolver,
};
use failure;
use futures::{future, prelude::*, Future};
use lru::LruCache;
use r53::Name;
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
use tokio::executor::spawn;

const DEFAULT_ZONE_ENTRY_CACHE_SIZE: usize = 1009;
const DEFAULT_NAMESERVER_ENTRY_CACHE_SIZE: usize = 3001;
const MAX_PROBING_NAMESERVER_COUNT: usize = 1000;

#[derive(Clone)]
pub struct NSAddressStore<R> {
    nameservers: Arc<Mutex<NameserverCache>>,
    zones: Arc<Mutex<ZoneCache>>,
    probing_name_servers: Arc<Mutex<HashSet<Name>>>,
    pub resolver: Option<R>,
}

impl<R: Resolver + Clone + Send + 'static> NSAddressStore<R> {
    pub fn new() -> Self {
        NSAddressStore {
            nameservers: Arc::new(Mutex::new(NameserverCache(LruCache::new(
                DEFAULT_NAMESERVER_ENTRY_CACHE_SIZE,
            )))),
            zones: Arc::new(Mutex::new(ZoneCache(LruCache::new(
                DEFAULT_ZONE_ENTRY_CACHE_SIZE,
            )))),
            probing_name_servers: Arc::new(Mutex::new(HashSet::with_capacity(
                MAX_PROBING_NAMESERVER_COUNT,
            ))),
            resolver: None,
        }
    }

    pub fn set_resolver(&mut self, resolver: R) {
        self.resolver = Some(resolver);
    }

    //this must be invoked in a future
    pub fn get_nameserver(&self, zone: &Name) -> Option<(Nameserver)> {
        let (nameserver, missing_nameserver) = {
            let key = &EntryKey::from_name(zone);
            let mut zones = self.zones.lock().unwrap();
            if let Some(entry) = zones.get_zone(key) {
                entry.select_nameserver(&mut self.nameservers.lock().unwrap())
            } else {
                (None, Vec::new())
            }
        };

        if nameserver.is_none() {
            return None;
        }

        if !missing_nameserver.is_empty() {
            let missing_nameserver = {
                let mut unprobe_nameserver = Vec::with_capacity(missing_nameserver.len());
                let mut probing_name_servers = self.probing_name_servers.lock().unwrap();
                missing_nameserver
                    .into_iter()
                    .fold(unprobe_nameserver, |mut servers, n| {
                        if probing_name_servers.insert(n.clone()) {
                            servers.push(n);
                        }
                        servers
                    })
            };
            if !missing_nameserver.is_empty() {
                println!("start to probe {:?}", missing_nameserver);
                let probing_name_servers = self.probing_name_servers.clone();
                let done_nameserver = missing_nameserver.clone();
                let resolver = self.resolver.as_ref().unwrap().clone();
                spawn(
                    NameserverFetcher::new(missing_nameserver, self.nameservers.clone(), resolver)
                        .map(move |_| {
                            let mut probing_name_servers = probing_name_servers.lock().unwrap();
                            done_nameserver.into_iter().for_each(|n| {
                                probing_name_servers.remove(&n);
                            });
                        }),
                );
            }
        }
        Some(nameserver.unwrap())
    }

    pub fn fetch_nameservers(&self, nameservers: Vec<Name>) -> NameserverFetcher<R> {
        NameserverFetcher::new(
            nameservers,
            self.nameservers.clone(),
            self.resolver.as_ref().unwrap().clone(),
        )
    }

    pub fn fetch_zone(&self, zone: Name) -> ZoneFetcher<R> {
        return ZoneFetcher::new(
            zone,
            self.resolver.as_ref().unwrap().clone(),
            self.nameservers.clone(),
            self.zones.clone(),
        );
    }

    pub fn update_nameserver_rtt(&self, nameserver: &Nameserver) {
        let mut nameservers = self.nameservers.lock().unwrap();
        let key = &EntryKey::from_name(&nameserver.name);
        if let Some(entry) = nameservers.get_nameserver_mut(key) {
            entry.update_nameserver(nameserver);
        }
    }
}
