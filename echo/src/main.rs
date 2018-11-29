#[macro_use]
extern crate futures;
extern crate tokio;
extern crate clap; 
extern crate tokio_timer;

use std::io;
use std::net::SocketAddr;

use clap::{App, Arg};
use tokio::prelude::*;
use tokio::net::UdpSocket;
use std::time::{Duration};
use tokio_timer::Interval;
use std::sync::{Arc, Mutex};
use tokio::executor;

struct Server {
    socket: UdpSocket,
    buf: Vec<u8>,
    size: usize,
    to_send: Option<SocketAddr>,
    qps: Arc<Mutex<u32>>,
    scheduled: bool,
}

impl Server {
    pub fn new(socket: UdpSocket, qps: Arc<Mutex<u32>>) -> Self {
        Server {
            socket: socket,
            buf: vec![0; 1024],
            size: 0,
            to_send: None,
            qps: qps,
            scheduled: false,
        }
    }
}

impl Future for Server {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), io::Error> {
        if self.scheduled == false {
            let qps = self.qps.clone();
            executor::spawn(
                Interval::new_interval(Duration::from_secs(1))
                .map_err(|_| ())
                .and_then(move |_| {
                    let mut qps = qps.lock().unwrap();
                    println!("qps: {}", *qps);
                    *qps = 0;
                    Ok(())
                })
                .for_each(|_| Ok(()))
            );
            self.scheduled = true;
        }

        loop {
            if let Some(peer) = self.to_send {
                let _amt = try_ready!(self.socket.poll_send_to(&self.buf[..self.size], &peer));
                self.to_send = None;
                self.size = 0;
            }

            let (size, client) = try_ready!(self.socket.poll_recv_from(&mut self.buf));
            self.size = size;
            self.to_send = Some(client);
            {
                let mut qps = self.qps.lock().unwrap();
                *qps += 1;
            }
        }
    }
}

fn main() {
    let matches = App::new("echo")
        .arg(Arg::with_name("server")
             .help("server address")
             .short("s")
             .long("server")
             .required(true)
             .takes_value(true))
        .get_matches();
    let addr = matches.value_of("server").unwrap().to_string();
    let addr = addr.parse::<SocketAddr>().unwrap();

    let socket = UdpSocket::bind(&addr).unwrap();
    println!("Listening on: {}", socket.local_addr().unwrap());

    let qps = Arc::new(Mutex::new(0));
    let server = Server::new(socket, qps);
    tokio::run(server.map_err(|e| println!("server error = {:?}", e)));
}
