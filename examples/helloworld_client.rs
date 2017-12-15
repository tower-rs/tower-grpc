extern crate env_logger;
extern crate futures;
extern crate http;
#[macro_use]
extern crate prost_derive;
extern crate tokio_connect;
extern crate tokio_core;
extern crate tower;
extern crate tower_grpc;
extern crate tower_h2;

use futures::Future;
use tokio_connect::Connect;
use tokio_core::net::TcpStream;
use tokio_core::reactor::{Core, Handle};
use tower::NewService;
use tower_grpc::codegen::client::*;

use std::net::SocketAddr;

struct Conn(SocketAddr, Handle);

impl Connect for Conn {
    type Connected = TcpStream;
    type Error = ::std::io::Error;
    type Future = Box<Future<Item = TcpStream, Error = ::std::io::Error>>;

    fn connect(&self) -> Self::Future {
        let c = TcpStream::connect(&self.0, &self.1)
            .and_then(|tcp| tcp.set_nodelay(true).map(move |_| tcp));
        Box::new(c)
    }
}

/// The request message containing the user's name.
#[derive(Clone, Debug, PartialEq, Message)]
pub struct HelloRequest {
    #[prost(string, tag="1")]
    pub name: String,
}

/// The response message containing the greetings
#[derive(Clone, Debug, PartialEq, Message)]
pub struct HelloReply {
    #[prost(string, tag="1")]
    pub message: String,
}

#[derive(Debug)]
pub struct Greeter<T> {
    /// The inner HTTP/2.0 service
    inner: grpc::Grpc<T>,
}

use tower_grpc::client::Encodable;

impl<T> Greeter<T>
where T: tower_h2::HttpService,
{
    pub fn new(inner: T, uri: http::Uri) -> Self {
        let inner = grpc::Builder::new()
            .uri(uri)
            .build(inner);

        Greeter { inner }
    }

    pub fn poll_ready(&mut self) -> futures::Poll<(), grpc::Error<T::Error>> {
        self.inner.poll_ready()
    }

    pub fn say_hello(&mut self, request: grpc::Request<HelloRequest>)
        -> grpc::unary::ResponseFuture<HelloReply, T::Future, T::ResponseBody>
    where grpc::unary::Once<HelloRequest>: Encodable<T::RequestBody>,
    {
        let path = http::uri::PathAndQuery::from_static("/helloworld.Greeter/SayHello");
        self.inner.unary(request, path)
    }
}

pub fn main() {
    let _ = ::env_logger::init();

    let mut core = Core::new().unwrap();
    let reactor = core.handle();

    let addr = "[::1]:50051".parse().unwrap();

    let conn = Conn(addr, reactor.clone());
    let h2 = tower_h2::Client::new(conn, Default::default(), reactor);

    let done = h2.new_service()
        .map_err(|e| unimplemented!("h2 new_service error: {:?}", e))
        .and_then(move |service| {
            let uri = "http://127.0.0.1:8888/".parse().unwrap();
            let mut client = Greeter::new(service, uri);

            client.say_hello(grpc::Request::new(HelloRequest {
                name: String::from("world"),
            }))
        })
        .map(|reply| println!("Greeter.SayHello: {}", reply.get_ref().message))
        .map_err(|e| println!("error: {:?}", e));

    let _ = core.run(done);
}
