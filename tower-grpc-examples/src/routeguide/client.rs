#![allow(dead_code)]
#![allow(unused_variables)]

extern crate bytes;
extern crate env_logger;
extern crate futures;
extern crate http;
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

use futures::Future;
use tokio_core::reactor::Core;
use tokio_core::net::TcpStream;
use tower_grpc::Request;
use tower_h2::client::Connection;

use routeguide::Point;

mod data;
pub mod routeguide {
    include!(concat!(env!("OUT_DIR"), "/routeguide.rs"));
}

pub fn main() {
    let _ = ::env_logger::init();

    let mut core = Core::new().unwrap();
    let reactor = core.handle();

    let addr = "127.0.0.1:10000".parse().unwrap();
    let uri: http::Uri = format!("http://localhost:10000").parse().unwrap();

    let mut client = core.run({
        TcpStream::connect(&addr, &reactor)
            .and_then(move |socket| {
                // Bind the HTTP/2.0 connection
                Connection::handshake(socket, reactor)
                    .map_err(|_| panic!("failed HTTP/2.0 handshake"))
            })
            .and_then(move |conn| {
                use routeguide::client::RouteGuide;
                Ok(RouteGuide::new(conn, uri).unwrap())
            })
    }).unwrap();

    let response = core.run({
        client.get_feature(Request::new(Point {
            latitude: 409146138,
            longitude: -746188906,
        }))
    }).unwrap();

    println!("GetFeature Response = {:?}", response);
}
