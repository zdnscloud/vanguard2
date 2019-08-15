use auth::AuthServer;
use forwarder::Forwarder;
use futures::{future, Future};
use r53::{Message, Name, RRType, RRset};
use resolver::{MessageCache, RRsetTrustLevel, Recursor};
use server::{Done, Failed, Query, QueryHandler};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

const DEFAULT_MESSAGE_CACHE_SIZE: usize = 10000;

pub struct Resolver {
    auth: AuthServer,
    forwarder: Forwarder,
    recursor: Recursor,
}

impl Resolver {
    pub fn new(auth: AuthServer, forwarder: Forwarder) -> Self {
        let mut cache = MessageCache::new(DEFAULT_MESSAGE_CACHE_SIZE);
        cache.add_rrset(
            RRset::from_str(". 441018 IN NS j.root-servers.net.").unwrap(),
            RRsetTrustLevel::AuthorityWithAA,
        );
        cache.add_rrset(
            RRset::from_str("j.root-servers.net. 569490	IN A 192.58.128.30").unwrap(),
            RRsetTrustLevel::AuthorityWithAA,
        );
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
