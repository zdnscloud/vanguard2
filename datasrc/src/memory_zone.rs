use crate::domaintree::{node::NodePtr, node_chain::NodeChain, tree::RBTree};
use crate::rdataset::Rdataset;
use crate::zone::{FindOption, FindResult, FindResultType, ZoneFinder};
use r53::{Name, RRType, RRset};

type ZoneData = RBTree<Rdataset>;

pub struct MemoryZone {
    origin: Name,
    data: ZoneData,
}

impl MemoryZone {
    pub fn new(name: Name) -> Self {
        MemoryZone {
            origin: name,
            data: ZoneData::new(),
        }
    }
}

pub struct MemoryZoneFindResult<'a> {
    node_chain: NodeChain<Rdataset>,
    zone: &'a MemoryZone,
}

impl<'a> FindResult for MemoryZoneFindResult<'a> {
    fn get_result_type(&self) -> FindResultType {
        FindResultType::Success
    }

    fn get_rrset(&self) -> Option<RRset> {
        None
    }

    fn get_additional(&self) -> Vec<RRset> {
        Vec::new()
    }
}

impl<'a> ZoneFinder<'a> for MemoryZone {
    type FindResult = MemoryZoneFindResult<'a>;

    fn get_origin(&self) -> &Name {
        &self.origin
    }

    fn find(&self, name: &Name, typ: RRType, opt: FindOption) -> MemoryZoneFindResult {
        let mut node_path = NodeChain::new();
        MemoryZoneFindResult {
            node_chain: node_path,
            zone: self,
        }
    }
}
