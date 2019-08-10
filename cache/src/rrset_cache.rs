use crate::cache::{RRsetCache, RRsetTrustLevel};
use crate::entry_key::EntryKey;
use crate::rrset_cache_entry::RRsetEntry;
use lru::LruCache;
use r53::{Name, RRType, RRset};

pub struct RRsetLruCache {
    rrsets: LruCache<EntryKey, RRsetEntry>,
}

impl RRsetLruCache {
    pub fn new(cap: usize) -> Self {
        RRsetLruCache {
            rrsets: LruCache::new(cap),
        }
    }
}

impl RRsetCache for RRsetLruCache {
    fn len(&self) -> usize {
        self.rrsets.len()
    }

    fn get_rrset(&mut self, name: &Name, typ: RRType) -> Option<RRset> {
        let key = &EntryKey(name as *const Name, typ);
        if let Some(entry) = self.rrsets.get(key) {
            let rrset = entry.get_rrset();
            if rrset.is_none() {
                self.rrsets.pop(key);
            }
            rrset
        } else {
            None
        }
    }

    fn add_rrset(&mut self, rrset: RRset, trust_level: RRsetTrustLevel) {
        let key = &EntryKey(&rrset.name as *const Name, rrset.typ);
        if let Some(entry) = self.rrsets.peek(key) {
            if !entry.is_expired() && entry.trust_level > trust_level {
                return;
            }
        }
        self.rrsets.pop(key);
        let entry = RRsetEntry::new(rrset, trust_level);
        self.rrsets.put(entry.key(), entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    #[test]
    fn test_rrset_cache() {
        let mut cache = RRsetLruCache::new(2);

        let rrset = RRset::from_str("www.zdns.cn 300 IN A 1.1.1.1").unwrap();
        assert!(cache.get_rrset(&rrset.name, rrset.typ).is_none());
        cache.add_rrset(rrset.clone(), RRsetTrustLevel::NonAuthAnswerWithAA);
        let insert_rrset = cache.get_rrset(&rrset.name, rrset.typ).unwrap();
        assert_eq!(insert_rrset.rdatas, rrset.rdatas);

        let low_trust_level_rrset = RRset::from_str("www.zdns.cn 300 IN A 2.2.2.2").unwrap();
        cache.add_rrset(
            low_trust_level_rrset.clone(),
            RRsetTrustLevel::AdditionalWithoutAA,
        );
        let insert_rrset = cache.get_rrset(&rrset.name, rrset.typ).unwrap();
        assert_eq!(insert_rrset.rdatas, rrset.rdatas);

        cache.add_rrset(low_trust_level_rrset.clone(), RRsetTrustLevel::PrimNonGlue);
        let insert_rrset = cache.get_rrset(&rrset.name, rrset.typ).unwrap();
        assert_eq!(insert_rrset.rdatas, low_trust_level_rrset.rdatas);
        assert_eq!(cache.len(), 1);

        let rrset = RRset::from_str("www1.zdns.cn 300 IN A 1.1.1.1").unwrap();
        cache.add_rrset(rrset.clone(), RRsetTrustLevel::NonAuthAnswerWithAA);
        let rrset = RRset::from_str("www2.zdns.cn 300 IN A 1.1.1.1").unwrap();
        cache.add_rrset(rrset.clone(), RRsetTrustLevel::NonAuthAnswerWithAA);
        assert_eq!(cache.len(), 2);
        assert!(cache
            .get_rrset(&Name::new("www.zdns.cn").unwrap(), RRType::A)
            .is_none());
        assert!(cache
            .get_rrset(&Name::new("www1.zdns.cn").unwrap(), RRType::A)
            .is_some());
    }

    #[test]
    fn test_rrset_cache_bench() {
        let mut cache = RRsetLruCache::new(10);
        assert_eq!(cache.len(), 0);
        for i in 0..1000 {
            let rrset = format!("www{}.zdns.cn 300 IN A 1.1.1.1", i);
            let rrset = RRset::from_str(rrset.as_ref()).unwrap();
            cache.add_rrset(rrset.clone(), RRsetTrustLevel::NonAuthAnswerWithAA);
        }
        assert_eq!(cache.len(), 10);

        for i in 0..989 {
            let rrset = format!("www{}.zdns.cn 300 IN A 1.1.1.1", i);
            let rrset = RRset::from_str(rrset.as_ref()).unwrap();
            assert!(cache.get_rrset(&rrset.name, rrset.typ).is_none());
        }

        for i in 990..1000 {
            let rrset = format!("www{}.zdns.cn 300 IN A 1.1.1.1", i);
            let rrset = RRset::from_str(rrset.as_ref()).unwrap();
            assert!(cache.get_rrset(&rrset.name, rrset.typ).is_some());
        }
    }
}
