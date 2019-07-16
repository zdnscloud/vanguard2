use crate::domaintree::{
    node_chain::NodeChain,
    tree::{FindResultFlag, RBTree},
};
use proptest::{collection::vec, prelude::*};
use r53::Name;
use std::collections::HashSet;

prop_compose! {
    fn arb_name()(
        mut labels in vec("[a-z0-9]{1,63}", 1..128).prop_map(|v| v.join(".")),
        ) -> String {
        if labels.len() >= 254 {
            labels.truncate(253);
        }
        labels
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(10))]
    #[ignore]
    #[test]
    fn prop_test_insert_delete_batch(
        name_strings in vec(arb_name(), 100..1000)
    ) {
        //filter duplicate name
        let name_and_values = name_strings
        .into_iter()
        .fold(HashSet::new(), |mut set, n| {
            set.insert(n);
           set
        })
        .into_iter()
        .map(|labels| Name::new(labels.as_ref()).expect("rand name isn't valid"))
        .zip(1..)
        .collect::<Vec<(Name, u32)>>();
        test_insert_delete_batch(name_and_values);
    }
}

pub fn test_insert_delete_batch(name_and_values: Vec<(Name, u32)>) {
    let mut tree = RBTree::<u32>::new();
    for (name, value) in &name_and_values {
        let (_, old) = tree.insert(name.clone(), Some(*value));
        //Some(None) == non-terminal node is created
        //None == new node
        assert!(old == Some(None) || old == None);
    }

    //duplicate insert should return old value
    for (name, value) in &name_and_values {
        let (_, old) = tree.insert(name.clone(), Some(*value));
        assert_eq!(old.unwrap(), Some(*value));
    }

    for (name, value) in &name_and_values {
        let mut node_chain = NodeChain::new(&tree);
        let result = tree.find_node(name, &mut node_chain);
        assert_eq!(result.flag, FindResultFlag::ExacatMatch);
        assert_eq!(result.node.get_value(), &Some(*value));
    }

    for (name, value) in name_and_values {
        let mut node_chain = NodeChain::new(&tree);
        let result = tree.find_node(&name, &mut node_chain);
        let node = result.node;
        assert_eq!(tree.remove_node(node).unwrap(), value);
    }

    assert_eq!(tree.len(), 0);
}
