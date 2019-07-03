use std::cmp::Ord;
use std::cmp::Ordering;
use std::fmt::{self, Debug};
use std::mem::swap;
use std::ptr;

const COLOR_MASK: u16 = 0x0001;
const CALLBACK_MASK: u16 = 0x0002;
const SUBTREE_ROOT_MASK: u16 = 0x0004;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum Color {
    Red,
    Black,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct NodeFlag(u16);
impl NodeFlag {
    pub fn set_flag(&mut self, mask: u16) {
        self.0 |= COLOR_MASK
    }

    pub fn clear_flag(&mut self, mask: u16) {
        self.0 &= !mask
    }

    pub fn is_flag_set(self, mask: u16) -> bool {
        (self.0 & mask) != 0
    }
}

impl Default for NodeFlag {
    fn default() -> Self {
        NodeFlag(COLOR_MASK)
    }
}

pub struct RBTreeNode<K: Ord, V> {
    pub flag: NodeFlag,
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
        write!(
            f,
            "k:{:?} v:{:?} c:{:?}",
            self.key,
            self.value,
            if self.flag.is_flag_set(COLOR_MASK) {
                Color::Black
            } else {
                Color::Red
            }
        )
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
            flag: NodeFlag::default(),
            left: NodePtr::null(),
            right: NodePtr::null(),
            parent: NodePtr::null(),
            key: k,
            value: v,
        };
        NodePtr(Box::into_raw(Box::new(node)))
    }

    pub fn set_color(&mut self, color: Color) {
        match color {
            Color::Black => self.set_flag(COLOR_MASK),
            Color::Red => self.clear_flag(COLOR_MASK),
        }
    }

    pub fn get_color(self) -> Color {
        if self.is_null() {
            return Color::Black;
        }

        if self.is_flag_set(COLOR_MASK) {
            Color::Black
        } else {
            Color::Red
        }
    }

    pub fn set_flag(&mut self, mask: u16) {
        if self.is_null() {
            return;
        }
        unsafe {
            (*self.0).flag.set_flag(mask);
        }
    }

    pub fn clear_flag(&mut self, mask: u16) {
        if self.is_null() {
            return;
        }
        unsafe {
            (*self.0).flag.clear_flag(mask);
        }
    }

    pub fn is_flag_set(self, mask: u16) -> bool {
        unsafe { (*self.0).flag.is_flag_set(mask) }
    }

    pub fn get_key(&self) -> &K {
        unsafe { &(*self.0).key }
    }

    pub fn get_value(&self) -> &V {
        unsafe { &(*self.0).value }
    }

    pub fn set_value(&mut self, v: V) -> V {
        let mut back = v;
        unsafe {
            swap(&mut (*self.0).value, &mut back);
        }
        back
    }

    pub fn is_red(self) -> bool {
        self.get_color() == Color::Red
    }

    pub fn is_black(self) -> bool {
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
