use crate::nsas::{address_entry::AddressEntry, address_selector, entry_key::EntryKey};
use lru::LruCache;
use r53::Name;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::{Duration, Instant};

pub struct NameserverEntry {
    name: *mut Name,
    addresses: Vec<AddressEntry>,
    expire_time: Instant,
}

pub type NameserverCache = LruCache<EntryKey, NameserverEntry>;

unsafe impl Send for NameserverEntry {}

impl NameserverEntry {
    pub fn new(name: Name, addresses: Vec<AddressEntry>) -> Self {
        let name = Box::into_raw(Box::new(name));
        NameserverEntry {
            name,
            addresses,
            expire_time: Instant::now(),
        }
    }

    #[inline]
    pub fn get_key(&self) -> EntryKey {
        EntryKey(self.name)
    }

    #[inline]
    pub fn get_name(&self) -> &Name {
        unsafe { &(*self.name) }
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.addresses.is_empty()
    }

    #[inline]
    pub fn get_addresses(&self) -> &Vec<AddressEntry> {
        &self.addresses
    }

    #[inline]
    pub fn select_address(&self) -> Option<AddressEntry> {
        address_selector::select_address(&self.addresses)
    }

    pub fn update_address_rtt(&mut self, target: IpAddr, rtt: u32) {
        for addr in self.addresses.iter_mut() {
            if addr.get_addr() == target {
                addr.set_rtt(rtt);
                return;
            }
        }
    }

    pub fn set_address_unreachable(&mut self, target: IpAddr) {
        for addr in self.addresses.iter_mut() {
            if addr.get_addr() == target {
                addr.set_unreachable();
                return;
            }
        }
    }
}

impl Drop for NameserverEntry {
    fn drop(&mut self) {
        unsafe {
            Box::from_raw(self.name);
        }
    }
}

mod test {
    use super::*;
    use lru::LruCache;
    use r53::Name;

    #[test]
    fn test_nameserver_cache() {
        let mut cache: NameserverCache = LruCache::new(10);

        let entry = NameserverEntry::new(Name::new("n1").unwrap(), Vec::new());
        cache.put(entry.get_key(), entry);

        let name = Name::new("n1").unwrap();
        let key = EntryKey::from_name(&name);
        let entry = cache.get(&key);
        assert!(entry.is_some());
    }
}
