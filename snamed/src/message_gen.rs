use r53::{Message, MessageBuilder, Name, RRset, RRType, RRClass, RData, RRTtl, A, NS};
use rand::prelude::*;

pub struct MessageGenerator {
    level: u8,
    next_server: Option<String>,
}

impl MessageGenerator {
    pub fn new(level: u8, next_server: Option<String>) -> Self {
        MessageGenerator {
            level: level,
            next_server: next_server,
        }
    }

    pub fn gen_message(&self, query: &mut Message, rand_gen: &mut ThreadRng) {
        let required_level = query.question.name.label_count - 1;
        let ttl = rand_gen.gen_range::<u32>(10, 500);
        if required_level > (self.level as u8) && self.next_server.is_some() {
            let zone = query.question.name.parent((required_level - self.level) as usize).unwrap();
            let ns_count = rand_gen.gen_range::<u8>(1,4);
            self.gen_referal(query, zone, ns_count, ttl)
        } else {
            self.gen_direct_answer(query, ttl)
        }
    }

    fn gen_referal(&self, resp: &mut Message, zone: Name, ns_count: u8, ttl: u32) {
        let next_server = self.next_server.as_ref().map(|s| s.as_ref()).unwrap();

        let mut builder = MessageBuilder::new(resp);
        for i in 0..ns_count {
            let mut ns_name = zone.to_string();
            ns_name.insert_str(0, format!("ns{}.", i+1).as_ref());
            builder.
                add_auth(
                    RRset {
                        name: zone.clone(),
                        typ: RRType::NS,
                        class: RRClass::IN,
                        ttl: RRTtl(ttl),
                        rdatas: [RData::NS(Box::new(NS::from_string(ns_name.as_ref()).unwrap()))].to_vec(),
                    }).add_additional(
                        RRset {
                            name: Name::new(ns_name.as_ref(), false).unwrap(),
                            typ: RRType::A,
                            class: RRClass::IN,
                            ttl: RRTtl(ttl),
                            rdatas: [RData::A(A::from_string(next_server).unwrap())].to_vec(),
                        }).make_response().done();
        }
    }

    fn gen_direct_answer(&self, resp: &mut Message, ttl: u32) {
        let qname = resp.question.name.clone();
        let mut builder = MessageBuilder::new(resp);
        builder.add_answer(
            RRset {
                name: qname,
                typ: RRType::A,
                class: RRClass::IN,
                ttl: RRTtl(ttl),
                rdatas: [RData::A(A::from_string("192.0.2.2").unwrap()), RData::A(A::from_string("192.0.2.1").unwrap())].to_vec(),
            }).make_response().done();
    }
}
