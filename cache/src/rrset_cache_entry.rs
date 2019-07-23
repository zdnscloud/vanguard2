use crate::cache::RRsetTrustLevel;
use r53::{Name, RData, RRClass, RRTtl, RRType, RRset};
use std::{
    cmp::{Eq, PartialEq},
    fmt::{self, Debug},
    hash::{Hash, Hasher},
    time::{Duration, Instant},
};

pub struct RRsetEntryKey(pub *const Name, pub RRType);

impl RRsetEntryKey {
    pub fn new(name: Name, typ: RRType) -> Self {
        RRsetEntryKey(Box::into_raw(Box::new(name)), typ)
    }
}

impl Clone for RRsetEntryKey {
    fn clone(&self) -> Self {
        RRsetEntryKey(self.0, self.1)
    }
}

impl Copy for RRsetEntryKey {}

impl Debug for RRsetEntryKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        unsafe { write!(f, "{}:{}", (*self.0), self.1) }
    }
}

impl Hash for RRsetEntryKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        unsafe {
            (*self.0).hash(state);
        }
        state.write_u16(self.1.to_u16());
    }
}

impl PartialEq for RRsetEntryKey {
    fn eq(&self, other: &RRsetEntryKey) -> bool {
        unsafe { self.1 == other.1 && (*self.0).eq(&(*other.0)) }
    }
}

impl Eq for RRsetEntryKey {}

#[derive(Clone, Debug)]
pub struct RRsetEntry {
    name: *mut Name,
    typ: RRType,
    pub trust_level: RRsetTrustLevel,
    rdatas: Vec<RData>,
    expire_time: Instant,
}

impl RRsetEntry {
    pub fn new(rrset: RRset, trust_level: RRsetTrustLevel) -> Self {
        let expire_time = Instant::now()
            .checked_add(Duration::from_secs(rrset.ttl.0 as u64))
            .unwrap();

        let name = Box::into_raw(Box::new(rrset.name));
        RRsetEntry {
            name,
            typ: rrset.typ,
            trust_level,
            rdatas: rrset.rdatas,
            expire_time,
        }
    }

    pub fn key(&self) -> RRsetEntryKey {
        RRsetEntryKey(self.name, self.typ)
    }

    pub fn is_expired(&self) -> bool {
        self.expire_time <= Instant::now()
    }

    pub fn get_rrset(&self) -> Option<RRset> {
        let now = Instant::now();
        if self.expire_time <= now {
            return None;
        }

        unsafe {
            Some(RRset {
                name: (*self.name).clone(),
                typ: self.typ,
                class: RRClass::IN,
                ttl: RRTtl(self.expire_time.duration_since(now).as_secs() as u32),
                rdatas: self.rdatas.clone(),
            })
        }
    }
}

impl Drop for RRsetEntry {
    fn drop(&mut self) {
        unsafe {
            Box::from_raw(self.name);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    #[test]
    fn test_rrset_entry() {
        let rrset = RRset::from_str("www.zdns.cn 300 IN A 2.2.2.2").unwrap();
        let entry = RRsetEntry::new(rrset.clone(), RRsetTrustLevel::Default);
        let entry_key = entry.key();
        assert_eq!(
            entry_key,
            RRsetEntryKey(
                &Name::from_str("www.zdns.cn").unwrap() as *const Name,
                RRType::A,
            )
        );

        let mut rrset_with_new_ttl = entry.get_rrset().unwrap();
        assert!(rrset_with_new_ttl.ttl != rrset.ttl);
        rrset_with_new_ttl.ttl = rrset.ttl;
        assert_eq!(rrset_with_new_ttl, rrset);
    }
}
