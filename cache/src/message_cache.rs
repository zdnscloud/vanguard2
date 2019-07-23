use crate::cache::MessageCache;
use crate::cache_entry_key::EntryKey;
use crate::message_cache_entry::MessageEntry;
use crate::message_util::can_message_be_cached;
use crate::rrset_cache::RRsetLruCache;
use lru::LruCache;
use r53::{Message, Name, RRType};

pub struct MessageLruCache {
    messages: LruCache<EntryKey, MessageEntry>,
    positive_cache: RRsetLruCache,
    negative_cache: RRsetLruCache,
}

impl MessageLruCache {
    pub fn new(cap: usize) -> Self {
        MessageLruCache {
            messages: LruCache::new(cap),
            positive_cache: RRsetLruCache::new(cap),
            negative_cache: RRsetLruCache::new(cap),
        }
    }
}

impl MessageCache for MessageLruCache {
    fn get_message(&mut self, name: &Name, typ: RRType) -> Option<Message> {
        let key = &EntryKey(name as *const Name, typ);
        if let Some(entry) = self.messages.get(key) {
            let message = entry.get_message(&mut self.positive_cache, &mut self.negative_cache);
            if message.is_none() {
                self.messages.pop(key);
            }
            message
        } else {
            None
        }
    }

    fn add_message(&mut self, message: Message) {
        if !can_message_be_cached(&message) {
            return;
        }

        let key = &EntryKey(&message.question.name as *const Name, message.question.typ);
        self.messages.pop(key);
        let entry = MessageEntry::new(message, &mut self.positive_cache, &mut self.negative_cache);
        self.messages.put(entry.key(), entry);
    }
}

#[cfg(test)]
mod tests {
    /*
    use super::*;
    use std::str::FromStr;
    #[test]
    fn test_rrset_cache() {
        let mut cache = MessageLruCache::new(2);

        let rrset = RRset::from_str("www.zdns.cn 300 IN A 1.1.1.1").unwrap();
        assert!(cache.get_rrset(&rrset.name, rrset.typ).is_none());
        cache.add_rrset(rrset.clone(), RRsetTrustLevel::NonAuthAnswerWithAA);
        let insert_rrset = cache.get_rrset(&rrset.name, rrset.typ).unwrap();
        assert_eq!(insert_rrset.rdatas, rrset.rdatas);

        let low_trust_level_rrset = RRset::from_str("www.zdns.cn 300 IN A 2.2.2.2").unwrap();
        cache.add_rrset(low_trust_level_rrset.clone(), RRsetTrustLevel::Default);
        let insert_rrset = cache.get_rrset(&rrset.name, rrset.typ).unwrap();
        assert_eq!(insert_rrset.rdatas, rrset.rdatas);

        cache.add_rrset(low_trust_level_rrset.clone(), RRsetTrustLevel::PrimNonGlue);
        let insert_rrset = cache.get_rrset(&rrset.name, rrset.typ).unwrap();
        assert_eq!(insert_rrset.rdatas, low_trust_level_rrset.rdatas);
    }
    */
}
