extern crate futures;
extern crate tokio;
mod resolver;

use auth::{AuthServer, DynamicUpdateHandler};
use clap::{App, Arg};
use forwarder::Forwarder;
use server::UdpStream;
use std::net::SocketAddr;
use tokio::{net::UdpSocket, prelude::*};

fn main() {
    let matches = App::new("auth")
        .arg(
            Arg::with_name("dns_server")
                .help("dns server address")
                .long("dns")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("rpc_server")
                .help("rpc server address")
                .long("rpc")
                .required(false)
                .takes_value(true),
        )
        .get_matches();

    let addr = matches
        .value_of("dns_server")
        .unwrap_or("0.0.0.0:53")
        .to_string();
    let addr = addr.parse::<SocketAddr>().unwrap();
    let socket = UdpSocket::bind(&addr).unwrap();
    println!("Listening on: {}", socket.local_addr().unwrap());

    let auth_server = AuthServer::new();
    let forwarder = Forwarder::new("114.114.114.114:53".parse::<SocketAddr>().unwrap());
    let dynamic_server = DynamicUpdateHandler::new(auth_server.zones());
    let resolver = resolver::Resolver::new(auth_server, forwarder);
    let udp_stream = UdpStream::new(socket, resolver);

    let addr = matches.value_of("rpc_server").unwrap_or("0.0.0.0:5555");
    let addr_and_port = addr.split(":").collect::<Vec<&str>>();
    let _handler = dynamic_server.run(
        addr_and_port[0].to_string(),
        addr_and_port[1].parse().unwrap(),
    );
    tokio::run(udp_stream.map_err(|e| println!("server error = {:?}", e)));
}
