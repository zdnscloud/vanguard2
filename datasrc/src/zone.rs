use failure_ext::prelude::*;
use r53::{Name, RRType, RRset};

pub enum FindResultType {
    Success,
    Delegation,
    NXDomain,
    NXRRset,
    CName,
    ServerFailed,
}

pub enum FindOption {
    FollowZoneCut,
    GlueOK,
}

pub trait FindResult {
    fn get_result_type(&self) -> FindResultType;
    fn get_rrset(&self) -> Option<RRset>;
    fn get_additional(&self) -> Vec<RRset>;
}

pub trait ZoneFinder<'a> {
    type FindResult: FindResult;
    fn get_origin(&self) -> &Name;
    fn find(&'a self, name: &Name, typ: RRType, opt: FindOption) -> Self::FindResult;
}
