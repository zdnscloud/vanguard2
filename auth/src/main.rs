#[macro_use]
extern crate futures;
extern crate tokio;

use clap::{App, Arg};
use datasrc::{zone::FindResult, zone::ZoneFinder, FindOption, FindResultType, MemoryZone, RBTree};
use failure::Result;
use r53::{HeaderFlag, Message, MessageBuilder, MessageRender, Name, RData, RRType, RRset, Rcode};
use std::{
    fs::File,
    io,
    io::{BufRead, BufReader},
    net::SocketAddr,
    str::FromStr,
};
use tokio::{net::UdpSocket, prelude::*};

pub struct Auth {
    socket: UdpSocket,
    zones: RBTree<MemoryZone>,
    current_user: Option<SocketAddr>,
    render: MessageRender,
    buf: Vec<u8>,
}

impl Auth {
    pub fn new(socket: UdpSocket) -> Self {
        Auth {
            socket: socket,
            zones: RBTree::new(),
            current_user: None,
            render: MessageRender::new(),
            buf: vec![0; 1024],
        }
    }

    pub fn load_zone(&mut self, name: Name, path: &str) -> Result<()> {
        let mut zone = MemoryZone::new(name.clone());
        let file = File::open(path)?;
        let file = BufReader::new(file);
        for line in file.lines() {
            let line = line?;
            let rrset = RRset::from_str(line.as_ref())?;
            zone.add_rrset(rrset)?;
        }
        self.zones.insert(name, Some(zone));
        Ok(())
    }

    pub fn handle_query(&self, mut req: Message) -> Message {
        let result = self.zones.find(&req.question.name);
        let zone = result.get_value();
        if zone.is_none() {
            let mut builder = MessageBuilder::new(&mut req);
            builder.make_response().rcode(Rcode::Refused).done();
            return req;
        }

        let zone = zone.unwrap();
        let mut result = zone.find(
            &req.question.name,
            req.question.typ,
            FindOption::FollowZoneCut,
        );

        let query_type = req.question.typ;
        let mut builder = MessageBuilder::new(&mut req);
        builder.make_response().set_flag(HeaderFlag::AuthAnswer);
        match result.typ {
            FindResultType::CName => {
                builder.add_answer(result.rrset.take().unwrap());
            }
            FindResultType::Success => {
                for rrset in result.get_additional() {
                    builder.add_additional(rrset);
                }
                builder.add_answer(result.rrset.take().unwrap());
                if query_type != RRType::NS {
                    let (auth, additional) = get_auth_and_additional(zone);
                    builder.add_auth(auth);
                    for rrset in additional {
                        builder.add_additional(rrset);
                    }
                }
            }
            FindResultType::Delegation => {
                for rrset in result.get_additional() {
                    builder.add_additional(rrset);
                }
                builder
                    .clear_flag(HeaderFlag::AuthAnswer)
                    .add_auth(result.rrset.take().unwrap());
            }
            FindResultType::NXDomain => {
                builder.rcode(Rcode::NXDomian).add_auth(get_soa(zone));
            }
            FindResultType::NXRRset => {
                builder.rcode(Rcode::NXRRset).add_auth(get_soa(zone));
            }
        }
        builder.done();
        req
    }
}

impl Future for Auth {
    type Item = ();
    type Error = std::io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        loop {
            if let Some(peer) = self.current_user {
                let _amt = try_ready!(self.socket.poll_send_to(self.render.data(), &peer));
                self.current_user = None;
            }

            let (_, client) = try_ready!(self.socket.poll_recv_from(&mut self.buf));
            let message = Message::from_wire(self.buf.as_slice());
            if message.is_err() {
                continue;
            }
            let message = message.unwrap();
            let resp = self.handle_query(message);
            self.render.clear();
            resp.rend(&mut self.render);
            self.current_user = Some(client);
        }
    }
}

fn get_auth_and_additional(zone: &MemoryZone) -> (RRset, Vec<RRset>) {
    let mut address = Vec::new();
    let mut result = zone.find(zone.get_origin(), RRType::NS, FindOption::FollowZoneCut);
    let ns = result.rrset.take().unwrap();
    for rdata in &ns.rdatas {
        if let RData::NS(ns) = rdata {
            address.append(&mut result.get_address(&ns.name));
        }
    }
    (ns, address)
}

fn get_soa(zone: &MemoryZone) -> RRset {
    let mut result = zone.find(zone.get_origin(), RRType::SOA, FindOption::FollowZoneCut);
    result.rrset.take().unwrap()
}

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

    let addr = matches.value_of("server").unwrap().to_string();
    let addr = addr.parse::<SocketAddr>().unwrap();
    let socket = UdpSocket::bind(&addr).unwrap();
    println!("Listening on: {}", socket.local_addr().unwrap());

    let zone_file = matches.value_of("zonefile").unwrap().to_string();
    let zone_name = matches.value_of("zonename").unwrap().to_string();

    let mut auth = Auth::new(socket);
    auth.load_zone(Name::new(zone_name.as_str()).unwrap(), zone_file.as_str())
        .unwrap();
    tokio::run(auth.map_err(|e| println!("server error = {:?}", e)));
}
