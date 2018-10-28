#[macro_use]
extern crate futures;
extern crate tokio;
extern crate r53;
extern crate clap; 

use std::net::SocketAddr;

use clap::{App, Arg};
use tokio::prelude::*;
use tokio::net::UdpSocket;

mod forwarder;
mod query;
mod udp_stream;
use forwarder::Forwarder;
use udp_stream::UdpStream;

fn main() {
    let default_forwarder = "114.114.114.114:53";
    let matches = App::new("forwarder")
        .arg(Arg::with_name("server")
             .help("server address")
             .short("s")
             .long("server")
             .required(true)
             .takes_value(true))
        .arg(Arg::with_name("forwarder")
             .help("forward query to server")
             .short("f")
             .long("forwarder")
             .takes_value(true))
        .get_matches();
    let addr = matches.value_of("server").unwrap().parse::<SocketAddr>().unwrap();

    let socket = UdpSocket::bind(&addr).unwrap();
    println!("Listening on: {}", socket.local_addr().unwrap());

    let forwarder = match matches.value_of("forwarder") {
        Some(addr) => {
            let mut addr = addr.to_string();
            addr.push_str(":53");
            addr
        },
        None => default_forwarder.to_owned()
    };
    let forwarder = Forwarder::new(forwarder.parse::<SocketAddr>().unwrap());
    let stream = UdpStream::new(socket, forwarder);
    tokio::run(stream.map_err(|e| println!("server error = {:?}", e)));
}
