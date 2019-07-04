use r53::{Name, NameRelation};
use std::cmp::Ord;
use std::cmp::Ordering;
use std::fmt::{self, Debug};
use std::iter::{FromIterator, IntoIterator};
use std::marker;
use std::mem;
use std::ops::Index;

use crate::domaintree::node::{
    connect_child, get_sibling, Color, NodePtr, RBTreeNode, COLOR_MASK, SUBTREE_ROOT_MASK,
};
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

fn indent(depth: usize) {
    const INDENT_FOR_EACH_DEPTH: usize = 5;
    print!("{}", " ".repeat((depth * INDENT_FOR_EACH_DEPTH) as usize));
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
        (**root).flag.clear_flag(COLOR_MASK);
    }

    pub fn insert(&mut self, target_: Name, v: T) -> Option<Option<T>> {
        let mut parent = NodePtr::null();
        let mut up = NodePtr::null();
        let mut current = self.root;
        let mut order = -1;
        let mut target = target_;

        while !current.is_null() {
            let compare_result = target.get_relation(current.get_name());
            match compare_result.relation {
                NameRelation::Equal => unsafe {
                    return Some(mem::replace(&mut (*current.0).value, Some(v)));
                },
                NameRelation::None => {
                    parent = current;
                    order = compare_result.order;
                    current = if order < 0 {
                        current.left()
                    } else {
                        current.right()
                    };
                }
                NameRelation::SubDomain => {
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
                    chain.push(node);
                    target = target
                        .strip_right((chain.last_compared_result.common_label_count - 1) as usize);
                    node = node.down();
                }
                _ => {
                    break;
                }
            }
        }
        result
    }

    pub fn remove_node(&mut self, mut node: NodePtr<T>) -> Option<T> {
        let old_value = node.set_value(None);

        if !node.down().is_null() {
            return old_value;
        }

        loop {
            let mut up = node.get_upper_node();
            if !node.left().is_null() && !node.right().is_null() {
                let mut right_most = node.left();
                while !right_most.right().is_null() {
                    right_most = right_most.right();
                }
                unsafe {
                    node.exchange(right_most, self.root.get_double_pointer());
                }
            }

            let mut child = NodePtr::null();
            if !node.right().is_null() {
                child = node.right();
            } else {
                child = node.left();
            }

            unsafe {
                connect_child(self.root.get_double_pointer(), node, node, child);
            }

            if !child.is_null() {
                child.set_parent(node.parent());
                if child.parent().is_null() || child.parent().down() == child {
                    child.use_flag(node.is_flag_set(SUBTREE_ROOT_MASK), SUBTREE_ROOT_MASK);
                }
            }

            if node.is_black() {
                if !child.is_null() && child.is_red() {
                    child.set_color(Color::Black);
                } else {
                    let current_root = if !up.is_null() {
                        up.get_double_pointer_of_down()
                    } else {
                        self.root.get_double_pointer()
                    };
                    unsafe {
                        self.remove_fixup(current_root, child, node.parent());
                    }
                }
            }

            self.len -= 1;

            if up.is_null() || up.get_value().is_some() || !up.down().is_null() {
                break;
            }

            node = up;
        }
        old_value
    }

    unsafe fn remove_fixup(
        &mut self,
        root: *mut *mut RBTreeNode<T>,
        mut child: NodePtr<T>,
        mut parent: NodePtr<T>,
    ) {
        while child.get_pointer() != *root && child.is_black() {
            if !parent.is_null() && parent.down().get_pointer() == *root {
                break;
            }

            let mut sibling = get_sibling(parent, child);
            if sibling.is_red() {
                parent.set_color(Color::Red);
                sibling.set_color(Color::Black);
                if parent.left() == child {
                    self.left_rotate(root, parent);
                } else {
                    self.right_rotate(root, parent);
                }
                sibling = get_sibling(parent, child);
            }
            if sibling.left().is_black() && sibling.right().is_black() {
                sibling.set_color(Color::Red);
                if parent.is_black() {
                    child = parent;
                    parent = parent.parent();
                    continue;
                } else {
                    parent.set_color(Color::Black);
                    break;
                }
            }
            let mut ss1 = sibling.left();
            let mut ss2 = sibling.right();
            if parent.left() != child {
                mem::swap(&mut ss1, &mut ss2);
            }
            if ss2.is_black() {
                sibling.set_color(Color::Red);
                ss1.set_color(Color::Black);
                if parent.left() == child {
                    self.right_rotate(root, sibling);
                } else {
                    self.left_rotate(root, sibling);
                }
                sibling = get_sibling(parent, child);
            }

            sibling.set_color(parent.get_color());
            parent.set_color(Color::Black);
            ss1 = sibling.left();
            ss2 = sibling.right();
            if parent.left() != child {
                mem::swap(&mut ss1, &mut ss2);
            }
            ss2.set_color(Color::Black);
            if parent.left() == child {
                self.left_rotate(root, parent);
            } else {
                self.right_rotate(root, parent);
            }
            break;
        }
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

    pub fn dump(&self, depth: usize) {
        indent(depth);
        println!("tree has {} node(s)", self.len);
        self.dump_helper(self.root, depth);
    }

    fn dump_helper(&self, node: NodePtr<T>, depth: usize) {
        if node.is_null() {
            indent(depth);
            println!("NULL");
            return;
        }
        indent(depth);

        let parent = node.parent();
        if !parent.is_null() {
            if parent.left() == node {
                print!("left>");
            } else {
                print!("right>");
            }
        }

        print!("{} ({:?})", node.get_name().to_string(), node.get_color());
        if node.get_value().is_none() {
            print!("[invisible]");
        }
        if node.is_flag_set(SUBTREE_ROOT_MASK) {
            print!(" [subtreeroot]");
        }
        print!("\n");

        let down = node.down();
        if !down.is_null() {
            indent(depth + 1);
            println!("begin down from {}\n", down.get_name().to_string());
            self.dump_helper(down, depth + 1);
            indent(depth + 1);
            println!("end down from {}", down.get_name().to_string());
        }
        self.dump_helper(node.left(), depth + 1);
        self.dump_helper(node.right(), depth + 1);
    }
}

mod tests {
    use super::{FindResultFlag, RBTree};
    use crate::domaintree::test_helper::name_from_string;
    use r53::Name;

    fn sample_names() -> Vec<(&'static str, i32)> {
        vec![
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
        ]
        .iter()
        .zip(0..)
        .map(|(&s, v)| (s, v))
        .collect()
    }

    fn build_tree(data: &Vec<(&'static str, i32)>) -> RBTree<i32> {
        let mut tree = RBTree::new();
        for (k, v) in data {
            tree.insert(name_from_string(k), *v);
        }
        tree
    }

    #[test]
    fn test_find() {
        let data = sample_names();
        let tree = build_tree(&data);
        assert_eq!(tree.len(), 13);

        for (n, v) in sample_names() {
            let result = tree.find_node(&name_from_string(n));
            assert_eq!(result.flag, FindResultFlag::ExacatMatch);
            assert_eq!(result.node.get_value(), &Some(v));
        }

        let none_terminal = vec!["d.e.f", "w.y.d.e.f"];
        for n in &none_terminal {
            let result = tree.find_node(&name_from_string(n));
            assert_eq!(result.flag, FindResultFlag::ExacatMatch);
            assert_eq!(result.node.get_value(), &None);
        }
    }

    #[test]
    fn test_delete() {
        let data = sample_names();
        let mut tree = build_tree(&data);
        assert_eq!(tree.len(), 13);
        for (n, v) in data {
            let mut result = tree.find_node(&name_from_string(n));
            assert_eq!(result.flag, FindResultFlag::ExacatMatch);
            assert_eq!(tree.remove_node(result.node), Some(v));
        }
        assert_eq!(tree.len(), 0);
    }
}
