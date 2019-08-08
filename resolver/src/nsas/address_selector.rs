use crate::nsas::address_entry::AddressEntry;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub fn select_address(addresses: &Vec<AddressEntry>) -> Option<AddressEntry> {
    addresses
        .iter()
        .filter(|a| a.is_v4())
        .min()
        .map(|a| a.clone())
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_address_selector() {
        let addresses = vec![
            AddressEntry::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 0),
            AddressEntry::new(IpAddr::V6(Ipv6Addr::new(0, 0, 0, 0, 0, 0, 0, 1)), 1),
            AddressEntry::new(IpAddr::V4(Ipv4Addr::new(2, 2, 2, 2)), 2),
        ];

        let target = select_address(&addresses);
        assert_eq!(target.unwrap().get_addr(), Ipv4Addr::new(1, 1, 1, 1));
    }
}
