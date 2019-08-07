use crate::nsas::{address_entry::AddressEntry, entry_key::EntryKey};
use r53::Name;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::{Duration, Instant};

pub struct NameserverEntry {
    name: Name,
    addresses: Vec<AddressEntry>,
    expire_time: Instant,
}

impl NameserverEntry {
    pub fn new(name: Name, addresses: Vec<AddressEntry>) -> Self {
        NameserverEntry {
            name,
            addresses,
            expire_time: Instant::now(),
        }
    }

    pub fn get_name(&self) -> Name {
        self.name.clone()
    }

    pub fn is_empty(&self) -> bool {
        self.addresses.is_empty()
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

    pub fn get_address(&self) -> Option<Ipv4Addr> {
        self.select_address()
    }

    fn select_address(&self) -> Option<Ipv4Addr> {
        self.addresses
            .iter()
            .filter(|a| a.is_v4())
            .min()
            .map(|a| match a.get_addr() {
                IpAddr::V4(addr) => addr,
                _ => panic!("never shold be here"),
            })
    }
}
