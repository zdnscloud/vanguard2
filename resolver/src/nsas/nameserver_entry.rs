use crate::nsas::{address_entry::AddressEntry, address_selector, entry_key::EntryKey};
use lru::LruCache;
use r53::Name;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::{Duration, Instant};

pub struct NameserverEntry {
    name: Name,
    addresses: Vec<AddressEntry>,
    expire_time: Instant,
}

pub type NameserverCache = LruCache<EntryKey, NameserverEntry>;

impl NameserverEntry {
    pub fn new(name: Name, addresses: Vec<AddressEntry>) -> Self {
        NameserverEntry {
            name,
            addresses,
            expire_time: Instant::now(),
        }
    }

    #[inline]
    pub fn get_name(&self) -> &Name {
        &self.name
    }

    #[inline]
    pub fn get_key(&self) -> EntryKey {
        EntryKey(&self.name as *const Name)
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

    /*
    pub fn select_address(&self) -> Option<&AddressEntry> {
        address_selector::select_address(&self.addresses)
    }
    */
}
