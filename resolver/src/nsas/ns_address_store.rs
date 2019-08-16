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
use std::sync::{Arc, Mutex};
use tokio::executor::spawn;

const DEFAULT_ZONE_ENTRY_CACHE_SIZE: usize = 1009;
const DEFAULT_NAMESERVER_ENTRY_CACHE_SIZE: usize = 3001;

#[derive(Clone)]
pub struct NSAddressStore {
    nameservers: Arc<Mutex<NameserverCache>>,
    zones: Arc<Mutex<ZoneCache>>,
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
        }
    }

    pub fn get_nameserver(&self, zone: &Name) -> Option<(Nameserver, Vec<Name>)> {
        let (nameserver, missing_nameserver) = {
            let key = &EntryKey::from_name(zone);
            let mut zones = self.zones.lock().unwrap();
            if let Some(entry) = zones.get_zone(key) {
                entry.select_nameserver(&mut self.nameservers.lock().unwrap())
            } else {
                (None, Vec::new())
            }
        };

        if nameserver.is_some() {
            Some((nameserver.unwrap(), missing_nameserver))
        } else {
            None
        }
    }

    pub fn fetch_nameservers<R: Resolver + Clone + 'static + Send>(
        &self,
        nameservers: Vec<Name>,
        resolver: R,
    ) -> NameserverFetcher<R> {
        NameserverFetcher::new(nameservers, self.nameservers.clone(), resolver)
    }

    pub fn fetch_zone<R: Resolver + Clone + 'static + Send>(
        &self,
        zone: Name,
        resolver: R,
    ) -> ZoneFetcher<R> {
        return ZoneFetcher::new(zone, resolver, self.nameservers.clone(), self.zones.clone());
    }

    pub fn update_nameserver_rtt(&self, nameserver: &Nameserver) {
        let mut nameservers = self.nameservers.lock().unwrap();
        let key = &EntryKey::from_name(&nameserver.name);
        if let Some(entry) = nameservers.get_nameserver_mut(key) {
            entry.update_nameserver(nameserver);
        }
    }
}
