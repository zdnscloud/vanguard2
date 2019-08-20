extern crate futures;
extern crate tokio;
mod resolver;

use auth::{AuthServer, DynamicUpdateHandler};
use clap::{App, Arg};
use metrics::start_metric_server;
use server::{start_qps_calculate, Server};
use std::net::SocketAddr;
use std::thread;
use tokio::runtime::current_thread;

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
        .arg(
            Arg::with_name("forwarder")
                .help("dns recursive server address to forward request")
                .long("forwarder")
                .required(false)
                .takes_value(true),
        )
        .arg(
            Arg::with_name("metrics")
                .help("dns recursive server address to forward request")
                .long("metrics")
                .required(false)
                .takes_value(true),
        )
        .get_matches();

    let addr = matches
        .value_of("dns_server")
        .unwrap_or("0.0.0.0:53")
        .to_string();
    let dns_addr = addr.parse::<SocketAddr>().unwrap();
    println!("Listening on: {}", dns_addr);

    let auth_server = AuthServer::new();
    let dynamic_server = DynamicUpdateHandler::new(auth_server.zones());
    let resolver = resolver::Resolver::new(auth_server);
    let server = Server::new(dns_addr, resolver);

    let addr = matches.value_of("rpc_server").unwrap_or("0.0.0.0:5555");
    let addr_and_port = addr.split(":").collect::<Vec<&str>>();
    let _handler = dynamic_server.run(
        addr_and_port[0].to_string(),
        addr_and_port[1].parse().unwrap(),
    );

    let addr = matches
        .value_of("metrics")
        .unwrap_or("0.0.0.0:9100")
        .to_string();
    let addr = addr.parse::<SocketAddr>().unwrap();
    start_metrics(addr);

    tokio::run(server.into_future());
}

fn start_metrics(addr: SocketAddr) {
    thread::Builder::new()
        .name("metrics".into())
        .spawn(move || {
            let mut rt = current_thread::Runtime::new().unwrap();
            rt.spawn(start_metric_server(
                addr,
                "/metrics".to_string(),
                "/statistics".to_string(),
            ));
            rt.block_on(start_qps_calculate()).unwrap();
        })
        .unwrap();
}
