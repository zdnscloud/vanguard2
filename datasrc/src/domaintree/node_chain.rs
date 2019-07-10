use crate::domaintree::{node::NodePtr, tree::RBTree};
use r53::{name::NameComparisonResult, name::MAX_LABEL_COUNT, Name, NameRelation};
use std::fmt;

pub struct NodeChain<'a, T: 'a> {
    tree: &'a RBTree<T>,
    pub level_count: usize,
    pub nodes: [NodePtr<T>; MAX_LABEL_COUNT as usize],
    pub last_compared: NodePtr<T>,
    pub last_compared_result: NameComparisonResult,
}

impl<'a, T> fmt::Display for NodeChain<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.last_compared.is_null() {
            write!(f, "level: {}, last_compared is nil", self.level_count)
        } else {
            write!(
                f,
                "level: {}, last_compared_relation {:?}",
                self.level_count, self.last_compared_result
            )
        }
    }
}

impl<'a, T> NodeChain<'a, T> {
    pub fn new(tree: &'a RBTree<T>) -> Self {
        NodeChain {
            tree,
            level_count: 0,
            nodes: [NodePtr::null(); MAX_LABEL_COUNT as usize],
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

    pub fn top(&self) -> NodePtr<T> {
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
