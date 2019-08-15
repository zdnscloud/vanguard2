use crate::{
    nsas::{
        address_entry,
        entry_key::EntryKey,
        nameserver_entry::{self, Nameserver, NameserverCache},
        nameserver_fetcher::NameserverFetcher,
        zone_entry::ZoneCache,
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
            nameservers: Arc::new(Mutex::new(LruCache::new(
                DEFAULT_NAMESERVER_ENTRY_CACHE_SIZE,
            ))),
            zones: Arc::new(Mutex::new(LruCache::new(DEFAULT_ZONE_ENTRY_CACHE_SIZE))),
        }
    }

    pub fn get_nameserver<R: Resolver + Clone + 'static + Send>(
        &self,
        zone: &Name,
        resolver: R,
    ) -> Box<Future<Item = Nameserver, Error = failure::Error> + Send + 'static> {
        let (nameserver, missing_nameserver) = {
            let key = &EntryKey::from_name(zone);
            let mut zones = self.zones.lock().unwrap();
            if let Some(entry) = zones.get(key) {
                entry.select_nameserver(&mut self.nameservers.lock().unwrap())
            } else {
                (None, Vec::new())
            }
        };

        if nameserver.is_some() {
            if !missing_nameserver.is_empty() {
                spawn(
                    NameserverFetcher::new(missing_nameserver, self.nameservers.clone(), resolver)
                        .map_err(|e| println!("query missing nameserver failed:{:?}", e)),
                );
            }
            return Box::new(future::ok(nameserver.unwrap()));
        } else {
            return Box::new(ZoneFetcher::new(
                zone.clone(),
                resolver,
                self.nameservers.clone(),
                self.zones.clone(),
            ));
        }
    }

    pub fn update_nameserver_rtt(&self, nameserver: &Nameserver) {
        let mut nameservers = self.nameservers.lock().unwrap();
        let key = &EntryKey::from_name(&nameserver.name);
        if let Some(entry) = nameservers.get_mut(key) {
            entry.update_nameserver(nameserver);
        }
    }
}
