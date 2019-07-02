use failure_ext::prelude::*;
use r53::{Name, RRType, RRset};

pub enum FindResultFlag {
    Success,
    Delegation,
    NXDomain,
    NXRRset,
    CName,
}

pub enum FindOption {
    FollowZoneCut,
    GlueOK,
}

pub struct FindResult {
    flag: FindResultFlag,
    rrset: Option<RRset>,
    additional: Vec<RRset>,
}

pub trait ZoneFinder {
    fn get_origin(&self) -> &Name;
    fn find(name: &Name, typ: RRType, opt: FindOption) -> FindResult;
}

pub trait ZoneUpdator {
    type Transaction: ZoneTransaction;
    fn begin(&mut self) -> Self::Transaction;
}

pub trait ZoneTransaction {
    fn add_rrset(&mut self, rrset: &RRset) -> Result<()>;
    fn delete_rrset(&mut self, rrset: &RRset) -> Result<()>;
    fn delete_name(&mut self, name: &Name) -> Result<()>;
    fn delete_rr(&mut self, rrset: &RRset) -> Result<()>;
    fn increase_serial_number(&mut self) -> Result<()>;

    fn commit(self) -> Result<()>;
    fn roll_back(self) -> Result<()>;
}
