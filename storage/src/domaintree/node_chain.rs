use crate::domaintree::node::{NodePtr, RBTreeNode};
use r53::{name::NameComparisonResult, Name, NameRelation};

pub struct NodeChain<T> {
    pub level_count: usize,
    pub nodes: [NodePtr<T>; 256],
    pub last_compared: NodePtr<T>,
    pub last_compared_result: NameComparisonResult,
}

impl<T> NodeChain<T> {
    pub fn new() -> Self {
        NodeChain {
            level_count: 0,
            nodes: [NodePtr::null(); 256],
            last_compared: NodePtr::null(),
            last_compared_result: NameComparisonResult {
                order: 0,
                common_label_count: 0,
                relation: NameRelation::Equal,
            },
        }
    }

    pub fn get_absolute_name(&self) -> Name {
        let mut name = self.top().get_name().clone();
        let mut level = self.level_count - 1;
        while level > 0 {
            name = name.concat(self.nodes[level - 1].get_name()).unwrap();
            level -= 1;
        }
        name
    }

    fn top(&self) -> NodePtr<T> {
        self.nodes[self.level_count - 1]
    }

    fn pop(&mut self) {
        self.level_count -= 1;
    }

    pub fn push(&mut self, node: NodePtr<T>) {
        self.nodes[self.level_count] = node;
        self.level_count += 1;
    }
}
