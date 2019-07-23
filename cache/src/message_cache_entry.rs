use crate::cache::RRsetCache;
use crate::cache_entry_key::EntryKey;
use crate::message_util::{get_rrset_trust_level, is_negative_response};
use crate::rrset_cache::RRsetLruCache;
use r53::{message::SectionType, Message, MessageBuilder, Name, RRTtl, RRType, RRset};
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub struct RRsetRef {
    pub name: Name,
    pub typ: RRType,
    pub is_negative: bool,
}

#[derive(Clone, Debug)]
pub struct MessageEntry {
    name: *mut Name,
    typ: RRType,
    answer_rrset_count: u16,
    auth_rrset_count: u16,
    additional_rrset_count: u16,
    rrset_refs: Vec<RRsetRef>,
    expire_time: Instant,
}

impl MessageEntry {
    pub fn new(
        mut message: Message,
        positive_cache: &mut RRsetLruCache,
        negative_cache: &mut RRsetLruCache,
    ) -> Self {
        let mut min_ttl = RRTtl(u32::max_value());
        let response_is_negative = is_negative_response(&message);
        let mut rrset_refs = Vec::new();
        let mut answer_rrset_count = 0;
        let mut auth_rrset_count = 0;
        let mut additional_rrset_count = 0;
        if let Some(answers) = message.sections[SectionType::Answer as usize].0.take() {
            answer_rrset_count = answers.len() as u16;
            rrset_refs.append(&mut add_rrset_in_section(
                positive_cache,
                &message,
                answers,
                SectionType::Answer,
                &mut min_ttl,
            ));
        }
        if response_is_negative {
            if let Some(authorities) = message.sections[SectionType::Auth as usize].0.take() {
                auth_rrset_count = authorities.len() as u16;
                rrset_refs.append(&mut add_rrset_in_negative_response_auth_section(
                    positive_cache,
                    negative_cache,
                    &message,
                    authorities,
                    &mut min_ttl,
                ));
            }
        } else {
            if let Some(authorities) = message.sections[SectionType::Auth as usize].0.take() {
                auth_rrset_count = authorities.len() as u16;
                rrset_refs.append(&mut add_rrset_in_section(
                    positive_cache,
                    &message,
                    authorities,
                    SectionType::Auth,
                    &mut min_ttl,
                ));
            }
        }

        if let Some(additionals) = message.sections[SectionType::Additional as usize].0.take() {
            additional_rrset_count = additionals.len() as u16;
            rrset_refs.append(&mut add_rrset_in_section(
                positive_cache,
                &message,
                additionals,
                SectionType::Additional,
                &mut min_ttl,
            ));
        }
        let expire_time = Instant::now()
            .checked_add(Duration::from_secs(min_ttl.0 as u64))
            .unwrap();

        MessageEntry {
            name: Box::into_raw(Box::new(message.question.name)),
            typ: message.question.typ,
            answer_rrset_count,
            auth_rrset_count,
            additional_rrset_count,
            rrset_refs,
            expire_time,
        }
    }

    pub fn key(&self) -> EntryKey {
        EntryKey(self.name, self.typ)
    }

    pub fn get_message(
        &self,
        positive_cache: &mut RRsetLruCache,
        negative_cache: &mut RRsetLruCache,
    ) -> Option<Message> {
        if self.expire_time <= Instant::now() {
            return None;
        }

        let rrsets = self.get_rrsets(positive_cache, negative_cache);
        if rrsets.is_none() {
            return None;
        }

        let mut message = unsafe { Message::with_query((*self.name).clone(), self.typ) };
        let mut builder = MessageBuilder::new(&mut message);
        let mut iter = rrsets.unwrap().into_iter();
        for _ in 0..self.answer_rrset_count {
            builder.add_answer(iter.next().unwrap());
        }
        for _ in 0..self.auth_rrset_count {
            builder.add_auth(iter.next().unwrap());
        }

        for _ in 0..self.additional_rrset_count {
            builder.add_additional(iter.next().unwrap());
        }
        builder.done();
        Some(message)
    }

    fn get_rrsets(
        &self,
        positive_cache: &mut RRsetLruCache,
        negative_cache: &mut RRsetLruCache,
    ) -> Option<Vec<RRset>> {
        let rrset_count = self.rrset_refs.len();
        let mut rrsets = Vec::with_capacity(rrset_count);
        for rrset_ref in &self.rrset_refs {
            let rrset = if rrset_ref.is_negative {
                negative_cache.get_rrset(&rrset_ref.name, rrset_ref.typ)
            } else {
                positive_cache.get_rrset(&rrset_ref.name, rrset_ref.typ)
            };

            if let Some(rrset) = rrset {
                rrsets.push(rrset);
            } else {
                return None;
            }
        }
        Some(rrsets)
    }
}

impl Drop for MessageEntry {
    fn drop(&mut self) {
        unsafe {
            Box::from_raw(self.name);
        }
    }
}

fn add_rrset_in_section(
    positive_cache: &mut RRsetLruCache,
    message: &Message,
    rrsets: Vec<RRset>,
    section: SectionType,
    min_ttl: &mut RRTtl,
) -> Vec<RRsetRef> {
    let mut refs = Vec::with_capacity(rrsets.len());
    for rrset in rrsets.into_iter() {
        refs.push(RRsetRef {
            name: rrset.name.clone(),
            typ: rrset.typ,
            is_negative: false,
        });
        if rrset.ttl.0 < min_ttl.0 {
            *min_ttl = rrset.ttl;
        }
        let trust_level = get_rrset_trust_level(message, &rrset, section);
        positive_cache.add_rrset(rrset, trust_level);
    }
    refs
}

fn add_rrset_in_negative_response_auth_section(
    positive_cache: &mut RRsetLruCache,
    negative_cache: &mut RRsetLruCache,
    message: &Message,
    rrsets: Vec<RRset>,
    min_ttl: &mut RRTtl,
) -> Vec<RRsetRef> {
    let mut refs = Vec::with_capacity(rrsets.len());
    for rrset in rrsets.into_iter() {
        refs.push(RRsetRef {
            name: rrset.name.clone(),
            typ: rrset.typ,
            is_negative: rrset.typ == RRType::SOA,
        });
        if rrset.ttl.0 < min_ttl.0 {
            *min_ttl = rrset.ttl;
        }
        let trust_level = get_rrset_trust_level(message, &rrset, SectionType::Auth);
        if rrset.typ == RRType::SOA {
            negative_cache.add_rrset(rrset, trust_level);
        } else {
            positive_cache.add_rrset(rrset, trust_level);
        }
    }
    refs
}

#[cfg(test)]
mod tests {}
