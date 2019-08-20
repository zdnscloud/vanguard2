use auth::AuthServer;
use failure;
use futures::{future, Future};
use resolver::Recursor;
use server::{Query, QueryHandler};

pub struct Resolver {
    auth: AuthServer,
    recursor: Recursor,
}

impl Resolver {
    pub fn new(auth: AuthServer) -> Self {
        let recursor = Recursor::new();
        Resolver { auth, recursor }
    }
}

impl QueryHandler for Resolver {
    fn handle_query(
        &self,
        query: Query,
    ) -> Box<dyn Future<Item = Query, Error = failure::Error> + Send + 'static> {
        Box::new(self.recursor.handle_query(query))
    }
}
