use r53::{Name, NameRelation};
use std::cmp::Ord;
use std::cmp::Ordering;
use std::fmt::{self, Debug};
use std::iter::{FromIterator, IntoIterator};
use std::marker;
use std::mem;
use std::ops::Index;

use crate::domaintree::node::{Color, NodePtr, RBTreeNode, COLOR_MASK, SUBTREE_ROOT_MASK};
use crate::domaintree::node_chain::NodeChain;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum FindResultFlag {
    ExacatMatch,
    NotFound,
    PartialMatch,
}

pub struct FindResult<T> {
    pub node: NodePtr<T>,
    pub flag: FindResultFlag,
}

impl<T> FindResult<T> {
    fn new() -> Self {
        FindResult {
            node: NodePtr::null(),
            flag: FindResultFlag::NotFound,
        }
    }
}

pub struct RBTree<T> {
    root: NodePtr<T>,
    len: usize,
}

impl<T> Drop for RBTree<T> {
    fn drop(&mut self) {
        self.clear();
    }
}

impl<T: Clone> Clone for RBTree<T> {
    fn clone(&self) -> RBTree<T> {
        unsafe {
            let mut new = RBTree::new();
            new.root = self.root.deep_clone();
            new.len = self.len;
            new
        }
    }
}

impl<T> RBTree<T> {
    pub fn new() -> RBTree<T> {
        RBTree {
            root: NodePtr::null(),
            len: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.root.is_null()
    }

    unsafe fn left_rotate(&mut self, root: *mut *mut RBTreeNode<T>, mut node: NodePtr<T>) {
        let mut right = node.right();
        let mut rleft = right.left();
        node.set_right(rleft);
        if !rleft.is_null() {
            rleft.set_parent(node);
        }

        let mut parent = node.parent();
        right.set_parent(parent);
        if !node.is_flag_set(SUBTREE_ROOT_MASK) {
            right.clear_flag(SUBTREE_ROOT_MASK);
            if node == parent.left() {
                parent.set_left(right);
            } else {
                parent.set_right(right);
            }
        } else {
            right.set_flag(SUBTREE_ROOT_MASK);
            *root = right.get_pointer();
        }
        right.set_left(node);
        node.set_parent(right);
        node.clear_flag(SUBTREE_ROOT_MASK);
    }

    unsafe fn right_rotate(&mut self, root: *mut *mut RBTreeNode<T>, mut node: NodePtr<T>) {
        let mut left = node.left();
        let mut lright = left.right();
        node.set_left(lright);
        if !lright.is_null() {
            lright.set_parent(node);
        }

        let parent = node.parent();
        left.set_parent(parent);
        if !node.is_flag_set(SUBTREE_ROOT_MASK) {
            left.clear_flag(SUBTREE_ROOT_MASK);
            if node == parent.right() {
                parent.set_right(left);
            } else {
                parent.set_left(left);
            }
        } else {
            left.set_flag(SUBTREE_ROOT_MASK);
            *root = left.get_pointer();
        }
        left.set_right(node);
        node.set_parent(left);
        node.clear_flag(SUBTREE_ROOT_MASK);
    }

    unsafe fn insert_fixup(&mut self, root: *mut *mut RBTreeNode<T>, node_: NodePtr<T>) {
        let mut node = node_;
        while node.get_pointer() != *root {
            let mut parent = node.parent();
            if parent.is_black() {
                break;
            }

            let mut uncle = node.uncle();
            let mut grand_parent = node.grand_parent();
            if !uncle.is_null() && uncle.is_red() {
                parent.set_color(Color::Black);
                uncle.set_color(Color::Black);
                grand_parent.set_color(Color::Red);
                node = grand_parent;
            } else {
                if node == parent.right() && parent == grand_parent.left() {
                    node = parent;
                    self.left_rotate(root, parent);
                } else if node == parent.left() && parent == grand_parent.right() {
                    node = parent;
                    self.right_rotate(root, parent);
                }
                parent = node.parent();
                parent.set_color(Color::Black);
                grand_parent.set_color(Color::Red);
                if node == parent.left() {
                    self.right_rotate(root, grand_parent);
                } else {
                    self.left_rotate(root, grand_parent);
                }
                break;
            }
        }
        (**root).flag.set_flag(COLOR_MASK);
    }

    pub fn insert(&mut self, target_: Name, v: T) -> Option<Option<T>> {
        let mut parent = NodePtr::null();
        let mut up = NodePtr::null();
        let mut current = self.root;
        let mut order = -1;
        let mut target = target_;

        println!("insert {}", target);
        while !current.is_null() {
            let compare_result = target.get_relation(current.get_name());
            match compare_result.relation {
                NameRelation::Equal => unsafe {
                    return Some(mem::replace(&mut (*current.0).value, Some(v)));
                },
                NameRelation::None => {
                    println!("into same tree");
                    parent = current;
                    order = compare_result.order;
                    current = if order < 0 {
                        current.left()
                    } else {
                        current.right()
                    };
                }
                NameRelation::SubDomain => {
                    println!("into sub domain");
                    parent = NodePtr::null();
                    up = current;
                    target = target.strip_right((compare_result.common_label_count - 1) as usize);
                    current = current.down();
                }
                _ => {
                    let common_ancestor = target.strip_left(
                        (target.label_count - compare_result.common_label_count) as usize,
                    );
                    let new_name = current
                        .get_name()
                        .strip_right((compare_result.common_label_count - 1) as usize);
                    println!(
                        "common {}, new {}",
                        common_ancestor.to_string(),
                        new_name.to_string()
                    );
                    unsafe {
                        self.node_fission(&mut current, new_name, common_ancestor);
                    }
                    current = current.parent();
                }
            }
        }

        let mut current_root = if !up.is_null() {
            up.get_double_pointer_of_down()
        } else {
            self.root.get_double_pointer()
        };
        self.len += 1;
        let mut node = NodePtr::new(target, Some(v));
        node.set_parent(parent);
        if parent.is_null() {
            unsafe {
                *current_root = node.get_pointer();
            }
            node.set_color(Color::Black);
            node.set_flag(SUBTREE_ROOT_MASK);
            node.set_parent(up);
        } else if order < 0 {
            node.clear_flag(SUBTREE_ROOT_MASK);
            parent.set_left(node);
            unsafe {
                self.insert_fixup(current_root, node);
            }
        } else {
            node.clear_flag(SUBTREE_ROOT_MASK);
            parent.set_right(node);
            unsafe {
                self.insert_fixup(current_root, node);
            }
        }
        None
    }

    unsafe fn node_fission(&mut self, node: &mut NodePtr<T>, new_prefix: Name, new_suffix: Name) {
        println!(
            "do fission with old name: {}, new name {}, common {}",
            node.get_name(),
            new_prefix,
            new_suffix
        );
        let mut up = NodePtr::new(new_suffix, None);
        node.set_name(new_prefix);
        up.set_parent(node.parent());
        connect_child(self.root.get_double_pointer(), *node, *node, up);
        up.set_down(*node);
        node.set_parent(up);
        up.set_left(node.left());
        if !node.left().is_null() {
            node.left().set_parent(up);
        }
        up.set_right(node.right());
        if !node.right().is_null() {
            node.right().set_parent(up);
        }
        node.set_left(NodePtr::null());
        node.set_right(NodePtr::null());
        up.set_color(node.get_color());
        node.set_color(Color::Black);
        if node.is_flag_set(SUBTREE_ROOT_MASK) {
            up.set_flag(SUBTREE_ROOT_MASK);
        } else {
            up.clear_flag(SUBTREE_ROOT_MASK);
        }
        node.set_flag(SUBTREE_ROOT_MASK);
        self.len += 1;
    }

    pub fn find_node(&self, target_: &Name) -> FindResult<T> {
        let mut node = self.root;
        let mut chain = NodeChain::new();
        let mut result = FindResult::new();
        let mut target = target_.clone();
        while !node.is_null() {
            chain.last_compared = node;
            chain.last_compared_result = target.get_relation(node.get_name());
            match chain.last_compared_result.relation {
                NameRelation::Equal => {
                    chain.push(node);
                    result.flag = FindResultFlag::ExacatMatch;
                    result.node = node;
                    break;
                }
                NameRelation::None => {
                    if chain.last_compared_result.order < 0 {
                        node = node.left();
                    } else {
                        node = node.right();
                    }
                }
                NameRelation::SubDomain => {
                    result.flag = FindResultFlag::PartialMatch;
                    result.node = node;
                    chain.push(node);
                    target =
                        target.strip_right(chain.last_compared_result.common_label_count as usize);
                    node = node.down();
                }
                _ => {
                    break;
                }
            }
        }
        result
    }

    fn clear_recurse(&mut self, current: NodePtr<T>) {
        if !current.is_null() {
            unsafe {
                self.clear_recurse(current.left());
                self.clear_recurse(current.right());
                self.clear_recurse(current.down());
                Box::from_raw(current.0);
            }
        }
    }

    pub fn clear(&mut self) {
        let root = self.root;
        self.root = NodePtr::null();
        self.clear_recurse(root);
    }
}

unsafe fn connect_child<T>(
    root: *mut *mut RBTreeNode<T>,
    current: NodePtr<T>,
    old: NodePtr<T>,
    new: NodePtr<T>,
) {
    let mut parent = current.parent();
    if parent.is_null() {
        *root = new.get_pointer();
    } else {
        if parent.left() == old {
            parent.set_left(new)
        } else if parent.right() == old {
            parent.set_right(new);
        } else {
            parent.set_down(new);
        }
    }
}

mod tests {
    use super::{FindResultFlag, RBTree};
    use crate::domaintree::test_helper::name_from_string;
    use r53::Name;

    fn build_tree() -> RBTree<i32> {
        let names = vec![
            "c",
            "b",
            "a",
            "x.d.e.f",
            "z.d.e.f",
            "g.h",
            "i.g.h",
            "o.w.y.d.e.f",
            "j.z.d.e.f",
            "p.w.y.d.e.f",
            "q.w.y.d.e.f",
        ];
        let mut tree = RBTree::new();
        let mut value = 0;
        for k in &names {
            tree.insert(name_from_string(k), value);
            value += 1;
        }
        tree
    }

    #[test]
    fn test_find() {
        let tree = build_tree();
        assert_eq!(tree.len(), 13);
        let result = tree.find_node(&name_from_string("c"));
        assert_eq!(result.flag, FindResultFlag::ExacatMatch);
        assert_eq!(result.node.get_value(), &Some(0));
    }
}
