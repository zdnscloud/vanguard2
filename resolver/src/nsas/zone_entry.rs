use crate::nsas::nameserver_entry::NameserverEntry;
use r53::Name;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::{Duration, Instant};

pub struct ZoneEntry {
    name: Name,
    nameservers: Vec<Name>,
    expire_time: Instant,
}

pub struct NameServer {
    zone: Name,
    nameserver: Name,
    address: Ipv4Addr,
}

impl ZoneEntry {
    pub fn new(name: Name, nameservers: Vec<Name>, ttl: Duration) -> Self {
        ZoneEntry {
            name,
            nameservers,
            expire_time: Instant::now()
                .checked_add(ttl)
                .expect("zone ttl out of range"),
        }
    }
}
