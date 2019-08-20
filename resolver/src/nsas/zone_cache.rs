use crate::nsas::{
    entry_key::EntryKey,
    nameserver_cache::{Nameserver, NameserverCache, NameserverEntry},
};
use lru::LruCache;
use r53::Name;
use std::{
    fmt,
    time::{Duration, Instant},
};

pub struct ZoneEntry {
    name: *mut Name,
    nameservers: Vec<Name>,
    expire_time: Instant,
}

impl fmt::Debug for ZoneEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe {
            write!(
                f,
                "name:{:?} nameservers:{:?}",
                (*self.name),
                self.nameservers
            )
        }
    }
}

pub struct ZoneCache(pub LruCache<EntryKey, ZoneEntry>);

impl ZoneCache {
    pub fn add_zone(&mut self, entry: ZoneEntry) {
        let key = entry.get_key();
        self.0.pop(&key);
        self.0.put(key, entry);
    }

    pub fn get_zone(&mut self, key: &EntryKey) -> Option<&ZoneEntry> {
        self.0.get(key)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }
}

unsafe impl Send for ZoneEntry {}

impl ZoneEntry {
    pub fn new(name: Name, nameservers: Vec<Name>, ttl: Duration) -> Self {
        let name = Box::into_raw(Box::new(name));
        ZoneEntry {
            name,
            nameservers,
            expire_time: Instant::now()
                .checked_add(ttl)
                .expect("zone ttl out of range"),
        }
    }

    #[inline]
    pub fn get_key(&self) -> EntryKey {
        EntryKey(self.name)
    }

    pub fn select_nameserver(
        &self,
        nameservers: &mut NameserverCache,
    ) -> (Option<Nameserver>, Vec<Name>) {
        let mut missing_names = self.nameservers.clone();
        let mut servers = Vec::with_capacity(missing_names.len());
        for i in (0..missing_names.len()).rev() {
            let name = missing_names.swap_remove(i);
            let key = &EntryKey::from_name(&name);
            let mut nameserver_is_healthy = false;
            if let Some(entry) = nameservers.get_nameserver(key) {
                servers.push(entry.select_nameserver());
                nameserver_is_healthy = true;
            }
            if !nameserver_is_healthy {
                missing_names.push(name);
            }
        }
        if servers.is_empty() {
            (None, missing_names)
        } else {
            (
                Some(servers.iter().min().map(|a| a.clone()).unwrap()),
                missing_names,
            )
        }
    }

    pub fn get_server_names(&self) -> &Vec<Name> {
        &self.nameservers
    }
}

impl Drop for ZoneEntry {
    fn drop(&mut self) {
        unsafe {
            Box::from_raw(self.name);
        }
    }
}
