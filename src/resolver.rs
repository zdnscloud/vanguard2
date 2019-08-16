use auth::AuthServer;
use forwarder::Forwarder;
use futures::{future, Future};
use r53::{Message, Name, RData, RRType, RRset};
use resolver::{MessageCache, RRsetTrustLevel, Recursor};
use server::{Done, Failed, Query, QueryHandler};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

const DEFAULT_MESSAGE_CACHE_SIZE: usize = 10000;
const ROOT_SERVERS: [&'static str; 13] = [
    "a.root-servers.net.",
    "b.root-servers.net.",
    "c.root-servers.net.",
    "d.root-servers.net.",
    "e.root-servers.net.",
    "f.root-servers.net.",
    "g.root-servers.net.",
    "h.root-servers.net.",
    "i.root-servers.net.",
    "j.root-servers.net.",
    "k.root-servers.net.",
    "l.root-servers.net.",
    "m.root-servers.net.",
];

pub struct Resolver {
    auth: AuthServer,
    forwarder: Forwarder,
    recursor: Recursor,
}

impl Resolver {
    pub fn new(auth: AuthServer, forwarder: Forwarder) -> Self {
        let mut cache = MessageCache::new(DEFAULT_MESSAGE_CACHE_SIZE);
        let root = ROOT_SERVERS
            .iter()
            .fold(None, |rrset: Option<RRset>, ns| {
                if let Some(mut rrset) = rrset {
                    let rdata = RData::from_str(RRType::NS, ns).unwrap();
                    rrset.rdatas.push(rdata);
                    Some(rrset)
                } else {
                    Some(RRset::from_str(format!(". 441018 IN NS {}", ns).as_ref()).unwrap())
                }
            })
            .unwrap();
        cache.add_rrset(root, RRsetTrustLevel::AuthorityWithAA);
        let recursor = Recursor::new(cache);
        Resolver {
            auth,
            forwarder,
            recursor,
        }
    }
}

impl QueryHandler for Resolver {
    fn handle_query(
        &self,
        query: Query,
    ) -> Box<dyn Future<Item = Done, Error = Failed> + Send + 'static> {
        //let forwarder = self.forwarder.clone();
        let client = query.client;
        Box::new(self.recursor.handle_query(query).map_err(move |e| {
            println!("resolver get err: {:?}", e);
            Failed(Query {
                client,
                message: Message::with_query(Name::new("fuck").unwrap(), RRType::A),
            })
        }))
    }
}
