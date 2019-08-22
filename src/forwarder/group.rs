use super::forwarder::Forwarder;
use std::{cell::Cell, cmp::Eq, ops::Rem};

pub struct ForwarderPool(pub Vec<Forwarder>);

impl ForwarderPool {
    pub fn get_forwarder(&self, index: usize) -> Forwarder {
        self.0[index]
    }
}

pub struct ForwarderGroup {
    indexes: Vec<usize>,
}

impl ForwarderGroup {
    pub fn select_forwarder(&self, pool: &ForwarderPool) -> Forwarder {
        if self.indexes.len() == 1 {
            return pool.get_forwarder(self.indexes[0]);
        }

        let count = self.indexes.len();
        self.indexes
            .iter()
            .fold(Vec::with_capacity(count), |mut fs, &index| {
                fs.push(pool.get_forwarder(index));
                fs
            })
            .iter()
            .min()
            .map(|f| *f)
            .unwrap()
    }
}
