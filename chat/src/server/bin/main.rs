extern crate futures;
extern crate tokio;
extern crate tokio_io;

use std::net::SocketAddr;

use futures::future::lazy;
use futures::prelude::*;
use tokio::codec::*;
use tokio::executor;
use tokio::net::TcpListener;
use tokio::runtime;

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

    let (listener, _) = tcp_listener(addr);
    rt.executor().spawn(
        listener
            .incoming()
            .for_each(move |stream| {
                executor::spawn(lazy(move || {
                    let transport = length_delimited::Builder::new()
                        .length_field_length(2)
                        .new_framed(stream);
                    transport
                        .for_each(|frame| {
                            println!(
                                "Received frame {}",
                                String::from_utf8_lossy(frame.as_ref()).trim_end()
                            );
                            Ok(())
                        }).map_err(|e| println!("----> get err {:?}", e))
                }));
                Ok(())
            }).map_err(|_| ()),
    );
    rt.shutdown_on_idle().wait().unwrap();
}

fn main() {
    start_server("127.0.0.1:5555");
}
