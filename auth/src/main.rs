#[macro_use]
extern crate futures;
extern crate tokio;

mod auth_server;
mod dynamic_server;
mod error;
mod proto;
mod zones;

use crate::auth_server::AuthServer;
use crate::zones::AuthZone;
use clap::{App, Arg};
use dynamic_server::DynamicUpdateHandler;
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};
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

    let zones = AuthZone::new();
    let zones = Arc::new(RwLock::new(zones));
    let auth_server = AuthServer::new(socket, zones.clone());

    let addr = matches.value_of("rpc_server").unwrap_or("0.0.0.0:5555");
    let addr_and_port = addr.split(":").collect::<Vec<&str>>();
    let dynamic_server = DynamicUpdateHandler::new(zones.clone());
    let _handler = dynamic_server.run(
        addr_and_port[0].to_string(),
        addr_and_port[1].parse().unwrap(),
    );
    tokio::run(auth_server.map_err(|e| println!("server error = {:?}", e)));
}
