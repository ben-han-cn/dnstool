use r53::{Name, Message, MessageRender, RRType};
use futures::{Async, Future, Stream, Poll};
use std::time::{Instant, Duration};
use tokio_timer::Delay;
use tokio::net::UdpSocket;
use std::net::SocketAddr;

pub struct QueryGenerator {
    names: Vec<Name>,
    index: usize,
}

impl QueryGenerator {
    pub fn new(names: Vec<Name>) -> Self {
        QueryGenerator {
            names: names,
            index: 0,
        }
    }

    pub fn generate_query(&mut self) -> Message {
        if self.index == self.names.len() {
            self.index = 0;
        }
        let message = Message::with_query(self.names[self.index].clone(), RRType::A);
        self.index += 1;
        message
    }
}

pub struct LimitedQueryGen {
    qps: u32,
    gen: QueryGenerator,
    state: State,
    next: Instant,
}

enum State {
    GenQuery(u32),
    WaitUntil(Delay),
}

impl LimitedQueryGen {
    pub fn new(qps: u32, names: Vec<Name>) -> Self {
        LimitedQueryGen {
            qps: qps,
            gen: QueryGenerator::new(names),
            next: Instant::now() + Duration::new(1, 0),
            state: State::GenQuery(qps),
        }
    }

    pub fn poll_message(&mut self) -> Option<Message> {
        let mut next_round = false;
        match self.state {
            State::GenQuery(left) => {
                if left == 0 {
                    let now = Instant::now();
                    if now < self.next {
                        self.state = State::WaitUntil(Delay::new(self.next));
                        return None;
                    } else {
                        self.next = self.next + Duration::new(1, 0);
                        self.state = State::GenQuery(self.qps);
                    }
                } else {
                   self.state = State::GenQuery(left-1);
                }
            },
            State::WaitUntil(ref mut timeout) => {
                match timeout.poll() {
                    Ok(Async::NotReady) => { return None; }, 
                    _ => { next_round = true; },
                }
            }
        }

        if next_round {
            self.next = Instant::now() + Duration::new(1, 0);
            self.state = State::GenQuery(self.qps);
        }
        Some(self.gen.generate_query())
    }
}

pub struct QueryStream {
    gen: LimitedQueryGen,
    target: SocketAddr,
    socket: UdpSocket,
    render: MessageRender,
}

impl QueryStream {
    pub fn new(target: &str, qps: u32, names: Vec<Name>) -> QueryStream {
        QueryStream {
            gen: LimitedQueryGen::new(qps, names),
            target: target.parse::<SocketAddr>().unwrap(),
            socket: UdpSocket::bind(&("0.0.0.0:0".parse::<SocketAddr>().unwrap())).unwrap(),
            render: MessageRender::new(),
        }
    }
}

impl Stream for QueryStream {
    type Item = u32;
    type Error = ();

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let message = self.gen.poll_message();
        if message.is_none() {
            return Ok(Async::NotReady);
        }

        message.unwrap().rend(&mut self.render);
        self.socket.poll_send_to(self.render.data(), &self.target).map_err(|_| ())?;
        let len = self.render.data().len();
        self.render.clear();
        Ok(Async::Ready(Some(len as u32)))
    }
}
