use std::{
    cmp::{Eq, Ord, Ordering, PartialEq, PartialOrd},
    net::{IpAddr, Ipv4Addr},
    time::{Duration, Instant},
};

const UNREACHABLE_CACHE_TIME: u64 = 5;
const UNREACHABLE_RTT: u32 = u32::max_value();

#[derive(Clone, Copy)]
pub struct AddressEntry {
    address: IpAddr,
    rtt: u32,
    dead_util: Option<Instant>,
}

impl AddressEntry {
    pub fn new(address: IpAddr, rtt: u32) -> Self {
        AddressEntry {
            address,
            rtt,
            dead_util: None,
        }
    }

    #[inline]
    pub fn get_addr(&self) -> IpAddr {
        self.address
    }

    #[inline]
    pub fn get_v4_addr(&self) -> Ipv4Addr {
        match self.address {
            IpAddr::V4(addr) => addr,
            _ => unreachable!(),
        }
    }

    #[inline]
    pub fn get_rtt(&self) -> u32 {
        if self.dead_util.is_some() {
            UNREACHABLE_RTT
        } else {
            self.rtt
        }
    }

    #[inline]
    pub fn set_rtt(&mut self, rtt: u32) {
        if rtt == UNREACHABLE_RTT {
            self.dead_util = Some(
                Instant::now()
                    .checked_add(Duration::new(UNREACHABLE_CACHE_TIME, 0))
                    .expect("instant out of range"),
            );
        }
        self.rtt = rtt;
    }

    #[inline]
    pub fn is_reachable(&self) -> bool {
        self.rtt != UNREACHABLE_RTT
    }

    #[inline]
    pub fn set_unreachable(&mut self) {
        self.set_rtt(UNREACHABLE_RTT);
    }

    #[inline]
    pub fn is_v4(&self) -> bool {
        self.address.is_ipv4()
    }

    #[inline]
    pub fn is_v6(&self) -> bool {
        self.address.is_ipv6()
    }
}

impl PartialEq for AddressEntry {
    fn eq(&self, other: &AddressEntry) -> bool {
        self.address.eq(&other.address)
    }
}

impl Eq for AddressEntry {}

impl PartialOrd for AddressEntry {
    fn partial_cmp(&self, other: &AddressEntry) -> Option<Ordering> {
        self.rtt.partial_cmp(&other.rtt)
    }
}

impl Ord for AddressEntry {
    fn cmp(&self, other: &AddressEntry) -> Ordering {
        self.rtt.cmp(&other.rtt)
    }
}

pub fn select_address(addresses: &Vec<AddressEntry>) -> Option<AddressEntry> {
    addresses
        .iter()
        .filter(|a| a.get_addr().is_ipv4())
        .min()
        .map(|a| *a)
}

#[cfg(test)]
mod test {
    use super::*;
    use std::net::Ipv6Addr;

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
