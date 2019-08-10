use crate::Resolver;
use futures::{future, Future};
use r53::{
    HeaderFlag, Message, MessageBuilder, Name, Opcode, RData, RRClass, RRTtl, RRType, RRset, Rcode,
};
use std::cell::Cell;
use std::io::{Error, ErrorKind};

#[derive(Clone)]
pub struct DumbResolver {
    answer: Vec<RData>,
    additional: Vec<RRset>,
    failed_when_invoked: usize,
    invoked: Cell<usize>,
}

impl DumbResolver {
    pub fn new(failed_when_invoked: usize) -> Self {
        DumbResolver {
            answer: Vec::new(),
            additional: Vec::new(),
            failed_when_invoked,
            invoked: Cell::new(0),
        }
    }

    pub fn set_answer(&mut self, answer: Vec<RData>) {
        self.answer = answer;
    }

    pub fn set_additional(&mut self, additional: Vec<RRset>) {
        self.additional = additional;
    }
}

impl Resolver for DumbResolver {
    fn resolve(
        &self,
        name: Name,
        typ: RRType,
    ) -> Box<Future<Item = Message, Error = Error> + Send> {
        let invoked = self.invoked.get() + 1;
        self.invoked.set(invoked);
        if self.failed_when_invoked != 0 && invoked == self.failed_when_invoked {
            return Box::new(future::err(Error::new(ErrorKind::Other, "oh no")));
        }

        let mut msg = Message::with_query(name.clone(), typ);
        {
            let mut builder = MessageBuilder::new(&mut msg);
            builder
                .id(1200)
                .opcode(Opcode::Query)
                .rcode(Rcode::NoError)
                .set_flag(HeaderFlag::QueryRespone)
                .set_flag(HeaderFlag::AuthAnswer)
                .set_flag(HeaderFlag::RecursionDesired);

            let rrset = RRset {
                name: name,
                typ: typ,
                class: RRClass::IN,
                ttl: RRTtl(2000),
                rdatas: self.answer.clone(),
            };
            builder.add_answer(rrset);

            for rrset in &self.additional {
                builder.add_additional(rrset.clone());
            }
            builder.done();
        }
        Box::new(future::ok(msg))
    }
}
