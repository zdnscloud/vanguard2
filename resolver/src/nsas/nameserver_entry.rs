use crate::nsas::{
    address_entry::{self, AddressEntry},
    entry_key::EntryKey,
};
use lru::LruCache;
use r53::Name;
use std::{
    cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    time::{Duration, Instant},
};

#[derive(Clone)]
pub struct Nameserver {
    pub name: Name,
    pub address: IpAddr,
    pub rtt: u32,
}

impl PartialEq for Nameserver {
    fn eq(&self, other: &Nameserver) -> bool {
        self.name.eq(&other.name)
    }
}

impl Eq for Nameserver {}

impl PartialOrd for Nameserver {
    fn partial_cmp(&self, other: &Nameserver) -> Option<Ordering> {
        self.rtt.partial_cmp(&other.rtt)
    }
}

impl Ord for Nameserver {
    fn cmp(&self, other: &Nameserver) -> Ordering {
        self.rtt.cmp(&other.rtt)
    }
}

pub struct NameserverEntry {
    name: *mut Name,
    addresses: Vec<AddressEntry>,
    expire_time: Instant,
}

pub type NameserverCache = LruCache<EntryKey, NameserverEntry>;

unsafe impl Send for NameserverEntry {}

impl NameserverEntry {
    pub fn new(name: Name, addresses: Vec<AddressEntry>) -> Self {
        debug_assert!(!addresses.is_empty());

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
    pub fn get_addresses(&self) -> &Vec<AddressEntry> {
        &self.addresses
    }

    #[inline]
    pub fn select_nameserver(&self) -> Nameserver {
        let addr = address_entry::select_address(&self.addresses).unwrap();
        Nameserver {
            name: self.get_name().clone(),
            address: addr.get_addr(),
            rtt: addr.get_rtt(),
        }
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

        let entry = NameserverEntry::new(
            Name::new("n1").unwrap(),
            vec![AddressEntry::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 0)],
        );
        cache.put(entry.get_key(), entry);

        let name = Name::new("n1").unwrap();
        let key = EntryKey::from_name(&name);
        let entry = cache.get(&key);
        assert!(entry.is_some());
    }
}
