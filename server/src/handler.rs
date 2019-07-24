use futures::Future;
use r53::Message;
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Query {
    pub client: SocketAddr,
    pub message: Message,
}

pub struct Done(pub Query);
pub struct Failed(pub Query);

impl Query {
    pub fn new(message: Message, client: SocketAddr) -> Self {
        Query { client, message }
    }
}

pub trait QueryHandler: Send {
    fn handle_query(
        &mut self,
        query: Query,
    ) -> Box<dyn Future<Item = Done, Error = Failed> + Send + 'static>;
}

pub trait HandlerLayer<S: QueryHandler> {
    type Handler: QueryHandler;
    fn layer(&self, inner: S) -> Self::Handler;
}
