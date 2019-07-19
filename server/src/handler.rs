use futures::Future;
use r53::Message;
use std::net::SocketAddr;

pub trait QueryHandler {
    fn handle_query(&mut self, query: Query) -> Query;
}

pub trait ResponseSender: Clone + Send {
    fn send_response(&mut self, response: Query) -> std::io::Result<()>;
}

pub trait QueryService: Send {
    type ResponseSender: ResponseSender;

    fn is_capable(&self, query: &Query) -> bool;
    fn handle_query(
        &mut self,
        query: Query,
        sender: Self::ResponseSender,
    ) -> Box<dyn Future<Item = (), Error = ()> + Send + 'static>;
}

#[derive(Debug, Clone)]
pub struct Query {
    pub client: SocketAddr,
    pub message: Message,
}

impl Query {
    pub fn new(message: Message, client: SocketAddr) -> Self {
        Query { client, message }
    }
}
