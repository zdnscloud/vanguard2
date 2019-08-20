use crate::{
    nsas::{
        address_entry,
        entry_key::EntryKey,
        nameserver_cache::{self, Nameserver, NameserverCache},
        nameserver_fetcher::NameserverFetcher,
        zone_cache::ZoneCache,
        zone_fetcher::ZoneFetcher,
    },
    Recursor,
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
pub struct NSAddressStore {
    nameservers: Arc<Mutex<NameserverCache>>,
    zones: Arc<Mutex<ZoneCache>>,
    probing_name_servers: Arc<Mutex<HashSet<Name>>>,
    pub recursor: Option<Recursor>,
}

impl NSAddressStore {
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
            recursor: None,
        }
    }

    pub fn set_resolver(&mut self, recursor: Recursor) {
        self.recursor = Some(recursor);
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

        if !missing_nameserver.is_empty()
            && self.probing_name_servers.lock().unwrap().len() < MAX_PROBING_NAMESERVER_COUNT
        {
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
                println!(
                    "start to probe {:?}, waiting queue len is {}",
                    missing_nameserver,
                    self.probing_name_servers.lock().unwrap().len()
                );
                let probing_name_servers = self.probing_name_servers.clone();
                let done_nameserver = missing_nameserver.clone();
                let recursor = self.recursor.as_ref().unwrap().clone();
                spawn(Box::new(
                    NameserverFetcher::new(missing_nameserver, self.nameservers.clone(), recursor)
                        .map(move |_| {
                            let mut probing_name_servers = probing_name_servers.lock().unwrap();
                            done_nameserver.into_iter().for_each(|n| {
                                probing_name_servers.remove(&n);
                            });
                        }),
                ));
            }
        }
        Some(nameserver.unwrap())
    }

    pub fn fetch_zone(&self, zone: Name, depth: usize) -> ZoneFetcher {
        return ZoneFetcher::new(
            zone,
            self.recursor.as_ref().unwrap().clone(),
            self.nameservers.clone(),
            self.zones.clone(),
            depth,
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
