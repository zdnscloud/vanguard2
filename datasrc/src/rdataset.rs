use r53::{Name, RData, RRClass, RRTtl, RRType, RRset};

type RRsetTuple = (RRType, RRTtl, Vec<RData>);

pub struct Rdataset {
    rrsets: Vec<RRsetTuple>,
}

impl Rdataset {
    pub fn new(rrset: RRset) -> Self {
        let mut rs = Rdataset { rrsets: Vec::new() };
        rs.add_rrset(rrset);
        rs
    }

    pub fn add_rrset(&mut self, mut rrset: RRset) {
        debug_assert!(!rrset.rdatas.is_empty());

        if let Some(index) = self.get_rrset_tuple(rrset.typ) {
            self.rrsets[index].1 = rrset.ttl;
            //todo: remove duplicate
            self.rrsets[index].2.append(&mut rrset.rdatas);
        } else {
            self.rrsets.push((rrset.typ, rrset.ttl, rrset.rdatas));
        }
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

    fn build_a_rrset(name: &str, ips: &[&str]) -> RRset {
        let mut iter = ips.iter().map(|s| *s);
        let rdatas = (0..ips.len()).fold(Vec::new(), |mut rdatas, _| {
            rdatas.push(RData::from_string(RRType::A, &mut iter).unwrap());
            rdatas
        });

        RRset {
            name: Name::new(name).unwrap(),
            typ: RRType::A,
            class: RRClass::IN,
            ttl: RRTtl(3600),
            rdatas,
        }
    }

    #[test]
    fn test_get_rrset() {
        let a_rrset = build_a_rrset("a.cn", &["1.1.1.1", "2.2.2.2"]);
        let rrset = Rdataset::new(a_rrset.clone());
        assert_eq!(
            rrset.get_rrset(&Name::new("a.cn").unwrap(), RRType::A),
            Some(a_rrset)
        );
    }

    #[test]
    fn test_remove_rrset() {
        let name = Name::new("a.cn").unwrap();
        let mut rrset = Rdataset::new(build_a_rrset("a.cn", &["1.1.1.1", "2.2.2.2"]));
        rrset.remove_rdata(&build_a_rrset("a.cn", &["1.1.1.1"]));
        assert_eq!(
            rrset.get_rrset(&name, RRType::A),
            Some(build_a_rrset("a.cn", &["2.2.2.2"]))
        );
        rrset.remove_rdata(&build_a_rrset("a.cn", &["2.2.2.2", "3.3.3.3"]));
        assert_eq!(rrset.get_rrset(&name, RRType::A), None,);

        let new_rrset = build_a_rrset("a.cn", &["1.1.1.1", "2.2.2.2"]);
        rrset.add_rrset(new_rrset.clone());
        assert_eq!(rrset.get_rrset(&name, RRType::A), Some(new_rrset));
        rrset.remove_rrset(RRType::A);
        assert_eq!(rrset.get_rrset(&name, RRType::A), None);
    }
}
