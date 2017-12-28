extern crate bytes;
extern crate env_logger;
extern crate http;
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

use futures::Future;
use tokio_core::reactor::Core;
use tokio_core::net::TcpStream;
use tower_grpc::Request;
use tower_h2::client::Connection;

pub mod pb {
    include!(concat!(env!("OUT_DIR"), "/grpc.testing.rs"));
}

pub fn main() {
    let _ = ::env_logger::init();

    let mut core = Core::new().unwrap();
    let reactor = core.handle();

    let addr = "[::1]:10000".parse().unwrap();
    let uri: http::Uri = format!("http://localhost:10000").parse().unwrap();

    let rpc = TcpStream::connect(&addr, &reactor)
        .and_then(move |socket| {
            // Bind the HTTP/2.0 connection
            Connection::handshake(socket, reactor)
                .map_err(|_| panic!("failed HTTP/2.0 handshake"))
        })
        .and_then(move |conn| {
            use pb::client::TestService;
            Ok(TestService::new(conn, uri).unwrap())
        })
        .and_then(|mut client| {
            use pb::Empty;
            client.empty_call(Request::new(Empty{}))
                .map_err(|e| panic!("gRPC request failed; err={:?}", e))
        })
        .and_then(move |response| {
            println!("RESPONSE = {:?}", response.into_inner());
            Ok(())
        })
        .map_err(|e| {
            println!("ERR = {:?}", e);
        });

    core.run(rpc).unwrap();
}