use r53::Name;
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

pub struct RBTreeNode<T> {
    pub flag: NodeFlag,
    pub left: NodePtr<T>,
    pub right: NodePtr<T>,
    pub parent: NodePtr<T>,
    pub down: NodePtr<T>,
    pub name: Name,
    pub value: T,
}

impl<T> RBTreeNode<T> {
    pub fn pair(self) -> (Name, T) {
        (self.name, self.value)
    }
}

impl<T: Debug> Debug for RBTreeNode<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "k:{:?} v:{:?} c:{:?}",
            self.name,
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
pub struct NodePtr<T>(pub *mut RBTreeNode<T>);

impl<T> Clone for NodePtr<T> {
    fn clone(&self) -> NodePtr<T> {
        NodePtr(self.0)
    }
}

impl<T> Copy for NodePtr<T> {}

impl<T> Ord for NodePtr<T> {
    fn cmp(&self, other: &NodePtr<T>) -> Ordering {
        unsafe { (*self.0).name.cmp(&(*other.0).name) }
    }
}

impl<T> PartialOrd for NodePtr<T> {
    fn partial_cmp(&self, other: &NodePtr<T>) -> Option<Ordering> {
        unsafe { Some((*self.0).name.cmp(&(*other.0).name)) }
    }
}

impl<T> PartialEq for NodePtr<T> {
    fn eq(&self, other: &NodePtr<T>) -> bool {
        self.0 == other.0
    }
}

impl<T> Eq for NodePtr<T> {}

impl<T> NodePtr<T> {
    pub fn new(name: Name, v: T) -> NodePtr<T> {
        let node = RBTreeNode {
            flag: NodeFlag::default(),
            left: NodePtr::null(),
            right: NodePtr::null(),
            parent: NodePtr::null(),
            down: NodePtr::null(),
            name: name,
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

    pub fn get_key(&self) -> &Name {
        unsafe { &(*self.0).name }
    }

    pub fn get_value(&self) -> &T {
        unsafe { &(*self.0).value }
    }

    pub fn set_value(&mut self, v: T) -> T {
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

    pub fn min_node(self) -> NodePtr<T> {
        let mut node = self;
        while !node.left().is_null() {
            node = node.left();
        }
        node
    }

    pub fn max_node(self) -> NodePtr<T> {
        let mut node = self;
        while !node.right().is_null() {
            node = node.right();
        }
        node
    }

    pub fn next(self) -> NodePtr<T> {
        if !self.right().is_null() {
            return self.right().min_node();
        }

        let mut node = self;
        let mut parent = self.parent();
        while !node.is_flag_set(SUBTREE_ROOT_MASK) && node == parent.right() {
            node = parent;
            parent = parent.parent();
        }
        if !node.is_flag_set(SUBTREE_ROOT_MASK) {
            parent
        } else {
            NodePtr::null()
        }
    }

    pub fn prev(self) -> NodePtr<T> {
        if !self.left().is_null() {
            return self.left().max_node();
        }

        let mut node = self;
        let mut parent = self.parent();
        while !node.is_flag_set(SUBTREE_ROOT_MASK) && node == parent.left() {
            node = parent;
            parent = parent.parent();
        }
        if !node.is_flag_set(SUBTREE_ROOT_MASK) {
            parent
        } else {
            NodePtr::null()
        }
    }

    pub fn set_parent(&mut self, parent: NodePtr<T>) {
        unsafe { (*self.0).parent = parent }
    }

    pub fn set_left(&mut self, left: NodePtr<T>) {
        unsafe { (*self.0).left = left }
    }

    pub fn set_right(&mut self, right: NodePtr<T>) {
        unsafe { (*self.0).right = right }
    }

    pub fn parent(self) -> NodePtr<T> {
        unsafe { (*self.0).parent }
    }

    pub fn grand_parent(self) -> NodePtr<T> {
        let parent = self.parent();
        if parent.is_null() {
            NodePtr::null()
        } else {
            parent.parent()
        }
    }

    pub fn sibling(parent: NodePtr<T>, child: NodePtr<T>) -> NodePtr<T> {
        if parent.is_null() {
            NodePtr::null()
        } else if parent.left() == child {
            parent.right()
        } else {
            parent.left()
        }
    }

    pub fn uncle(self) -> NodePtr<T> {
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

    pub fn left(self) -> NodePtr<T> {
        unsafe { (*self.0).left }
    }

    pub fn right(self) -> NodePtr<T> {
        unsafe { (*self.0).right }
    }

    pub fn null() -> NodePtr<T> {
        NodePtr(ptr::null_mut())
    }

    pub fn subtree_root(self) -> NodePtr<T> {
        let mut node = self;
        while node.is_flag_set(SUBTREE_ROOT_MASK) == false {
            node = node.parent();
        }
        node
    }

    pub fn get_upper_node(self) -> NodePtr<T> {
        self.subtree_root().parent()
    }

    pub fn is_null(self) -> bool {
        self.0.is_null()
    }
}

impl<T: Clone> NodePtr<T> {
    pub unsafe fn deep_clone(self) -> NodePtr<T> {
        let mut node = NodePtr::new((*self.0).name.clone(), (*self.0).value.clone());
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

mod tests {
    use super::NodePtr;

    #[test]
    fn test_set_value() {
        let mut n = NodePtr::new("k1", Some("v1"));
        assert_eq!(n.get_value(), &Some("v1"));
        let old = n.set_value(Some("v2"));
        assert_eq!(old, Some("v1"));
        assert_eq!(n.get_value(), &Some("v2"));

        let old = n.set_value(None);
        assert_eq!(old, Some("v2"));
        assert_eq!(n.get_value(), &None);
    }
}
