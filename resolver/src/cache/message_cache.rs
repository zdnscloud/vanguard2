use super::{
    cache::MessageCache, entry_key::EntryKey, message_cache_entry::MessageEntry,
    message_util::can_message_be_cached, rrset_cache::RRsetLruCache,
};
use lru::LruCache;
use r53::{Message, Name, RData, RRType, RRset};

const DEFAULT_MESSAGE_CACHE_SIZE: usize = 10000;

pub struct MessageLruCache {
    messages: LruCache<EntryKey, MessageEntry>,
    positive_cache: RRsetLruCache,
    negative_cache: RRsetLruCache,
}

impl MessageLruCache {
    pub fn new(mut cap: usize) -> Self {
        if cap == 0 {
            cap = DEFAULT_MESSAGE_CACHE_SIZE;
        }
        MessageLruCache {
            messages: LruCache::new(cap),
            positive_cache: RRsetLruCache::new(2 * cap),
            negative_cache: RRsetLruCache::new(cap),
        }
    }
}

impl MessageCache for MessageLruCache {
    fn len(&self) -> usize {
        self.messages.len()
    }

    fn get_deepest_ns(&mut self, name: &Name) -> Option<Name> {
        let key = &EntryKey(name as *const Name, RRType::NS);
        if let Some(mut rrset) = self.positive_cache.get_rrset_with_key(key) {
            match rrset.rdatas.pop().unwrap() {
                RData::NS(rdata) => {
                    return Some(rdata.name);
                }
                _ => unreachable!(),
            }
        } else if let Ok(parent) = name.parent(1) {
            return self.get_deepest_ns(&parent);
        } else {
            return None;
        };
    }

    fn gen_response(&mut self, query: &mut Message) -> bool {
        let key = &EntryKey(
            &query.question.as_ref().unwrap().name as *const Name,
            query.question.as_ref().unwrap().typ,
        );
        if let Some(entry) = self.messages.get(key) {
            let succeed =
                entry.fill_message(query, &mut self.positive_cache, &mut self.negative_cache);
            if !succeed {
                self.messages.pop(key);
            }
            succeed
        } else {
            self.positive_cache.gen_response(key, query)
        }
    }

    fn add_message(&mut self, message: Message) {
        if !can_message_be_cached(&message) {
            return;
        }

        let key = &EntryKey(
            &message.question.as_ref().unwrap().name as *const Name,
            message.question.as_ref().unwrap().typ,
        );
        self.messages.pop(key);
        let entry = MessageEntry::new(message, &mut self.positive_cache, &mut self.negative_cache);
        self.messages.put(entry.key(), entry);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use r53::{
        edns::Edns, header_flag, message::SectionType, MessageBuilder, RRType, RRset, Rcode,
    };
    use std::str::FromStr;

    fn build_positive_response() -> Message {
        let mut msg = Message::with_query(Name::new("test.example.com.").unwrap(), RRType::A);
        {
            let mut builder = MessageBuilder::new(&mut msg);
            builder
                .id(1200)
                .rcode(Rcode::NoError)
                .set_flag(header_flag::HeaderFlag::RecursionDesired)
                .add_answer(RRset::from_str("test.example.com. 3600 IN A 192.0.2.2").unwrap())
                .add_answer(RRset::from_str("test.example.com. 3600 IN A 192.0.2.1").unwrap())
                .add_auth(RRset::from_str("example.com. 100 IN NS ns1.example.com.").unwrap())
                .add_additional(RRset::from_str("ns1.example.com. 3600 IN A 2.2.2.2").unwrap())
                .edns(Edns {
                    versoin: 0,
                    extened_rcode: 0,
                    udp_size: 4096,
                    dnssec_aware: false,
                    options: None,
                })
                .done();
        }
        msg
    }

    #[test]
    fn test_message_cache() {
        let mut cache = MessageLruCache::new(100);
        let mut query = Message::with_query(Name::new("test.example.com.").unwrap(), RRType::A);
        assert!(!cache.gen_response(&mut query));
        cache.add_message(build_positive_response());
        assert!(cache.gen_response(&mut query));
        assert_eq!(query.header.rcode, Rcode::NoError);
        assert!(header_flag::is_flag_set(
            query.header.flag,
            header_flag::HeaderFlag::QueryRespone
        ));
        assert!(!header_flag::is_flag_set(
            query.header.flag,
            header_flag::HeaderFlag::AuthenticData
        ));
        assert_eq!(query.header.an_count, 2);
        let answers = query.section(SectionType::Answer).unwrap();
        assert_eq!(answers.len(), 1);
        assert_eq!(answers[0].rdatas[0].to_string(), "192.0.2.2");

        let mut query = Message::with_query(Name::new("example.com.").unwrap(), RRType::NS);
        assert!(cache.gen_response(&mut query));
        assert_eq!(query.header.an_count, 1);

        let deepest_ns = cache.get_deepest_ns(&Name::new("example.cn.").unwrap());
        assert!(deepest_ns.is_none());

        let deepest_ns = cache.get_deepest_ns(&Name::new("a.b.c.example.com.").unwrap());
        assert!(deepest_ns.is_some());
        assert_eq!(deepest_ns.unwrap(), Name::new("ns1.example.com.").unwrap());
    }
}
