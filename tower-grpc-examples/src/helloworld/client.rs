extern crate bytes;
extern crate env_logger;
extern crate http;
extern crate futures;
extern crate log;
extern crate prost;
extern crate tokio;
extern crate tower_h2;
extern crate tower_add_origin;
extern crate tower_grpc;
extern crate tower_service;
extern crate tower_util;

use futures::{Future, Poll};
use tokio::executor::DefaultExecutor;
use tokio::net::tcp::{ConnectFuture, TcpStream};
use tower_grpc::Request;
use tower_h2::client;
use tower_service::Service;
use tower_util::MakeService;

pub mod hello_world {
    include!(concat!(env!("OUT_DIR"), "/helloworld.rs"));
}

pub fn main() {
    let _ = ::env_logger::init();

    let uri: http::Uri = format!("http://localhost:50051").parse().unwrap();

    let h2_settings = Default::default();
    let mut make_client = client::Connect::new(Dst, h2_settings, DefaultExecutor::current());

    let say_hello = make_client.make_service(())
        .map(move |conn| {
            use hello_world::client::Greeter;

            let conn = tower_add_origin::Builder::new()
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

    tokio::run(say_hello);
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
        TcpStream::connect(&([127, 0, 0, 1], 50051).into())
    }
}

