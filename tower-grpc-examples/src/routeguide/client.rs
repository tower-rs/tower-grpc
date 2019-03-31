#![allow(dead_code)]
#![allow(unused_variables)]

extern crate bytes;
extern crate env_logger;
extern crate futures;
extern crate http;
extern crate log;
extern crate prost;
extern crate tokio;
extern crate tower_add_origin;
extern crate tower_grpc;
extern crate tower_h2;
extern crate tower_service;
extern crate tower_util;

extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

use futures::{Future, Poll};
use tokio::executor::DefaultExecutor;
use tokio::net::tcp::{ConnectFuture, TcpStream};
use tower_grpc::Request;
use tower_h2::client;
use tower_service::Service;
use tower_util::MakeService;

use routeguide::Point;

mod data;
pub mod routeguide {
    include!(concat!(env!("OUT_DIR"), "/routeguide.rs"));
}

pub fn main() {
    let _ = ::env_logger::init();

    let uri: http::Uri = format!("http://localhost:10000").parse().unwrap();

    let h2_settings = Default::default();
    let mut make_client = client::Connect::new(Dst, h2_settings, DefaultExecutor::current());

    let rg = make_client
        .make_service(())
        .map(move |conn| {
            use routeguide::client::RouteGuide;

            let conn = tower_add_origin::Builder::new()
                .uri(uri)
                .build(conn)
                .unwrap();

            RouteGuide::new(conn)
        })
        .and_then(|mut client| {
            client
                .get_feature(Request::new(Point {
                    latitude: 409146138,
                    longitude: -746188906,
                }))
                .map_err(|e| panic!("gRPC request failed; err={:?}", e))
        })
        .map(|response| {
            println!("RESPONSE = {:?}", response);
        })
        .map_err(|e| {
            println!("ERR = {:?}", e);
        });

    tokio::run(rg);
}

struct Dst;

impl Service<()> for Dst {
    type Response = TcpStream;
    type Error = ::std::io::Error;
    type Future = ConnectFuture;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(().into())
    }

    fn call(&mut self, _: ()) -> Self::Future {
        TcpStream::connect(&([127, 0, 0, 1], 10000).into())
    }
}
