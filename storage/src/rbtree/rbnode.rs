use std::cmp::Ord;
use std::cmp::Ordering;
use std::fmt::{self, Debug};
use std::ptr;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Color {
    Red,
    Black,
}

pub struct RBTreeNode<K: Ord, V> {
    pub color: Color,
    pub left: NodePtr<K, V>,
    pub right: NodePtr<K, V>,
    pub parent: NodePtr<K, V>,
    pub key: K,
    pub value: V,
}

impl<K: Ord, V> RBTreeNode<K, V> {
    pub fn pair(self) -> (K, V) {
        (self.key, self.value)
    }
}

impl<K, V> Debug for RBTreeNode<K, V>
where
    K: Ord + Debug,
    V: Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "k:{:?} v:{:?} c:{:?}", self.key, self.value, self.color)
    }
}

#[derive(Debug)]
pub struct NodePtr<K: Ord, V>(pub *mut RBTreeNode<K, V>);

impl<K: Ord, V> Clone for NodePtr<K, V> {
    fn clone(&self) -> NodePtr<K, V> {
        NodePtr(self.0)
    }
}

impl<K: Ord, V> Copy for NodePtr<K, V> {}

impl<K: Ord, V> Ord for NodePtr<K, V> {
    fn cmp(&self, other: &NodePtr<K, V>) -> Ordering {
        unsafe { (*self.0).key.cmp(&(*other.0).key) }
    }
}

impl<K: Ord, V> PartialOrd for NodePtr<K, V> {
    fn partial_cmp(&self, other: &NodePtr<K, V>) -> Option<Ordering> {
        unsafe { Some((*self.0).key.cmp(&(*other.0).key)) }
    }
}

impl<K: Ord, V> PartialEq for NodePtr<K, V> {
    fn eq(&self, other: &NodePtr<K, V>) -> bool {
        self.0 == other.0
    }
}

impl<K: Ord, V> Eq for NodePtr<K, V> {}

impl<K: Ord, V> NodePtr<K, V> {
    pub fn new(k: K, v: V) -> NodePtr<K, V> {
        let node = RBTreeNode {
            color: Color::Black,
            left: NodePtr::null(),
            right: NodePtr::null(),
            parent: NodePtr::null(),
            key: k,
            value: v,
        };
        NodePtr(Box::into_raw(Box::new(node)))
    }

    pub fn set_color(&mut self, color: Color) {
        if self.is_null() {
            return;
        }
        unsafe {
            (*self.0).color = color;
        }
    }

    pub fn set_red_color(&mut self) {
        self.set_color(Color::Red);
    }

    pub fn set_black_color(&mut self) {
        self.set_color(Color::Black);
    }

    pub fn get_color(self) -> Color {
        if self.is_null() {
            return Color::Black;
        }
        unsafe { (*self.0).color }
    }

    pub fn get_key(&self) -> &K {
        unsafe { &(*self.0).key }
    }

    pub fn get_value(&self) -> &V {
        unsafe { &(*self.0).value }
    }

    pub fn is_red_color(self) -> bool {
        self.get_color() == Color::Red
    }

    pub fn is_black_color(self) -> bool {
        self.get_color() == Color::Black
    }

    pub fn is_left_child(self) -> bool {
        self.parent().left() == self
    }

    pub fn is_right_child(self) -> bool {
        self.parent().right() == self
    }

    pub fn min_node(self) -> NodePtr<K, V> {
        let mut node = self;
        while !node.left().is_null() {
            node = node.left();
        }
        node
    }

    pub fn max_node(self) -> NodePtr<K, V> {
        let mut node = self;
        while !node.right().is_null() {
            node = node.right();
        }
        node
    }

    pub fn next(self) -> NodePtr<K, V> {
        if !self.right().is_null() {
            self.right().min_node()
        } else {
            let mut node = self;
            loop {
                if node.parent().is_null() {
                    return NodePtr::null();
                }
                if node.is_left_child() {
                    return node.parent();
                }
                node = node.parent();
            }
        }
    }

    pub fn prev(self) -> NodePtr<K, V> {
        if !self.left().is_null() {
            self.left().max_node()
        } else {
            let mut node = self;
            loop {
                if node.parent().is_null() {
                    return NodePtr::null();
                }
                if node.is_right_child() {
                    return node.parent();
                }
                node = node.parent();
            }
        }
    }

    pub fn set_parent(&mut self, parent: NodePtr<K, V>) {
        unsafe { (*self.0).parent = parent }
    }

    pub fn set_left(&mut self, left: NodePtr<K, V>) {
        unsafe { (*self.0).left = left }
    }

    pub fn set_right(&mut self, right: NodePtr<K, V>) {
        unsafe { (*self.0).right = right }
    }

    pub fn parent(self) -> NodePtr<K, V> {
        unsafe { (*self.0).parent }
    }

    pub fn grand_parent(self) -> NodePtr<K, V> {
        let parent = self.parent();
        if parent.is_null() {
            NodePtr::null()
        } else {
            parent.parent()
        }
    }

    pub fn sibling(parent: NodePtr<K, V>, child: NodePtr<K, V>) -> NodePtr<K, V> {
        if parent.is_null() {
            NodePtr::null()
        } else if parent.left() == child {
            parent.right()
        } else {
            parent.left()
        }
    }

    pub fn uncle(self) -> NodePtr<K, V> {
        let grand_parent = self.grand_parent();
        if grand_parent.is_null() {
            return NodePtr::null();
        }

        if self.parent() == grand_parent.left() {
            grand_parent.right()
        } else {
            grand_parent.left()
        }
    }

    pub fn left(self) -> NodePtr<K, V> {
        unsafe { (*self.0).left }
    }

    pub fn right(self) -> NodePtr<K, V> {
        unsafe { (*self.0).right }
    }

    pub fn null() -> NodePtr<K, V> {
        NodePtr(ptr::null_mut())
    }

    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl<K: Ord + Clone, V: Clone> NodePtr<K, V> {
    pub unsafe fn deep_clone(self) -> NodePtr<K, V> {
        let mut node = NodePtr::new((*self.0).key.clone(), (*self.0).value.clone());
        if !self.left().is_null() {
            node.set_left(self.left().deep_clone());
            node.left().set_parent(node);
        }
        if !self.right().is_null() {
            node.set_right(self.right().deep_clone());
            node.right().set_parent(node);
        }
        node
    }
}
