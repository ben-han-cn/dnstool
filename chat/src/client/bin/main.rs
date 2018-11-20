extern crate tokio;
extern crate tokio_io;
extern crate bytes;
extern crate futures;

use std::net::SocketAddr;
use std::thread;
use std::io::{self, Read};

use tokio::codec::*;
use bytes::{Bytes};
use tokio::net::TcpStream;
use futures::future::lazy;
use futures::prelude::*;
use futures::sync::mpsc;
use std::io::{Error, ErrorKind};

fn read_stdin(mut tx: mpsc::Sender<bytes::Bytes>) {
    let mut stdin = io::stdin();
    loop {
        let mut buf = vec![0; 1024];
        let n = match stdin.read(&mut buf) {
            Err(_) | Ok(0) => break,
            Ok(n) => n,
        };
        buf.truncate(n);
        tx = match tx.send(Bytes::from(buf)).wait() {
            Ok(tx) => tx,
            Err(_) => break,
        };
    }
}

fn main() {
    let addr = "127.0.0.1:5555".parse::<SocketAddr>().unwrap();
    let (stdin_tx, stdin_rx) = mpsc::channel(0);
    thread::spawn(|| read_stdin(stdin_tx));
    let stdin_rx = stdin_rx.map_err(|_| Error::new(ErrorKind::Other, "oh no!"));

    let stream = match TcpStream::connect(&addr).wait() {
        Ok(e) => e,
        Err(e) => panic!("connect server failed {}", e),
    };

    tokio::run(lazy(move || {
        let (sink, _) = length_delimited::Builder::new().length_field_length(2).new_framed(stream).split();
        stdin_rx.forward(sink).map_err(|_| ()).map(|_| ())
    }));
}
