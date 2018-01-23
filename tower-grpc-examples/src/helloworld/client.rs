extern crate bytes;
extern crate env_logger;
extern crate http;
extern crate futures;
extern crate log;
extern crate prost;
#[macro_use]
extern crate prost_derive;
extern crate tokio_core;
extern crate tower_h2;
extern crate tower_http;
extern crate tower_grpc;

use futures::Future;
use tokio_core::reactor::Core;
use tokio_core::net::TcpStream;
use tower_grpc::Request;
use tower_h2::client::Connection;

pub mod hello_world {
    include!(concat!(env!("OUT_DIR"), "/helloworld.rs"));
}

pub fn main() {
    let _ = ::env_logger::init();

    let mut core = Core::new().unwrap();
    let reactor = core.handle();

    let addr = "[::1]:50051".parse().unwrap();
    let uri: http::Uri = format!("http://localhost:50051").parse().unwrap();

    let say_hello = TcpStream::connect(&addr, &reactor)
        .and_then(move |socket| {
            // Bind the HTTP/2.0 connection
            Connection::handshake(socket, reactor)
                .map_err(|_| panic!("failed HTTP/2.0 handshake"))
        })
        .map(move |conn| {
            use hello_world::client::Greeter;
            use tower_http::add_origin;

            let conn = add_origin::Builder::new()
                .uri(uri)
                .build(conn)
                .unwrap();

            Greeter::new(conn)
        })
        .and_then(|mut client| {
            use hello_world::HelloRequest;

            client.say_hello(Request::new(HelloRequest {
                name: "What is in a name?".to_string(),
            })).map_err(|e| panic!("gRPC request failed; err={:?}", e))
        })
        .and_then(|response| {
            println!("RESPONSE = {:?}", response);
            Ok(())
        })
        .map_err(|e| {
            println!("ERR = {:?}", e);
        });

    core.run(say_hello).unwrap();
}
