use crate::cache::RRsetTrustLevel;
use r53::{header_flag, message::SectionType, Message, RRType, Rcode};

pub(crate) fn is_negative_response(message: &Message) -> bool {
    let rcode = message.header.rcode;
    if rcode == Rcode::NXDomian {
        return true;
    }
    if rcode == Rcode::NoError {
        if message.header.an_count == 0 {
            if has_rrset_with_type_in_auth_sec(message, RRType::SOA) {
                return true;
            } else if !has_rrset_with_type_in_auth_sec(message, RRType::NS) {
                return true;
            }
        }
    }
    false
}

pub(crate) fn can_message_be_cached(message: &Message) -> bool {
    if is_negative_response(message) && !has_rrset_with_type_in_auth_sec(message, RRType::SOA) {
        false
    } else {
        true
    }
}

fn has_rrset_with_type_in_auth_sec(message: &Message, typ: RRType) -> bool {
    let sections = &message.sections[SectionType::Auth as usize];
    if sections.0.is_none() {
        return false;
    }

    for rrset in sections.0.as_ref().unwrap() {
        if rrset.typ == typ {
            return true;
        }
    }
    false
}

pub(crate) fn get_rrset_trust_level(message: &Message, section: SectionType) -> RRsetTrustLevel {
    let aa = header_flag::is_flag_set(message.header.flag, header_flag::HeaderFlag::AuthAnswer);
    match section {
        SectionType::Answer => {
            if aa {
                return RRsetTrustLevel::AnswerWithAA;
            } else {
                return RRsetTrustLevel::AnswerWithoutAA;
            }
        }
        SectionType::Auth => {
            if aa {
                return RRsetTrustLevel::AuthorityWithAA;
            } else {
                return RRsetTrustLevel::AuthorityWithoutAA;
            }
        }
        SectionType::Additional => {
            if aa {
                return RRsetTrustLevel::AdditionalWithAA;
            } else {
                return RRsetTrustLevel::AdditionalWithoutAA;
            }
        }
    }
}
