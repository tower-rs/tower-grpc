extern crate env_logger;
extern crate futures;
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
    inner: grpc::Grpc<T>,
}

use tower_grpc::client::{Once, IntoBody};

impl<T> Greeter<T>
where T: grpc::HttpService,
{
    pub fn new(inner: T) -> Self {
        let inner = grpc::Grpc::new(inner);
        Greeter { inner }
    }

    pub fn poll_ready(&mut self) -> futures::Poll<(), grpc::Error<T::Error>> {
        self.inner.poll_ready()
    }

    // TODO: This should take grpc::Request<HelloRequest>
    pub fn say_hello(&mut self, request: HelloRequest)
        -> grpc::unary::ResponseFuture<HelloReply, T::Future, T::ResponseBody>
    where Once<HelloRequest>: IntoBody<T::RequestBody>,
    {
        let request = grpc::Request::new("/helloworld.Greeter/SayHello", request);
        self.inner.unary(request)
    }
}

pub fn main() {
    let _ = ::env_logger::init();

    let mut core = Core::new().unwrap();
    let reactor = core.handle();

    let addr = "[::1]:8888".parse().unwrap();

    let conn = Conn(addr, reactor.clone());
    let h2 = tower_h2::Client::new(conn, Default::default(), reactor);

    let done = h2.new_service()
        .map_err(|e| unimplemented!("h2 new_service error: {:?}", e))
        .and_then(move |service| {
            let mut client = Greeter::new(service);
            client.say_hello(HelloRequest {
                name: String::from("world"),
            })
            /*
            let service = AddOrigin(service);
            let grpc = tower_grpc::client::Client::new(StupidCodec, service);
            let say_hello = SayHello::new(grpc);
            let mut greeter = Greeter::new(say_hello);
            greeter.say_hello(HelloRequest {
                name: String::from("world"),
            })
            */
        })
        .map(|reply| println!("Greeter.SayHello: {}", reply.get_ref().message))
        .map_err(|e| println!("error: {:?}", e));

    let _ = core.run(done);
}
