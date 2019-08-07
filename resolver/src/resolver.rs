use futures::Future;
use r53::{Message, Name, RRType};
use std::io;

pub trait Resolver {
    fn resolve(&self, name: Name, typ: RRType) -> Box<Future<Item = Message, Error = io::Error>>;
}
