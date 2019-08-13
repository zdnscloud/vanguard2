use crate::resolver::Recursor;
use r53::{Message, Name, RRType};

pub struct RunningQuery {
    request: Message,
    current_name: Name,
    current_type: RRType,
    current_zone: Name,
    cname_depth: usize,
    recursor: Recursor,
}

impl RunningQuery {
    pub fn new(request: Message, current_zone: Name, recursor: Recursor) -> Self {
        let question = request.question.as_ref().unwrap();
        let current_name = question.name.clone();
        let current_type = question.typ;
        RunningQuery {
            request,
            current_name,
            current_type,
            current_zone,
            cname_depth: 0,
            recursor,
        }
    }
}
