use r53::Name;
use std::cmp::Ord;
use std::cmp::Ordering;
use std::fmt::{self, Debug};
use std::iter::{FromIterator, IntoIterator};
use std::marker;
use std::mem;
use std::ops::Index;

use crate::domaintree::node::{Color, NodePtr};

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

impl<T: Debug> Debug for RBTree<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_map().entries(self.iter()).finish()
    }
}

impl<T: PartialEq> PartialEq for RBTree<T> {
    fn eq(&self, other: &RBTree<T>) -> bool {
        if self.len() != other.len() {
            return false;
        }

        self.iter()
            .all(|(name, value)| other.get(name).map_or(false, |v| *value == *v))
    }
}

impl<T: Eq> Eq for RBTree<T> {}

impl<'a, T> Index<&'a Name> for RBTree<T> {
    type Output = T;

    fn index(&self, index: &Name) -> &T {
        self.get(index).expect("no entry found for name")
    }
}

impl<T> FromIterator<(Name, T)> for RBTree<T> {
    fn from_iter<I: IntoIterator<Item = (Name, T)>>(iter: I) -> RBTree<T> {
        let mut tree = RBTree::new();
        tree.extend(iter);
        tree
    }
}

impl<T> Extend<(Name, T)> for RBTree<T> {
    fn extend<I: IntoIterator<Item = (Name, T)>>(&mut self, iter: I) {
        let iter = iter.into_iter();
        for (k, v) in iter {
            self.insert(k, v);
        }
    }
}

pub struct Keys<'a, T: 'a> {
    inner: Iter<'a, T>,
}

impl<'a, T> Clone for Keys<'a, T> {
    fn clone(&self) -> Keys<'a, T> {
        Keys {
            inner: self.inner.clone(),
        }
    }
}

impl<'a, T> fmt::Debug for Keys<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl<'a, T> Iterator for Keys<'a, T> {
    type Item = &'a Name;

    fn next(&mut self) -> Option<(&'a Name)> {
        self.inner.next().map(|(k, _)| k)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

pub struct Values<'a, T: 'a> {
    inner: Iter<'a, T>,
}

impl<'a, T> Clone for Values<'a, T> {
    fn clone(&self) -> Values<'a, T> {
        Values {
            inner: self.inner.clone(),
        }
    }
}

impl<'a, T: Debug> fmt::Debug for Values<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl<'a, T> Iterator for Values<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<(&'a T)> {
        self.inner.next().map(|(_, v)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

pub struct ValuesMut<'a, T: 'a> {
    inner: IterMut<'a, T>,
}

impl<'a, T> Clone for ValuesMut<'a, T> {
    fn clone(&self) -> ValuesMut<'a, T> {
        ValuesMut {
            inner: self.inner.clone(),
        }
    }
}

impl<'a, T: Debug> fmt::Debug for ValuesMut<'a, T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_list().entries(self.clone()).finish()
    }
}

impl<'a, T> Iterator for ValuesMut<'a, T> {
    type Item = &'a mut T;

    fn next(&mut self) -> Option<(&'a mut T)> {
        self.inner.next().map(|(_, v)| v)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        self.inner.size_hint()
    }
}

pub struct IntoIter<T> {
    head: NodePtr<T>,
    tail: NodePtr<T>,
    len: usize,
}

impl<T> Drop for IntoIter<T> {
    fn drop(&mut self) {
        for (_, _) in self {}
    }
}

impl<T> Iterator for IntoIter<T> {
    type Item = (Name, T);

    fn next(&mut self) -> Option<(Name, T)> {
        if self.len == 0 {
            return None;
        }

        if self.head.is_null() {
            return None;
        }

        let next = self.head.next();
        let obj = unsafe { Box::from_raw(self.head.0) };
        let (k, v) = obj.pair();
        self.head = next;
        self.len -= 1;
        Some((k, v))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<T> DoubleEndedIterator for IntoIter<T> {
    fn next_back(&mut self) -> Option<(Name, T)> {
        if self.len == 0 {
            return None;
        }

        if self.tail.is_null() {
            return None;
        }

        let prev = self.tail.prev();
        let obj = unsafe { Box::from_raw(self.tail.0) };
        let (k, v) = obj.pair();
        self.tail = prev;
        self.len -= 1;
        Some((k, v))
    }
}

pub struct Iter<'a, T: 'a> {
    head: NodePtr<T>,
    tail: NodePtr<T>,
    len: usize,
    _marker: marker::PhantomData<&'a ()>,
}

impl<'a, T: 'a> Clone for Iter<'a, T> {
    fn clone(&self) -> Iter<'a, T> {
        Iter {
            head: self.head,
            tail: self.tail,
            len: self.len,
            _marker: self._marker,
        }
    }
}

impl<'a, T: 'a> Iterator for Iter<'a, T> {
    type Item = (&'a Name, &'a T);

    fn next(&mut self) -> Option<(&'a Name, &'a T)> {
        if self.len == 0 {
            return None;
        }

        if self.head.is_null() {
            return None;
        }

        let (k, v) = unsafe { (&(*self.head.0).name, &(*self.head.0).value) };
        self.head = self.head.next();
        self.len -= 1;
        Some((k, v))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, T: 'a> DoubleEndedIterator for Iter<'a, T> {
    fn next_back(&mut self) -> Option<(&'a Name, &'a T)> {
        if self.len == 0 {
            return None;
        }

        if self.tail == self.head {
            return None;
        }

        let (k, v) = unsafe { (&(*self.tail.0).name, &(*self.tail.0).value) };
        self.tail = self.tail.prev();
        self.len -= 1;
        Some((k, v))
    }
}

pub struct IterMut<'a, T: 'a> {
    head: NodePtr<T>,
    tail: NodePtr<T>,
    len: usize,
    _marker: marker::PhantomData<&'a ()>,
}

impl<'a, T: 'a> Clone for IterMut<'a, T> {
    fn clone(&self) -> IterMut<'a, T> {
        IterMut {
            head: self.head,
            tail: self.tail,
            len: self.len,
            _marker: self._marker,
        }
    }
}

impl<'a, T: 'a> Iterator for IterMut<'a, T> {
    type Item = (&'a Name, &'a mut T);

    fn next(&mut self) -> Option<(&'a Name, &'a mut T)> {
        if self.len == 0 {
            return None;
        }

        if self.head.is_null() {
            return None;
        }

        let (k, v) = unsafe { (&(*self.head.0).name, &mut (*self.head.0).value) };
        self.head = self.head.next();
        self.len -= 1;
        Some((k, v))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.len, Some(self.len))
    }
}

impl<'a, T: 'a> DoubleEndedIterator for IterMut<'a, T> {
    fn next_back(&mut self) -> Option<(&'a Name, &'a mut T)> {
        if self.len == 0 {
            return None;
        }

        if self.tail == self.head {
            return None;
        }

        let (k, v) = unsafe { (&(*self.tail.0).name, &mut (*self.tail.0).value) };
        self.tail = self.tail.prev();
        self.len -= 1;
        Some((k, v))
    }
}

impl<T> IntoIterator for RBTree<T> {
    type Item = (Name, T);
    type IntoIter = IntoIter<T>;

    fn into_iter(mut self) -> IntoIter<T> {
        let iter = if self.root.is_null() {
            IntoIter {
                head: NodePtr::null(),
                tail: NodePtr::null(),
                len: self.len,
            }
        } else {
            IntoIter {
                head: self.first_child(),
                tail: self.last_child(),
                len: self.len,
            }
        };
        self.fast_clear();
        iter
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

    unsafe fn left_rotate(&mut self, mut node: NodePtr<T>) {
        let mut right = node.right();
        let mut rleft = right.left();
        node.set_right(rleft);
        if !rleft.is_null() {
            rleft.set_parent(node);
        }

        right.set_parent(node.parent());
        if node == self.root {
            self.root = right;
        } else if node == node.parent().left() {
            node.parent().set_left(right);
        } else {
            node.parent().set_right(right);
        }
        right.set_left(node);
        node.set_parent(right);
    }

    unsafe fn right_rotate(&mut self, mut node: NodePtr<T>) {
        let mut left = node.left();
        let mut lright = left.right();
        node.set_left(lright);
        if !lright.is_null() {
            lright.set_parent(node);
        }

        left.set_parent(node.parent());
        if node == self.root {
            self.root = left;
        } else if node == node.parent().right() {
            node.parent().set_right(left);
        } else {
            node.parent().set_left(left);
        }
        left.set_right(node);
        node.set_parent(left);
    }

    unsafe fn insert_fixup(&mut self, mut node: NodePtr<T>) {
        while node != self.root {
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
                    self.left_rotate(parent);
                } else if node == parent.left() && parent == grand_parent.right() {
                    node = parent;
                    self.right_rotate(parent);
                }
                parent = node.parent();
                parent.set_color(Color::Black);
                grand_parent.set_color(Color::Red);
                if node == parent.left() {
                    self.right_rotate(grand_parent);
                } else {
                    self.left_rotate(grand_parent);
                }
                break;
            }
        }
        self.root.set_color(Color::Black);
    }

    pub fn insert(&mut self, k: Name, v: T) -> Option<T> {
        let mut y = NodePtr::null();
        let mut x = self.root;

        while !x.is_null() {
            y = x;
            match k.cmp(x.get_key()) {
                Ordering::Less => {
                    x = x.left();
                }
                Ordering::Equal => unsafe {
                    return Some(mem::replace(&mut (*x.0).value, v));
                },
                Ordering::Greater => {
                    x = x.right();
                }
            };
        }

        self.len += 1;
        let mut node = NodePtr::new(k, v);
        node.set_parent(y);

        if y.is_null() {
            self.root = node;
        } else {
            match node.cmp(&&mut y) {
                Ordering::Less => {
                    y.set_left(node);
                }
                _ => {
                    y.set_right(node);
                }
            };
        }

        node.set_color(Color::Red);
        unsafe {
            self.insert_fixup(node);
        }
        None
    }

    pub fn find_node(&self, k: &Name) -> NodePtr<T> {
        let mut current = self.root;
        unsafe {
            loop {
                if current.is_null() {
                    break;
                }
                let next = match k.cmp(&(*current.0).name) {
                    Ordering::Less => (*current.0).left,
                    Ordering::Greater => (*current.0).right,
                    Ordering::Equal => return current,
                };
                current = next;
            }
        }
        NodePtr::null()
    }

    pub fn find_less_equal(&self, k: &Name) -> (NodePtr<T>, bool) {
        let mut less = NodePtr::null();
        let mut current = self.root;
        unsafe {
            loop {
                if current.is_null() {
                    break;
                }
                let next = match k.cmp(&(*current.0).name) {
                    Ordering::Less => (*current.0).left,
                    Ordering::Greater => {
                        less = current;
                        (*current.0).right
                    }
                    Ordering::Equal => return (current, true),
                };
                current = next;
            }
        }
        (less, false)
    }

    fn first_child(&self) -> NodePtr<T> {
        if self.root.is_null() {
            NodePtr::null()
        } else {
            let mut temp = self.root;
            while !temp.left().is_null() {
                temp = temp.left();
            }
            temp
        }
    }

    fn last_child(&self) -> NodePtr<T> {
        if self.root.is_null() {
            NodePtr::null()
        } else {
            let mut temp = self.root;
            while !temp.right().is_null() {
                temp = temp.right();
            }
            temp
        }
    }

    pub fn get_first(&self) -> Option<(&Name, &T)> {
        let first = self.first_child();
        if first.is_null() {
            return None;
        }
        unsafe { Some((&(*first.0).name, &(*first.0).value)) }
    }

    pub fn get_last(&self) -> Option<(&Name, &T)> {
        let last = self.last_child();
        if last.is_null() {
            return None;
        }
        unsafe { Some((&(*last.0).name, &(*last.0).value)) }
    }

    pub fn pop_first(&mut self) -> Option<(Name, T)> {
        let first = self.first_child();
        if first.is_null() {
            return None;
        }
        unsafe { Some(self.delete(first)) }
    }

    pub fn pop_last(&mut self) -> Option<(Name, T)> {
        let last = self.last_child();
        if last.is_null() {
            return None;
        }
        unsafe { Some(self.delete(last)) }
    }

    pub fn get_first_mut(&mut self) -> Option<(&Name, &mut T)> {
        let first = self.first_child();
        if first.is_null() {
            return None;
        }
        unsafe { Some((&(*first.0).name, &mut (*first.0).value)) }
    }

    pub fn get_last_mut(&mut self) -> Option<(&Name, &mut T)> {
        let last = self.last_child();
        if last.is_null() {
            return None;
        }
        unsafe { Some((&(*last.0).name, &mut (*last.0).value)) }
    }

    pub fn get(&self, k: &Name) -> Option<&T> {
        let node = self.find_node(k);
        if node.is_null() {
            return None;
        }

        unsafe { Some(&(*node.0).value) }
    }

    pub fn get_mut(&mut self, k: &Name) -> Option<&mut T> {
        let node = self.find_node(k);
        if node.is_null() {
            return None;
        }

        unsafe { Some(&mut (*node.0).value) }
    }

    pub fn contains_key(&self, k: &Name) -> bool {
        let node = self.find_node(k);
        if node.is_null() {
            return false;
        }
        true
    }

    fn clear_recurse(&mut self, current: NodePtr<T>) {
        if !current.is_null() {
            unsafe {
                self.clear_recurse(current.left());
                self.clear_recurse(current.right());
                Box::from_raw(current.0);
            }
        }
    }

    pub fn clear(&mut self) {
        let root = self.root;
        self.root = NodePtr::null();
        self.clear_recurse(root);
    }

    fn fast_clear(&mut self) {
        self.root = NodePtr::null();
    }

    pub fn remove(&mut self, k: &Name) -> Option<T> {
        let node = self.find_node(k);
        if node.is_null() {
            return None;
        }
        unsafe { Some(self.delete(node).1) }
    }

    unsafe fn delete_fixup(&mut self, mut node: NodePtr<T>, mut parent: NodePtr<T>) {
        while node != self.root && node.is_black() {
            let mut sibling = NodePtr::sibling(parent, node);
            let is_right_sibling = parent.left() == node;
            if sibling.is_red() {
                sibling.set_color(Color::Black);
                parent.set_color(Color::Red);
                if is_right_sibling {
                    self.left_rotate(parent);
                    sibling = parent.right();
                } else {
                    self.right_rotate(parent);
                    sibling = parent.left();
                }
            }

            let mut sibleft = sibling.left();
            let mut sibright = sibling.right();
            if sibleft.is_black() && sibright.is_black() {
                sibling.set_color(Color::Red);
                node = parent;
                parent = node.parent();
            } else {
                if is_right_sibling {
                    if sibright.is_black() {
                        sibleft.set_color(Color::Black);
                        sibling.set_color(Color::Red);
                        self.right_rotate(sibling);
                        sibling = parent.right();
                    }
                } else if sibleft.is_black() {
                    sibright.set_color(Color::Black);
                    sibling.set_color(Color::Red);
                    self.left_rotate(sibling);
                    sibling = parent.left();
                }
                sibling.set_color(parent.get_color());
                parent.set_color(Color::Black);
                if is_right_sibling {
                    sibling.right().set_color(Color::Black);
                    self.left_rotate(parent);
                } else {
                    sibling.left().set_color(Color::Black);
                    self.right_rotate(parent);
                }
                node = self.root;
                break;
            }
        }
        node.set_color(Color::Black)
    }

    unsafe fn delete(&mut self, node: NodePtr<T>) -> (Name, T) {
        let mut child;
        let mut parent;
        let color;

        self.len -= 1;
        if !node.left().is_null() && !node.right().is_null() {
            let mut replace = node.next();
            if node.parent().is_null() {
                self.root = replace;
            } else if node.parent().left() == node {
                node.parent().set_left(replace);
            } else {
                node.parent().set_right(replace);
            }

            child = replace.right();
            parent = replace.parent();
            color = replace.get_color();
            if parent == node {
                parent = replace;
            } else {
                if !child.is_null() {
                    child.set_parent(parent);
                }
                parent.set_left(child);
                replace.set_right(node.right());
                node.right().set_parent(replace);
            }

            replace.set_parent(node.parent());
            replace.set_color(node.get_color());
            replace.set_left(node.left());
            node.left().set_parent(replace);

            if color == Color::Black {
                self.delete_fixup(child, parent);
            }

            return Box::from_raw(node.0).pair();
        }

        if !node.left().is_null() {
            child = node.left();
        } else {
            child = node.right();
        }
        if !child.is_null() {
            child.set_parent(node.parent());
        }

        if node.parent().is_null() {
            self.root = child
        } else if node.parent().left() == node {
            node.parent().set_left(child);
        } else {
            node.parent().set_right(child);
        }

        if node.is_black() {
            self.delete_fixup(child, node.parent());
        }

        Box::from_raw(node.0).pair()
    }

    pub fn keys(&self) -> Keys<T> {
        Keys { inner: self.iter() }
    }

    pub fn values(&self) -> Values<T> {
        Values { inner: self.iter() }
    }

    pub fn values_mut(&mut self) -> ValuesMut<T> {
        ValuesMut {
            inner: self.iter_mut(),
        }
    }

    pub fn iter(&self) -> Iter<T> {
        Iter {
            head: self.first_child(),
            tail: self.last_child(),
            len: self.len,
            _marker: marker::PhantomData,
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<T> {
        IterMut {
            head: self.first_child(),
            tail: self.last_child(),
            len: self.len,
            _marker: marker::PhantomData,
        }
    }
}
