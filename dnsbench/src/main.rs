extern crate futures;
extern crate tokio;
extern crate r53;
extern crate clap; 
extern crate tokio_timer;

use clap::{App, Arg};
use tokio::prelude::*;
use r53::Name;
use std::io::{BufRead, BufReader};
use std::fs::File;
use tokio::runtime::current_thread;

mod query_gen;
use query_gen::QueryStream;

fn get_names(path: &str) -> Vec<Name> {
    let file = File::open(path).expect("open file failed");
    let file = BufReader::new(file);
    file.lines().filter_map(|l| l.ok()).map(|line| {
        Name::new(&line, false).unwrap()
    }).collect()
}

fn main() {
    let matches = App::new("dnsbench")
        .arg(Arg::with_name("server")
             .help("server address")
             .short("s")
             .long("server")
             .required(true)
             .takes_value(true))
        .arg(Arg::with_name("file")
             .help("file which includes names to query")
             .short("f")
             .long("file")
             .takes_value(true))
        .arg(Arg::with_name("qps")
             .help("qps")
             .short("q")
             .long("qps")
             .takes_value(true))
        .get_matches();
    let server = matches.value_of("server").unwrap();
    let file = matches.value_of("file").unwrap();
    let qps = matches.value_of("qps").unwrap_or("1000").parse::<u32>().unwrap();
    let stream: Box<Future<Item=(), Error=()> + Send> = Box::new(QueryStream::new(server, qps, get_names(file))
        .then(|r| 
              match r {
                  Ok(r) => Ok(Some(r)),
                  Err(_) => Ok(Some(0)),
              })
        .for_each(|n| {
            println!("send to {}", n.unwrap());
            Ok(())
        }));
    current_thread::block_on_all(stream).unwrap();
}
