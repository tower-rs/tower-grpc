#![deny(warnings, rust_2018_idioms)]

use crate::metadata::{server, EnterReply, EnterRequest};

use futures::{future, Future, Stream};
use log::error;
use tokio::net::TcpListener;
use tower_grpc::{Request, Response};
use tower_hyper::server::{Http, Server};

pub mod metadata {
    include!(concat!(env!("OUT_DIR"), "/metadata.rs"));
}

#[derive(Clone, Debug)]
struct Door;

impl server::Doorman for Door {
    type AskToEnterFuture = future::FutureResult<Response<EnterReply>, tower_grpc::Status>;

    fn ask_to_enter(&mut self, request: Request<EnterRequest>) -> Self::AskToEnterFuture {
        println!("REQUEST = {:?}", request);

        let metadata = request
            .metadata()
            .get("metadata")
            .and_then(|header| header.to_str().ok());

        let message = match metadata {
            Some("Here is a cookie") => "Yummy! Please come in.".to_string(),
            _ => "You cannot come in!".to_string(),
        };

        let response = Response::new(EnterReply { message });

        future::ok(response)
    }
}

pub fn main() {
    let _ = ::env_logger::init();

    let new_service = server::DoormanServer::new(Door);

    let mut server = Server::new(new_service);

    let http = Http::new().http2_only(true).clone();

    let addr = "[::1]:50051".parse().unwrap();
    let bind = TcpListener::bind(&addr).expect("bind");

    let serve = bind
        .incoming()
        .for_each(move |sock| {
            if let Err(e) = sock.set_nodelay(true) {
                return Err(e);
            }

            let serve = server.serve_with(sock, http.clone());
            tokio::spawn(serve.map_err(|e| error!("h2 error: {:?}", e)));

            Ok(())
        })
        .map_err(|e| eprintln!("accept error: {}", e));

    tokio::run(serve);
}
