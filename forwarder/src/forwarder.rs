use std::io::{self, Error, ErrorKind};
use std::net::SocketAddr;

use tokio::net::{UdpSocket};
use r53::{Message, MessageRender};
use udp_stream::DNSHandler;
use query::Query;
use futures::{Future};
use std::time::Duration;
use tokio::util::FutureExt;

pub struct Forwarder {
    target: SocketAddr,
}


impl Forwarder {
    pub fn new(target: SocketAddr) -> Self {
        Forwarder {
            target: target,
        }
    }
}

impl DNSHandler for Forwarder {
    fn handle_query(&mut self, query: Query) -> Box<Future<Item = Query, Error = io::Error> + Send> {
        let mut render = MessageRender::new();
        query.query.rend(&mut render);
        let socket = UdpSocket::bind(&("0.0.0.0:0".parse::<SocketAddr>().unwrap())).unwrap();
        let mut query = query;
        Box::new(
                socket.send_dgram(render.take_data(), &self.target).and_then(|(socket, _)| {
                    socket.recv_dgram(vec![0; 1024])
                })
                .timeout(Duration::from_secs(3))
                .map_err(|_e| {
                    Error::new(ErrorKind::TimedOut, "send timeout")
                })
                .map(move |(_, buf, size, _)| {
                    let resp = Message::from_wire(&buf[..size]).unwrap();
                    query.set_response(resp);
                    query
                })
            )
    }
}
