use r53::{Message, Name, RRType, RRset};

#[derive(Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum RRsetTrustLevel {
    AdditionalWithoutAA,
    AuthorityWithoutAA,
    AdditionalWithAA,
    NonAuthAnswerWithAA,
    AnswerWithoutAA,
    PrimGlue,
    AuthorityWithAA,
    AnswerWithAA,
    PrimNonGlue,
}

pub trait RRsetCache {
    fn get_rrset(&mut self, name_and_type: &Name, typ: RRType) -> Option<RRset>;
    fn add_rrset(&mut self, rrset: RRset, trust_level: RRsetTrustLevel);
}

pub trait MessageCache {
    fn get_message(&mut self, name: &Name, typ: RRType) -> Option<Message>;
    fn add_message(&mut self, message: Message);
}
