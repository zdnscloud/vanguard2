use crate::{
    message_classifier::{classify_response, ResponseCategory},
    nsas::{
        entry_key::EntryKey, error::NSASError, nameserver_entry::NameserverEntry,
        zone_entry::ZoneEntry,
    },
    Resolver,
};
use failure::Result;
use futures::Future;
use lru::LruCache;
use r53::{Message, Name, RRType};
use std::{
    io,
    net::Ipv4Addr,
    sync::{Arc, Mutex},
};
use tokio::executor::spawn;

const DEFAULT_ZONE_ENTRY_CACHE_SIZE: usize = 1009;
const DEFAULT_NAMESERVER_ENTRY_CACHE_SIZE: usize = 3001;

pub struct ZoneCache<R: Resolver + Clone> {
    nameservers: LruCache<EntryKey, NameserverEntry>,
    zones: LruCache<EntryKey, ZoneEntry>,
    resolver: R,
}

impl<R: Resolver + Clone> ZoneCache<R> {
    pub fn new(resolver: R) -> Self {
        ZoneCache {
            nameservers: LruCache::new(DEFAULT_NAMESERVER_ENTRY_CACHE_SIZE),
            zones: LruCache::new(DEFAULT_ZONE_ENTRY_CACHE_SIZE),
            resolver,
        }
    }
}
