use crate::{
    error::RecursorError,
    message_classifier::{classify_response, ResponseCategory},
    nsas::ZoneFetcher,
    resolver::Recursor,
    sender::Sender,
};
use failure;
use futures::{future, prelude::*, Future};
use r53::{message::SectionType, name, Message, MessageBuilder, Name, RData, RRType, Rcode};
use std::{mem, time::Duration};

const MAX_CNAME_DEPTH: usize = 12;
const MAX_QUERY_DEPTH: usize = 10;

enum State {
    Init,
    GetNameServer(ZoneFetcher),
    QueryAuthServer(Sender),
    Poisoned,
}

pub struct RunningQuery {
    current_name: Name,
    current_type: RRType,
    current_zone: Option<Name>,
    cname_depth: usize,
    query: Option<Message>,
    response: Option<Message>,
    recursor: Recursor,
    state: State,
    depth: usize,
}

impl RunningQuery {
    pub fn new(query: Message, recursor: Recursor, depth: usize) -> Self {
        let question = query.question.as_ref().unwrap();
        let current_name = question.name.clone();
        let current_type = question.typ;

        RunningQuery {
            current_name,
            current_type,
            current_zone: None,
            cname_depth: 0,
            query: Some(query.clone()),
            response: Some(query),
            recursor,
            state: State::Init,
            depth,
        }
    }

    fn lookup_in_cache(&mut self) -> failure::Result<Option<Message>> {
        if self
            .recursor
            .cache
            .lock()
            .unwrap()
            .gen_response(self.query.as_mut().unwrap())
        {
            let last_answer = self.query.take().unwrap();
            return Ok(Some(self.make_response(last_answer)));
        }

        let mut cache = self.recursor.cache.lock().unwrap();
        if let Some(ns) = cache.get_deepest_ns(&self.current_name) {
            self.current_zone = Some(ns);
        } else {
            return Err(RecursorError::NoNameserver.into());
        }
        return Ok(None);
    }

    pub fn handle_response(&mut self, response: Message) -> failure::Result<Option<Message>> {
        let response_type = classify_response(&self.current_name, self.current_type, &response);
        match response_type {
            ResponseCategory::Answer
            | ResponseCategory::AnswerCName
            | ResponseCategory::NXDomain
            | ResponseCategory::NXRRset => {
                let response = self.make_response(response);
                self.recursor
                    .cache
                    .lock()
                    .unwrap()
                    .add_response(response_type, response.clone());
                return Ok(Some(response));
            }
            ResponseCategory::Referral => {
                self.recursor
                    .cache
                    .lock()
                    .unwrap()
                    .add_response(response_type, response.clone());
                if !self.fetch_closer_zone(response) {
                    return Ok(Some(self.make_server_failed()));
                } else {
                    return Ok(None);
                }
            }
            ResponseCategory::CName(next) => {
                println!("get cname and query {:?}", next);
                self.cname_depth += response.header.an_count as usize;
                if self.cname_depth > MAX_CNAME_DEPTH {
                    return Ok(Some(self.make_server_failed()));
                }
                self.merge_response(response);
                self.current_name = next.clone();
                self.current_zone = None;
                self.query = Some(Message::with_query(next, self.current_type));
                return Ok(None);
            }
            ResponseCategory::Invalid(_) | ResponseCategory::FormErr => {
                return Ok(Some(self.make_server_failed()));
            }
        }
    }

    fn make_response(&mut self, mut response: Message) -> Message {
        let mut accumulate_response = self.response.take().unwrap();
        let mut builder = MessageBuilder::new(&mut accumulate_response);
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
        accumulate_response
    }

    fn make_server_failed(&mut self) -> Message {
        let mut accumulate_response = self.response.take().unwrap();
        let mut builder = MessageBuilder::new(&mut accumulate_response);
        builder.rcode(Rcode::ServFail);
        builder.done();
        accumulate_response
    }

    fn merge_response(&mut self, mut response: Message) {
        let mut builder = MessageBuilder::new(self.response.as_mut().unwrap());
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

impl Future for RunningQuery {
    type Item = Message;
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        loop {
            match mem::replace(&mut self.state, State::Poisoned) {
                State::Init => match self.lookup_in_cache() {
                    Err(e) => {
                        return Err(e);
                    }
                    Ok(None) => {
                        if let Some(nameserver) = self
                            .recursor
                            .nsas
                            .get_nameserver(self.current_zone.as_ref().unwrap())
                        {
                            self.state = State::QueryAuthServer(Sender::new(
                                self.query.as_ref().unwrap().clone(),
                                nameserver,
                                self.recursor.nsas.clone(),
                            ));
                        } else {
                            if self.depth > MAX_QUERY_DEPTH {
                                println!(
                                    "----> query {:?}, {:?} {} , failed loop query",
                                    self.current_name,
                                    self.current_type.to_string(),
                                    self.depth,
                                );
                                return Err(RecursorError::LoopedQuery.into());
                            }

                            self.state = State::GetNameServer(self.recursor.nsas.fetch_zone(
                                self.current_zone.as_ref().unwrap().clone(),
                                self.depth,
                            ));
                        }
                    }
                    Ok(Some(resp)) => {
                        return Ok(Async::Ready(resp));
                    }
                },
                State::GetNameServer(mut fetcher) => match fetcher.poll() {
                    Err(e) => {
                        return Err(e);
                    }
                    Ok(Async::NotReady) => {
                        self.state = State::GetNameServer(fetcher);
                        return Ok(Async::NotReady);
                    }
                    Ok(Async::Ready(nameserver)) => {
                        self.state = State::QueryAuthServer(Sender::new(
                            self.query.as_ref().unwrap().clone(),
                            nameserver,
                            self.recursor.nsas.clone(),
                        ));
                    }
                },
                State::QueryAuthServer(mut sender) => match sender.poll() {
                    Err(e) => {
                        return Err(RecursorError::TimerErr(format!("{:?}", e)).into());
                    }
                    Ok(Async::NotReady) => {
                        self.state = State::QueryAuthServer(sender);
                        return Ok(Async::NotReady);
                    }
                    Ok(Async::Ready(resp)) => match self.handle_response(resp) {
                        Err(e) => {
                            return Err(e);
                        }
                        Ok(Some(resp)) => {
                            return Ok(Async::Ready(resp));
                        }
                        Ok(None) => {
                            self.state = State::Init;
                        }
                    },
                },
                State::Poisoned => {
                    panic!("running query state is corrupted");
                }
            }
        }
    }
}
