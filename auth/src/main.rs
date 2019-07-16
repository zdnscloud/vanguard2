#[macro_use]
extern crate futures;
extern crate tokio;

mod auth_server;
mod dynamic_server;
mod proto;
mod zones;

use crate::auth_server::AuthServer;
use crate::zones::AuthZone;
use clap::{App, Arg};
use dynamic_server::DynamicUpdateHandler;
use r53::Name;
use std::{
    net::SocketAddr,
    sync::{Arc, RwLock},
};
use tokio::{net::UdpSocket, prelude::*};

fn main() {
    let matches = App::new("auth")
        .arg(
            Arg::with_name("server")
                .help("server address")
                .short("s")
                .long("server")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("zonefile")
                .help("zone file path")
                .short("f")
                .long("zonefile")
                .required(true)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("zonename")
                .help("zone name")
                .short("z")
                .long("zonename")
                .required(true)
                .takes_value(true),
        )
        .get_matches();

    let addr = matches
        .value_of("server")
        .unwrap_or("0.0.0.0:53")
        .to_string();
    let addr = addr.parse::<SocketAddr>().unwrap();
    let socket = UdpSocket::bind(&addr).unwrap();
    println!("Listening on: {}", socket.local_addr().unwrap());

    let zone_file = matches
        .value_of("zonefile")
        .unwrap_or("/etc/vanguard2/example.org.zone")
        .to_string();
    let zone_name = matches
        .value_of("zonename")
        .unwrap_or("example.org")
        .to_string();

    let mut zones = AuthZone::new();
    zones
        .load_zone(Name::new(zone_name.as_str()).unwrap(), zone_file.as_str())
        .unwrap();
    let zones = Arc::new(RwLock::new(zones));
    let auth_server = AuthServer::new(socket, zones.clone());

    let dynamic_server = DynamicUpdateHandler::new(zones.clone());
    let handler = dynamic_server.run("127.0.0.1".to_string(), 5555);
    tokio::run(auth_server.map_err(|e| println!("server error = {:?}", e)));
}
