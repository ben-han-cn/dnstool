extern crate bytes;
extern crate futures;
extern crate tokio;
extern crate tokio_io;

use std::net::SocketAddr;

use bytes::Bytes;
use std::collections::HashMap;
use std::io::{Error, ErrorKind};
use std::sync::{Arc, Mutex};

use futures::future::lazy;
use futures::prelude::*;
use futures::sync::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use tokio::codec::*;
use tokio::executor;
use tokio::net::{TcpListener, TcpStream};
use tokio::runtime;

pub struct Client {
    id: u32,
    receiver: UnboundedReceiver<Bytes>,
    stream: Framed<TcpStream, LengthDelimitedCodec>,

    pub register: Option<Arc<Mutex<Register>>>,
}

pub struct Register {
    next_id: u32,
    publisher: UnboundedSender<(u32, Bytes)>,
    clients: HashMap<u32, UnboundedSender<Bytes>>,

    messageCh: Option<UnboundedReceiver<(u32, Bytes)>>,
}

pub struct Broadcaster {
    messageCh: UnboundedReceiver<(u32, Bytes)>,

    pub register: Option<Arc<Mutex<Register>>>,
}

impl Register {
    pub fn new() -> Register {
        let (sender, receiver) = unbounded::<(u32, Bytes)>();
        Register {
            next_id: 1,
            publisher: sender,
            clients: HashMap::new(),
            messageCh: Some(receiver),
        }
    }

    pub fn add_client(&mut self, stream: TcpStream) -> Client {
        let (sender, receiver) = unbounded::<Bytes>();
        self.clients.insert(self.next_id, sender);
        let client = Client {
            id: self.next_id,
            stream: length_delimited::Builder::new()
                .length_field_length(2)
                .new_framed(stream),
            receiver: receiver,
            register: None,
        };
        self.next_id += 1;
        client
    }

    pub fn gen_broadcaster(&mut self) -> Broadcaster {
        Broadcaster {
            messageCh: self.messageCh.take().unwrap(),
            register: None,
        }
    }
}

impl Client {
    pub fn into_future(
        mut self,
    ) -> (
        impl Future<Item = (), Error = ()>,
        impl Future<Item = (), Error = ()>,
    ) {
        let id = self.id;
        let register = self.register.take().unwrap();
        let (sink, reader) = self.stream.split();
        let receiver = self
            .receiver
            .map_err(|_| Error::new(ErrorKind::Other, "oh no!"));
        let s1 = receiver.forward(sink).map_err(|_| ()).map(|_| ());
        let s2 = reader
            .for_each(move |data| {
                let broadcaster = register.lock().unwrap();
                broadcaster.publisher.unbounded_send((id, data.freeze()));
                Ok(())
            }).map_err(|_| ());
        (s1, s2.into_future())
    }
}

impl Stream for Client {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Option<()>, ()> {
        loop {
            let data = match self.stream.poll() {
                Ok(Async::Ready(Some(data))) => data,
                Ok(Async::Ready(None)) => return Ok(Async::Ready(None)),
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(err) => return Err(()),
            };

            {
                self.register.as_ref().map(|register| {
                    let broadcaster = register.lock().unwrap();
                    broadcaster
                        .publisher
                        .unbounded_send((self.id, data.freeze()));
                });
            }
        }
    }
}

impl Stream for Broadcaster {
    type Item = ();
    type Error = ();

    fn poll(&mut self) -> Poll<Option<()>, ()> {
        loop {
            let (id, data) = match self.messageCh.poll() {
                Ok(Async::Ready(Some(data))) => data,
                Ok(Async::Ready(None)) => return Ok(Async::Ready(None)),
                Ok(Async::NotReady) => return Ok(Async::NotReady),
                Err(err) => return Err(()),
            };

            {
                self.register.as_ref().map(|register| {
                    register
                        .lock()
                        .unwrap()
                        .clients
                        .iter()
                        .filter(|(id_, _)| **id_ != id)
                        .for_each(|(_, sender)| {
                            sender.unbounded_send(data.clone());
                        });
                });
            }
        }
    }
}
