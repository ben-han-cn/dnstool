#[macro_use]
extern crate futures;
extern crate tokio;
extern crate r53;
extern crate clap; 
extern crate rand;

use std::io;
use std::net::SocketAddr;

use clap::{App, Arg};
use tokio::prelude::*;
use tokio::net::UdpSocket;
use r53::{Message, MessageRender};
use rand::prelude::*;

mod message_gen;
use message_gen::MessageGenerator;

struct Server {
    socket: UdpSocket,
    buf: Vec<u8>,
    render: MessageRender,
    to_send: Option<SocketAddr>,
    generator: MessageGenerator,
    random_delay: bool,
}


impl Future for Server {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        let mut rand_gen = thread_rng();
        loop {
            if let Some(peer) = self.to_send {
                let _amt = try_ready!(self.socket.poll_send_to(self.render.data(), &peer));
                self.render.clear();
                self.to_send = None;
            }

            let (size, client) = try_ready!(self.socket.poll_recv_from(&mut self.buf));
            if self.random_delay {
                let value: u32 = rand_gen.gen_range(0, 10000);
                if value <= 10 {
                    continue
                }
            } 

            let mut query = Message::from_wire(&self.buf[..size]).unwrap();
            self.generator.gen_message(&mut query, &mut rand_gen);
            query.rend(&mut self.render);
            self.to_send = Some(client);
        }
    }
}

fn main() {
    let matches = App::new("snamed")
        .arg(Arg::with_name("server")
             .help("server address")
             .short("s")
             .long("server")
             .required(true)
             .takes_value(true))
        .arg(Arg::with_name("next")
             .help("next server address")
             .short("n")
             .long("next")
             .takes_value(true))
        .arg(Arg::with_name("delay")
             .help("random delay")
             .short("d")
             .long("delay"))
        .arg(Arg::with_name("level")
             .help("name hierachy level")
             .short("l")
             .long("level")
             .required(true)
             .takes_value(true))
        .get_matches();
    let mut addr = matches.value_of("server").unwrap().to_string();
    addr.push_str(":53");
    let addr = addr.parse::<SocketAddr>().unwrap();

    let socket = UdpSocket::bind(&addr).unwrap();
    println!("Listening on: {}", socket.local_addr().unwrap());

    let level = matches.value_of("level").unwrap().parse::<u8>().unwrap();
    let server = Server {
        generator: MessageGenerator::new(level, matches.value_of("next").map(|s| s.to_string())),
        socket: socket,
        buf: vec![0; 1024],
        render: MessageRender::new(),
        to_send: None,
        random_delay: matches.is_present("delay"),
    };
    tokio::run(server.map_err(|e| println!("server error = {:?}", e)));
}
