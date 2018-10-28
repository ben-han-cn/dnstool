use std::io;

use futures::stream::{Fuse, Peekable, Stream};
use futures::sync::mpsc::{channel, Sender, Receiver};
use futures::{Async, Future, Poll};
use futures::sink::Sink;
use r53::{MessageRender, Message};
use tokio::net::UdpSocket;
use tokio::executor::spawn;

use query::Query;

pub trait DNSHandler {
    fn handle_query(&mut self, query: Query) -> Box<Future<Item = Query, Error = io::Error> + Send>;
}

pub struct UdpStream<H: DNSHandler> {
    socket: UdpSocket,
    sender: Sender<Query>,
    response_ch: Peekable<Fuse<Receiver<Query>>>,
    handler: H,
}

impl<H: DNSHandler> UdpStream<H> {
    pub fn new(socket: UdpSocket, handler: H) -> Self {
        let (sender, response_ch) = channel(1024);
        UdpStream {
            socket: socket,
            sender: sender,
            response_ch: response_ch.fuse().peekable(),
            handler: handler,
        }
    }

    fn send_all_response(&mut self) -> Poll<(), io::Error> {
        let mut render = MessageRender::new();
        loop {
            match self.response_ch.peek() {
                Ok(Async::Ready(Some(query))) => { 
                    query.resp.as_ref().map(|resp| resp.rend(&mut render));
                    try_ready!(self.socket.poll_send_to(render.data(), &query.client));
                    render.clear();
                }
                Ok(Async::Ready(None)) | Ok(Async::NotReady) => return Ok(Async::Ready(())),
                Err(_) => panic!("get error form channel"),
            }

            match self.response_ch.poll() {
                Err(_) => panic!("get error when poll response"),
                Ok(Async::NotReady) => return Ok(Async::Ready(())),
                _ => (),
            }
        }
    }
}

impl<H: DNSHandler> Future for UdpStream<H> {
    type Item = ();
    type Error = io::Error;

    fn poll(&mut self) -> Poll<(), Self::Error> {
        let mut buf = [0u8; 1024];
        loop {
            try_ready!(self.send_all_response());
            let (size, src) = try_ready!(self.socket.poll_recv_from(&mut buf));
            let sender = self.sender.clone();
            let query = Query::new(Message::from_wire(&buf[..size]).unwrap(), src);
            spawn(self.handler.handle_query(query).and_then(move |query| {
                sender.send(query).map_err(|_| {
                    io::Error::new(io::ErrorKind::Other, "unknown")
                })
            }).map(|_| ()).map_err(|e| {
                println!("get error:{}", e); 
                ()
            }));
        }
    }
}
