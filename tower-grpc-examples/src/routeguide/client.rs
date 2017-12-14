#![allow(dead_code)]
#![allow(unused_variables)]

extern crate bytes;
extern crate env_logger;
extern crate futures;
#[macro_use]
extern crate log;
extern crate prost;
#[macro_use]
extern crate prost_derive;
extern crate tokio_core;
extern crate tower;
extern crate tower_h2;
extern crate tower_grpc;

extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

mod data;
pub mod routeguide {
    include!(concat!(env!("OUT_DIR"), "/routeguide.rs"));
}

pub fn main() {
    unimplemented!();
}
