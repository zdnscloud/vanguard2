use crate::nsas::{
    address_entry::AddressEntry,
    address_selector,
    entry_key::EntryKey,
    nameserver_entry::{NameserverCache, NameserverEntry},
};
use r53::Name;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};
use std::time::{Duration, Instant};

pub struct ZoneEntry {
    name: *mut Name,
    nameservers: Vec<Name>,
    expire_time: Instant,
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
    ) -> (Option<AddressEntry>, Vec<Name>) {
        let mut missing_names = self.nameservers.clone();
        let mut servers = Vec::with_capacity(missing_names.len());
        for i in (0..missing_names.len()).rev() {
            let name = missing_names.swap_remove(i);
            let key = &EntryKey::from_name(&name);
            let mut nameserver_is_healthy = false;
            if let Some(entry) = nameservers.get(key) {
                if let Some(addr) = entry.select_address() {
                    servers.push(addr);
                    nameserver_is_healthy = true;
                }
            }
            if !nameserver_is_healthy {
                missing_names.push(name);
            }
        }
        (
            address_selector::select_address(&servers).map(|a| a.clone()),
            missing_names,
        )
    }

    pub fn get_nameservers(&self) -> Vec<Name> {
        self.nameservers.clone()
    }
}

impl Drop for ZoneEntry {
    fn drop(&mut self) {
        unsafe {
            Box::from_raw(self.name);
        }
    }
}
