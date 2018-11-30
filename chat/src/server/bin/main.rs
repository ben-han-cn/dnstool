extern crate futures;
extern crate tokio;
extern crate tokio_io;
extern crate bytes;

mod broadcaster;
use std::net::SocketAddr;

use std::sync::{Arc, Mutex};
use futures::future::lazy;
use futures::prelude::*;
use tokio::codec::*;
use tokio::executor;
use tokio::net::TcpListener;
use tokio::runtime;
use broadcaster::{Broadcaster, Client, Register};

fn tcp_listener(addr: &str) -> (TcpListener, SocketAddr) {
    let addr = addr.parse::<SocketAddr>().unwrap();
    let listener = TcpListener::bind(&addr).expect("unable to open TCP listener");
    (listener, addr)
}

fn start_server(addr: &str) {
    let rt = runtime::Builder::new()
        .core_threads(3)
        .name_prefix("chat_server")
        .build()
        .unwrap();

    let mut register = Arc::new(Mutex::new(Register::new()));
    let (listener, _) = tcp_listener(addr);

    let register1 = register.clone();
    rt.executor().spawn(
        listener
            .incoming()
            .for_each(move |stream| {
                let mut client = register1.lock().unwrap().add_client(stream);
                client.register = Some(register1.clone());
                //rt.executor().spawn(client.for_each(|_| Ok(())));
                let (c1, c2) = client.into_future();
                executor::spawn(c1);
                executor::spawn(c2);
                Ok(())
            }).map_err(|_| ()),
    );

    let mut broadcaster = register.lock().unwrap().gen_broadcaster();
    broadcaster.register = Some(register.clone());
    rt.executor().spawn(broadcaster.for_each(|_| Ok(())));
    rt.shutdown_on_idle().wait().unwrap();
}

fn main() {
    start_server("127.0.0.1:5555");
}
