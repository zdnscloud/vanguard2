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
}
