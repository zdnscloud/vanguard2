#[macro_use]
extern crate futures;
extern crate tokio;

mod handler;
mod udp_stream;

pub use handler::{Done, Failed, Query, QueryHandler};
pub use udp_stream::{UdpStream, UdpStreamSender};