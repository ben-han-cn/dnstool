use std::net::SocketAddr;
use r53::Message;

#[derive (Debug)]
pub struct Query {
    pub query: Message,
    pub resp: Option<Message>,
    pub client: SocketAddr,
}

impl Query {
    pub fn new(query: Message, client: SocketAddr) -> Self {
        Query {
            query: query,
            resp: None,
            client: client,
        }
    }

    pub fn set_response(&mut self, resp: Message) {
        self.resp = Some(resp);
    }
}
