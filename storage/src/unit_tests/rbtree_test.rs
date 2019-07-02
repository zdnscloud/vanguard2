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

        //duplicate insert should return old value
        for (key, value) in &back{
            assert_eq!(tree.insert(key.clone(), value.clone()), Some(value.clone()));
        }

        for (key, value) in &back {
            assert_eq!(tree.get(key).unwrap(), value);
        }

        let mut hm_keys = back.keys().collect::<Vec<&String>>();
        hm_keys.sort();
        assert_eq!(tree.keys().collect::<Vec<&String>>(), hm_keys);

        let half = tree.len()/2;
        for key in back.keys().take(half).map(|s| s.clone()).collect::<Vec<String>>() {
            assert_eq!(tree.remove(&key).unwrap(), *back.remove(&key).unwrap());
        }
        assert_eq!(tree.len(), back.len());

        let half = tree.len()/2;
        for key in tree.keys().take(half).map(|s| s.clone()).collect::<Vec<String>>() {
            assert_eq!(tree.remove(&key).unwrap(), *back.remove(&key).unwrap());
        }
        assert_eq!(tree.len(), back.len());
    }
}
