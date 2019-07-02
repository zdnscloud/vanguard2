use crate::rbtree::rbnode::{Color, NodePtr};
use crate::rbtree::rbtree::RBTree;
use proptest::prelude::*;
use std::fmt::Debug;

fn insert_then_delete_get_none<K: Ord + Clone, V: Debug + PartialEq + Clone>(
    tree: &mut RBTree<K, V>,
    key: K,
    value: V,
) {
    let back = key.clone();
    assert_eq!(tree.insert(key, value.clone()), None);
    assert_eq!(tree.len(), 1);
    assert_eq!(*tree.get(&back).unwrap(), value);
    assert!(tree.remove(&back).is_some());
    assert_eq!(tree.get(&back), None);
    assert_eq!(tree.len(), 0);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]
    #[test]
    fn test_insert_delete_single((key, value) in (1..10000u32, "[a-z]*")) {
        let mut m = RBTree::<u32, String>::new();
        insert_then_delete_get_none(&mut m, key.clone(), value.clone());
        let mut m = RBTree::<String, u32>::new();
        insert_then_delete_get_none(&mut m, value, key);
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]
    #[test]
    fn test_insert_delete_batch(hm in prop::collection::hash_map(".*", ".*", 0..1000)) {
        let mut tree = RBTree::<String, String>::new();
        let mut back = hm.clone();
        for (key, value) in hm {
            assert_eq!(tree.insert(key, value.clone()), None);
        }

        for (key, value) in &back {
            assert_eq!(tree.get(key).unwrap(), value);
        }

        assert_eq!(tree.len(), back.len());
    }
}
