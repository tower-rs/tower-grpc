#![allow(unused_variables)]

extern crate env_logger;
#[macro_use]
extern crate futures;
#[macro_use]
extern crate log;
#[macro_use]
extern crate prost_derive;
extern crate tokio_core;
extern crate tower;
extern crate tower_h2;
extern crate tower_grpc;

use futures::{future, Future, Stream};
use tokio_core::net::TcpListener;
use tokio_core::reactor::Core;
use tower_grpc::{Request, Response};
use tower_h2::Server;

#[derive(Clone, Debug)]
struct Greet;

impl Greeter for Greet {
    type SayHelloFuture = future::FutureResult<Response<HelloReply>, tower_grpc::Error>;

    fn say_hello(&mut self, request: Request<HelloRequest>) -> Self::SayHelloFuture {
        let response = Response::new(HelloReply {
            message: "Zomg, it works!".to_string(),
        });

        future::ok(response)
    }
}

pub fn main() {
    let _ = ::env_logger::init();

    let mut core = Core::new().unwrap();
    let reactor = core.handle();

    let new_service = server::GreeterServer::new(Greet);

    let h2 = Server::new(new_service, Default::default(), reactor.clone());

    let addr = "[::1]:50051".parse().unwrap();
    let bind = TcpListener::bind(&addr, &reactor).expect("bind");

    let serve = bind.incoming()
        .fold((h2, reactor), |(h2, reactor), (sock, _)| {
            if let Err(e) = sock.set_nodelay(true) {
                return Err(e);
            }

            let serve = h2.serve(sock);
            reactor.spawn(serve.map_err(|e| error!("h2 error: {:?}", e)));

            Ok((h2, reactor))
        });

    core.run(serve).unwrap();
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

pub trait Greeter: Clone {
    type SayHelloFuture: Future<Item = Response<HelloReply>, Error = tower_grpc::Error>;

    fn say_hello(&mut self, request: Request<HelloRequest>) -> Self::SayHelloFuture;
}

pub mod server {
    use super::{HelloRequest, HelloReply, Greeter};
    use ::tower_grpc::codegen::server::*;

    #[derive(Debug, Clone)]
    pub struct GreeterServer<T> {
        handler: T,
    }

    impl<T> GreeterServer<T>
    where T: Greeter,
    {
        pub fn new(handler: T) -> Self {
            GreeterServer { handler }
        }
    }

    impl<T> tower::Service for GreeterServer<T>
    where T: Greeter,
    {
        type Request = http::Request<tower_h2::RecvBody>;
        type Response = http::Response<greeter::ResponseBody<T>>;
        type Error = h2::Error;
        type Future = greeter::ResponseFuture<T>;

        fn poll_ready(&mut self) -> futures::Poll<(), Self::Error> {
            Ok(().into())
        }

        fn call(&mut self, request: Self::Request) -> Self::Future {
            use self::greeter::Kind::*;

            match request.uri().path() {
                "/helloworld.Greeter/SayHello" => {
                    let service = greeter::methods::SayHello(self.handler.clone());
                    let response = grpc::Grpc::unary(service, request);
                    greeter::ResponseFuture { kind: Ok(SayHello(response)) }
                }
                _ => {
                    greeter::ResponseFuture { kind: Err(grpc::Status::UNIMPLEMENTED) }
                }
            }
        }
    }

    impl<T> tower::NewService for GreeterServer<T>
    where T: Greeter,
    {
        type Request = http::Request<::tower_h2::RecvBody>;
        type Response = http::Response<greeter::ResponseBody<T>>;
        type Error = h2::Error;
        type Service = Self;
        type InitError = h2::Error;
        type Future = futures::FutureResult<Self::Service, Self::Error>;

        fn new_service(&self) -> Self::Future {
            futures::ok(self.clone())
        }
    }

    pub mod greeter {
        use ::tower_grpc::codegen::server::*;
        use super::Greeter;

        pub struct ResponseFuture<T>
        where T: Greeter,
        {
            pub(super) kind: Result<Kind<
                grpc::unary::ResponseFuture<methods::SayHello<T>, tower_h2::RecvBody>,
            >, grpc::Status>,
        }

        pub struct ResponseBody<T>
        where T: Greeter,
        {
            kind: Result<Kind<
                grpc::Encode<grpc::unary::Once<<methods::SayHello<T> as grpc::UnaryService>::Response>>,
            >, grpc::Status>,
        }

        /// Enumeration of all the service methods
        #[derive(Debug)]
        pub(super) enum Kind<SayHello> {
            SayHello(SayHello),
        }

        impl<T> futures::Future for ResponseFuture<T>
        where T: Greeter,
        {
            type Item = http::Response<ResponseBody<T>>;
            type Error = h2::Error;

            fn poll(&mut self) -> futures::Poll<Self::Item, Self::Error> {
                use self::Kind::*;

                match self.kind {
                    Ok(SayHello(ref mut fut)) => {
                        let response = try_ready!(fut.poll());
                        let (head, body) = response.into_parts();
                        let body = ResponseBody { kind: Ok(SayHello(body)) };
                        let response = http::Response::from_parts(head, body);
                        Ok(response.into())
                    }
                    Err(ref status) => {
                        let body = ResponseBody { kind: Err(status.clone()) };
                        Ok(grpc::Response::new(body).into_http().into())
                    }
                }
            }
        }

        impl<T> ::tower_h2::Body for ResponseBody<T>
        where T: Greeter,
        {
            type Data = bytes::Bytes;

            fn is_end_stream(&self) -> bool {
                use self::Kind::*;

                match self.kind {
                    Ok(SayHello(ref v)) => v.is_end_stream(),
                    Err(_) => true,
                }
            }

            fn poll_data(&mut self) -> futures::Poll<Option<Self::Data>, h2::Error> {
                use self::Kind::*;

                match self.kind {
                    Ok(SayHello(ref mut v)) => v.poll_data(),
                    Err(_) => Ok(None.into()),
                }
            }

            fn poll_trailers(&mut self) -> futures::Poll<Option<http::HeaderMap>, h2::Error> {
                use self::Kind::*;

                match self.kind {
                    Ok(SayHello(ref mut v)) => v.poll_trailers(),
                    Err(ref status) => {
                        let mut map = http::HeaderMap::new();
                        map.insert("grpc-status", status.to_header_value());
                        Ok(Some(map).into())
                    }
                }
            }
        }

        pub mod methods {
            use super::super::{HelloRequest, HelloReply, Greeter};
            use ::tower_grpc::codegen::server::*;

            pub struct SayHello<T>(pub T);

            impl<T: Greeter> tower::ReadyService for SayHello<T> {
                type Request = grpc::Request<HelloRequest>;
                type Response = grpc::Response<HelloReply>;
                type Error = grpc::Error;
                type Future = T::SayHelloFuture;

                fn call(&mut self, request: Self::Request) -> Self::Future {
                    self.0.say_hello(request)
                }
            }
        }
    }
}
