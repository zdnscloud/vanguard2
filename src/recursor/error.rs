use failure::Fail;
use std::{io, net::IpAddr};

#[derive(Debug, Fail)]
pub enum RecursorError {
    #[fail(display = "IO error: {}", _0)]
    IoError(#[fail(cause)] io::Error),

    #[fail(display = "query {} timed out", _0)]
    Timeout(IpAddr),

    #[fail(display = "timer error {}", _0)]
    TimerErr(String),

    #[fail(display = "no name server is found")]
    NoNameserver,

    #[fail(display = "query get loop")]
    LoopedQuery,
}

mod test {
    use super::*;

    #[test]
    fn test_err() {
        let err: failure::Error = RecursorError::TimerErr("good".to_string()).into();
        assert_eq!(format!("{:?}", err), "TimerErr(\"good\")".to_string());
    }
}
