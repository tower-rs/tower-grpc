#[macro_use]
extern crate clap;
extern crate futures;
#[macro_use]
extern crate log;
extern crate pretty_env_logger;
extern crate prost;
extern crate tokio;
extern crate tower_h2;
extern crate tower_grpc;

use futures::{future, stream, Future, Stream};
use tokio::executor::DefaultExecutor;
use tokio::net::TcpListener;
use tower_h2::Server;
use tower_grpc::{Code, Request, Response, Status};

mod pb {
    #![allow(dead_code)]
    #![allow(unused_imports)]
    include!(concat!(env!("OUT_DIR"), "/grpc.testing.rs"));
}

type GrpcFut<T> = Box<dyn Future<Item = tower_grpc::Response<T>, Error = tower_grpc::Status> + Send>;
type GrpcStream<T> = Box<dyn Stream<Item = T, Error = tower_grpc::Status> + Send>;

macro_rules! todo {
    ($($piece:tt)+) => ({
        let msg = format!(
            "server test case is not supported yet: {}",
            format_args!($($piece)+),
        );
        eprintln!("TODO! {}", msg);
        return Box::new(future::err(tower_grpc::Status::new(
            tower_grpc::Code::Unknown,
            msg,
        )));
    })
}

#[derive(Clone)]
struct Test;

impl pb::server::TestService for Test {
    type EmptyCallFuture = GrpcFut<pb::Empty>;
    type UnaryCallFuture = GrpcFut<pb::SimpleResponse>;
    type CacheableUnaryCallFuture = GrpcFut<pb::SimpleResponse>;
    type StreamingOutputCallStream = GrpcStream<pb::StreamingOutputCallResponse>;
    type StreamingOutputCallFuture = GrpcFut<Self::StreamingOutputCallStream>;
    type StreamingInputCallFuture = GrpcFut<pb::StreamingInputCallResponse>;
    type FullDuplexCallStream = GrpcStream<pb::StreamingOutputCallResponse>;
    type FullDuplexCallFuture = GrpcFut<Self::FullDuplexCallStream>;
    type HalfDuplexCallStream = GrpcStream<pb::StreamingOutputCallResponse>;
    type HalfDuplexCallFuture = GrpcFut<Self::HalfDuplexCallStream>;
    type UnimplementedCallFuture = GrpcFut<pb::Empty>;

    /// One empty request followed by one empty response.
    fn empty_call(&mut self, _request: Request<pb::Empty>) -> Self::EmptyCallFuture {
        eprintln!("empty_call");
        Box::new(future::ok(Response::new(pb::Empty::default())))
    }

    /// One request followed by one response.
    fn unary_call(&mut self, request: Request<pb::SimpleRequest>) -> Self::UnaryCallFuture {
        eprintln!("unary_call");
        let req = request.into_inner();

        // EchoStatus
        if let Some(echo_status) = req.response_status {
            let status = Status::new(
                Code::from_i32(echo_status.code),
                echo_status.message,
            );
            return Box::new(future::err(status));
        }

        let res_size = if req.response_size >= 0 {
            req.response_size as usize
        } else {
            let status = Status::new(
                Code::InvalidArgument,
                "response_size cannot be negative",
            );
            return Box::new(future::err(status));
        };

        let res = pb::SimpleResponse {
            payload: Some(pb::Payload {
                body: vec![0; res_size],
                ..Default::default()
            }),
            ..Default::default()
        };
        Box::new(future::ok(Response::new(res)))
    }

    /// One request followed by one response. Response has cache control
    /// headers set such that a caching HTTP proxy (such as GFE) can
    /// satisfy subsequent requests.
    fn cacheable_unary_call(&mut self, _request: Request<pb::SimpleRequest>) -> Self::CacheableUnaryCallFuture {
        todo!("cacheable_unary_call");
    }

    /// One request followed by a sequence of responses (streamed download).
    /// The server returns the payload with client desired type and sizes.
    fn streaming_output_call(&mut self, _request: Request<pb::StreamingOutputCallRequest>) -> Self::StreamingOutputCallFuture {
        todo!("streaming_output_call");
    }

    /// A sequence of requests followed by one response (streamed upload).
    /// The server returns the aggregated size of client payload as the result.
    fn streaming_input_call(&mut self, _request: Request<tower_grpc::Streaming<pb::StreamingInputCallRequest>>) -> Self::StreamingInputCallFuture {
        todo!("streaming_input_call");
    }

    /// A sequence of requests with each request served by the server immediately.
    /// As one request could lead to multiple responses, this interface
    /// demonstrates the idea of full duplexing.
    fn full_duplex_call(&mut self, request: Request<tower_grpc::Streaming<pb::StreamingOutputCallRequest>>) -> Self::FullDuplexCallFuture {
        eprintln!("full_duplex_call");
        let rx = request
            .into_inner()
            .and_then(|req| {
                // EchoStatus
                if let Some(echo_status) = req.response_status {
                    let status = Status::new(
                        Code::from_i32(echo_status.code),
                        echo_status.message,
                    );
                    return Err(status);
                }

                let mut resps = Vec::new();

                for params in req.response_parameters {
                    let res_size = if params.size >= 0 {
                        params.size as usize
                    } else {
                        let status = tower_grpc::Status::new(
                            tower_grpc::Code::InvalidArgument,
                            "response_size cannot be negative",
                        );
                        return Err(status);
                    };

                    resps.push(pb::StreamingOutputCallResponse {
                        payload: Some(pb::Payload {
                            body: vec![0; res_size],
                            ..Default::default()
                        }),
                        ..Default::default()
                    });
                }

                Ok(stream::iter_ok(resps))
            })
            .flatten();

        let res = Response::new(Box::new(rx) as Self::FullDuplexCallStream);
        Box::new(future::ok(res))
    }

    /// A sequence of requests followed by a sequence of responses.
    /// The server buffers all the client requests and then serves them in order. A
    /// stream of responses are returned to the client when the server starts with
    /// first request.
    fn half_duplex_call(&mut self, _request: Request<tower_grpc::Streaming<pb::StreamingOutputCallRequest>>) -> Self::HalfDuplexCallFuture {
        todo!("half_duplex_call");
    }

    /// The test server will not implement this method. It will be used
    /// to test the behavior when clients call unimplemented methods.
    fn unimplemented_call(&mut self, _request: Request<pb::Empty>) -> Self::UnimplementedCallFuture {
        eprintln!("unimplemented_call");
        Box::new(future::err(tower_grpc::Status::new(
            tower_grpc::Code::Unimplemented,
            "explicitly unimplemented_call",
        )))
    }
}

fn main() {
    use clap::{Arg, App};
    let _ = ::pretty_env_logger::init();

    let matches = App::new("interop-server")
        .arg(Arg::with_name("port")
            .long("port")
            .value_name("PORT")
            .help("The server port to listen on. For example, \"8080\".")
            .takes_value(true)
            .default_value("10000")
        )
        .get_matches();

    let port = value_t!(matches, "port", u16).expect("port argument");

    let new_service = pb::server::TestServiceServer::new(Test);

    let h2_settings = Default::default();
    let mut h2 = Server::new(new_service, h2_settings, DefaultExecutor::current());

    let addr = format!("0.0.0.0:{}", port).parse().unwrap();
    let bind = TcpListener::bind(&addr).expect("bind");

    let serve = bind.incoming()
        .for_each(move |sock| {
            if let Err(e) = sock.set_nodelay(true) {
                return Err(e);
            }

            let serve = h2.serve(sock);
            tokio::spawn(serve.map_err(|e| error!("h2 error: {:?}", e)));

            Ok(())
        })
        .map_err(|e| eprintln!("accept error: {}", e));

    eprintln!("grpc interop server listening on {}", addr);
    tokio::run(serve)
}
