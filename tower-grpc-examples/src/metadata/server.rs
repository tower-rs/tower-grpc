extern crate bytes;
extern crate env_logger;
extern crate futures;
#[macro_use]
extern crate log;
extern crate prost;
#[macro_use]
extern crate prost_derive;
extern crate http;
extern crate tokio_core;
extern crate tower_grpc;
extern crate tower_h2;

pub mod metadata {
    include!(concat!(env!("OUT_DIR"), "/metadata.rs"));
}

use metadata::{server, EnterReply, EnterRequest};

use futures::{future, Future, Stream};
use tokio_core::net::TcpListener;
use tokio_core::reactor::Core;
use tower_grpc::{Request, Response};
use tower_h2::Server;

#[derive(Clone, Debug)]
struct Door;

impl server::Doorman for Door {
    type AskToEnterFuture = future::FutureResult<Response<EnterReply>, tower_grpc::Error>;

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

    let mut core = Core::new().unwrap();
    let reactor = core.handle();

    let new_service = server::DoormanServer::new(Door);

    let h2 = Server::new(new_service, Default::default(), reactor.clone());

    let addr = "[::1]:50051".parse().unwrap();
    let bind = TcpListener::bind(&addr, &reactor).expect("bind");

    let serve = bind
        .incoming()
        .fold((h2, reactor), |(mut h2, reactor), (sock, _)| {
            if let Err(e) = sock.set_nodelay(true) {
                return Err(e);
            }

            let serve = h2.serve(sock);
            reactor.spawn(serve.map_err(|e| error!("h2 error: {:?}", e)));

            Ok((h2, reactor))
        });

    core.run(serve).unwrap();
}
