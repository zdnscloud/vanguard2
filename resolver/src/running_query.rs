use crate::{
    error::RecursorError,
    message_classifier::{classify_response, ResponseCategory},
    resolver::Recursor,
    sender::Sender,
};
use futures::{future, Future};
use r53::{message::SectionType, name, Message, MessageBuilder, Name, RData, RRType, Rcode};
use std::mem;

pub struct RunningQuery {
    current_name: Name,
    current_type: RRType,
    current_zone: Option<Name>,
    cname_depth: usize,
    response: Message,
    recursor: Recursor,
}

impl RunningQuery {
    pub fn new(query: Message, recursor: Recursor) -> Self {
        let question = query.question.as_ref().unwrap();
        let current_name = question.name.clone();
        let current_type = question.typ;
        RunningQuery {
            current_name,
            current_type,
            current_zone: None,
            cname_depth: 0,
            response: query,
            recursor,
        }
    }

    pub fn resolve(
        mut self,
    ) -> Box<Future<Item = Message, Error = failure::Error> + Send + 'static> {
        let mut query = Message::with_query(self.current_name.clone(), self.current_type);
        let mut cache_hit = false;
        {
            let mut cache = self.recursor.cache.lock().unwrap();

            if self.current_zone.is_none() {
                if let Some(ns) = cache.get_deepest_ns(&self.current_name) {
                    self.current_zone = Some(ns);
                } else {
                    return Box::new(future::err(RecursorError::NoNameserver.into()));
                }
            }
            if cache.gen_response(&mut query) {
                cache_hit = true;
            }
        }

        println!(
            "get query {:?} and use zone {:?}",
            self.current_name,
            self.current_zone.as_ref().unwrap()
        );

        if cache_hit {
            println!(
                "---> get respon from cache {:?}",
                query.section(SectionType::Answer)
            );
            return Box::new(future::ok(self.make_response(query)));
        }

        let query_name = self.current_name.clone();
        let query_typ = self.current_type;
        let nsas = self.recursor.nsas.clone();
        Box::new(
            self.recursor
                .nsas
                .get_nameserver(self.current_zone.as_ref().unwrap(), self.recursor.clone())
                .and_then(move |nameserver| {
                    Sender::new(
                        Message::with_query(query_name, query_typ),
                        nameserver,
                        nsas.clone(),
                    )
                })
                .and_then(move |response| self.handle_response(response)),
        )
    }

    pub fn handle_response(
        mut self,
        response: Message,
    ) -> Box<Future<Item = Message, Error = failure::Error> + Send + 'static> {
        let response_type = classify_response(&self.current_name, self.current_type, &response);
        println!(
            "get response {:?} for query {:?} {:?}",
            response_type, self.current_name, self.current_type
        );
        match response_type {
            ResponseCategory::Answer
            | ResponseCategory::AnswerCName
            | ResponseCategory::NXDomain
            | ResponseCategory::NXRRset => {
                self.recursor
                    .cache
                    .lock()
                    .unwrap()
                    .add_response(response_type, response.clone());

                return Box::new(future::ok(self.make_response(response)));
            }
            ResponseCategory::Referral => {
                self.recursor
                    .cache
                    .lock()
                    .unwrap()
                    .add_response(response_type, response.clone());
                if !self.fetch_closer_zone(response) {
                    return Box::new(future::ok(self.make_server_failed()));
                }
                println!(
                    "get refer for query {:?} use new zone {:?}",
                    self.current_name,
                    self.current_zone.as_ref().unwrap(),
                );
                self.resolve()
            }
            ResponseCategory::CName(next) => {
                println!("get cname and query {:?}", next);
                self.merge_response(response);
                self.current_name = next;
                self.current_zone = None;
                self.resolve()
            }
            ResponseCategory::Invalid(_) | ResponseCategory::FormErr => {
                return Box::new(future::ok(self.make_server_failed()));
            }
        }
    }

    fn make_response(mut self, mut response: Message) -> Message {
        let mut builder = MessageBuilder::new(&mut self.response);
        builder.make_response();
        builder.rcode(response.header.rcode);
        if let Some(answers) = response.take_section(SectionType::Answer) {
            for answer in answers {
                builder.add_answer(answer);
            }
        }

        if let Some(auths) = response.take_section(SectionType::Authority) {
            for auth in auths {
                builder.add_auth(auth);
            }
        }

        if let Some(additionals) = response.take_section(SectionType::Additional) {
            for additional in additionals {
                builder.add_additional(additional);
            }
        }
        builder.done();
        self.response
    }

    fn make_server_failed(mut self) -> Message {
        let mut builder = MessageBuilder::new(&mut self.response);
        builder.rcode(Rcode::ServFail);
        builder.done();
        self.response
    }

    fn merge_response(&mut self, mut response: Message) {
        let mut builder = MessageBuilder::new(&mut self.response);
        if let Some(answers) = response.take_section(SectionType::Answer) {
            for answer in answers {
                builder.add_answer(answer);
            }
        }
    }

    fn fetch_closer_zone(&mut self, mut response: Message) -> bool {
        let auth = response
            .take_section(SectionType::Authority)
            .expect("refer response should has answer");
        if auth.len() != 1 || auth[0].typ != RRType::NS {
            return false;
        }

        let current_zone = self.current_zone.as_ref().unwrap();
        let zone = auth[0].name.clone();
        if zone.is_subdomain(current_zone) && self.current_name.is_subdomain(&zone) {
            self.current_zone = Some(zone);
            return true;
        }
        return false;
    }
}
