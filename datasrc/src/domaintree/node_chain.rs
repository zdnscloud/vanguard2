use crate::domaintree::{node::NodePtr, tree::RBTree};
use r53::{name::NameComparisonResult, name::MAX_LABEL_COUNT, LabelSequence, Name, NameRelation};
use std::{fmt, marker::PhantomData};

pub struct NodeChain<'a, T: 'a> {
    pub level_count: usize,
    pub nodes: [NodePtr<T>; MAX_LABEL_COUNT as usize],
    pub last_compared: NodePtr<T>,
    pub last_compared_result: NameComparisonResult,
    phantom: PhantomData<&'a T>,
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
            )?;
            write!(f, " nodes:[")?;
            for n in &self.nodes[..self.level_count] {
                write!(f, "{},", n.get_name())?;
            }
            write!(f, " ]")?;
            Ok(())
        }
    }
}

impl<'a, T> NodeChain<'a, T> {
    pub fn new(_tree: &'a RBTree<T>) -> Self {
        NodeChain {
            level_count: 0,
            nodes: [NodePtr::null(); MAX_LABEL_COUNT as usize],
            last_compared: NodePtr::null(),
            last_compared_result: NameComparisonResult {
                order: 0,
                common_label_count: 0,
                relation: NameRelation::Equal,
            },
            phantom: PhantomData,
        }
    }

    pub fn get_absolute_name(&self, child: &LabelSequence) -> Name {
        child
            .concat_all(
                self.nodes[..self.level_count]
                    .iter()
                    .rev()
                    .map(|n| n.get_name())
                    .collect::<Vec<&LabelSequence>>()
                    .as_ref(),
            )
            .unwrap()
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
