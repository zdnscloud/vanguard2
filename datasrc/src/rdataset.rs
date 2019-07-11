use crate::error::DataSrcError;
use failure::Result;
use r53::{Name, RData, RRClass, RRTtl, RRType, RRset};

type RRsetTuple = (RRType, RRTtl, Vec<RData>);

pub struct Rdataset {
    rrsets: Vec<RRsetTuple>,
}

impl Rdataset {
    pub fn new() -> Self {
        Rdataset { rrsets: Vec::new() }
    }

    pub fn add_rrset(&mut self, mut rrset: RRset) -> Result<()> {
        debug_assert!(!rrset.rdatas.is_empty());

        if rrset.typ == RRType::CNAME {
            if !self.rrsets.is_empty() {
                return Err(DataSrcError::CNameCoExistsWithOtherRR.into());
            }
        } else if self.get_rrset_tuple(RRType::CNAME).is_some() {
            return Err(DataSrcError::CNameCoExistsWithOtherRR.into());
        }

        if let Some(index) = self.get_rrset_tuple(rrset.typ) {
            self.rrsets[index].1 = rrset.ttl;
            //todo: remove duplicate
            self.rrsets[index].2.append(&mut rrset.rdatas);
        } else {
            self.rrsets.push((rrset.typ, rrset.ttl, rrset.rdatas));
        }
        Ok(())
    }

    pub fn get_rrset(&self, name: &Name, typ: RRType) -> Option<RRset> {
        self.get_rrset_tuple(typ).map(|index| RRset {
            name: name.clone(),
            typ,
            class: RRClass::IN,
            ttl: self.rrsets[index].1,
            rdatas: self.rrsets[index].2.clone(),
        })
    }

    pub fn remove_rrset(&mut self, typ: RRType) {
        if let Some(index) = self.get_rrset_tuple(typ) {
            self.rrsets.remove(index);
        }
    }

    pub fn remove_rdata(&mut self, rrset: &RRset) {
        if let Some(index) = self.get_rrset_tuple(rrset.typ) {
            for rdata in &rrset.rdatas {
                if let Some(index_) = self.rrsets[index]
                    .2
                    .iter()
                    .position(|current| rdata.eq(current))
                {
                    self.rrsets[index].2.remove(index_);
                }
            }

            if self.rrsets[index].2.is_empty() {
                self.rrsets.remove(index);
            }
        }
    }

    fn get_rrset_tuple(&self, typ: RRType) -> Option<usize> {
        self.rrsets.iter().position(|rrset| rrset.0 == typ)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn build_a_rrset(name: &str, ips: &[&str]) -> RRset {
        ips.iter()
            .map(|ip| format!("{} 3600 IN A {}", name, ip))
            .fold(None, |rrset: Option<RRset>, s| {
                let mut new = RRset::from_str(s.as_str()).unwrap();
                if let Some(mut old) = rrset {
                    old.rdatas.append(&mut new.rdatas);
                    Some(old)
                } else {
                    Some(new)
                }
            })
            .unwrap()
    }

    #[test]
    fn test_get_rrset() {
        let a_rrset = build_a_rrset("a.cn", &["1.1.1.1", "2.2.2.2"]);
        let mut rrset = Rdataset::new();
        rrset.add_rrset(a_rrset.clone()).unwrap();
        assert_eq!(
            rrset.get_rrset(&Name::new("a.cn").unwrap(), RRType::A),
            Some(a_rrset)
        );
    }

    #[test]
    fn test_remove_rrset() {
        let name = Name::new("a.cn").unwrap();
        let mut rrset = Rdataset::new();
        rrset
            .add_rrset(build_a_rrset("a.cn", &["1.1.1.1", "2.2.2.2"]))
            .unwrap();
        rrset.remove_rdata(&build_a_rrset("a.cn", &["1.1.1.1"]));
        assert_eq!(
            rrset.get_rrset(&name, RRType::A),
            Some(build_a_rrset("a.cn", &["2.2.2.2"]))
        );
        rrset.remove_rdata(&build_a_rrset("a.cn", &["2.2.2.2", "3.3.3.3"]));
        assert_eq!(rrset.get_rrset(&name, RRType::A), None,);

        let new_rrset = build_a_rrset("a.cn", &["1.1.1.1", "2.2.2.2"]);
        rrset.add_rrset(new_rrset.clone()).unwrap();
        assert_eq!(rrset.get_rrset(&name, RRType::A), Some(new_rrset));
        rrset.remove_rrset(RRType::A);
        assert_eq!(rrset.get_rrset(&name, RRType::A), None);
    }
}
