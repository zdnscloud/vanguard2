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
    fn len(&self) -> usize;
    fn get_rrset(&mut self, name_and_type: &Name, typ: RRType) -> Option<RRset>;
    fn add_rrset(&mut self, rrset: RRset, trust_level: RRsetTrustLevel);
}

pub trait MessageCache {
    fn len(&self) -> usize;
    fn gen_response(&mut self, query: &mut Message) -> bool;
    fn add_message(&mut self, message: Message);
    fn get_deepest_ns(&mut self, name: &Name) -> Option<Name>;
}
