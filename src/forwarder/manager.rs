use super::{
    forwarder::Forwarder,
    group::{ForwarderGroup, ForwarderPool},
};
use crate::nsas::{AbstractNameserver, NameserverStore};
use datasrc::RBTree;
use r53::Name;
use std::sync::RwLock;

pub struct ForwarderManager {
    forwarders: RBTree<ForwarderGroup>,
    pool: RwLock<ForwarderPool>,
}

impl ForwarderManager {
    pub fn get_forwarder(&self, name: &Name) -> Option<Forwarder> {
        let result = self.forwarders.find(name);
        if let Some(selecotr) = result.get_value() {
            let pool = self.pool.read().unwrap();
            return Some(selecotr.select_forwarder(&pool));
        } else {
            return None;
        }
    }
}

impl NameserverStore<Forwarder> for ForwarderManager {
    fn update_nameserver_rtt(&self, nameserver: &Forwarder) {
        let mut pool = self.pool.write().unwrap();
        let position = pool.0.iter().position(|s| s == nameserver);
        if let Some(pos) = position {
            pool.0[pos].set_rtt(nameserver.get_rtt());
        }
    }
}
