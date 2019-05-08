extern crate bytes;
extern crate env_logger;
extern crate futures;
extern crate http;
extern crate hyper;
extern crate log;
extern crate prost;
extern crate tokio;
extern crate tower_grpc;
extern crate tower_hyper;
extern crate tower_request_modifier;
extern crate tower_service;
extern crate tower_util;

use futures::Future;
use hyper::client::connect::{Destination, HttpConnector};
use tower_grpc::Request;
use tower_hyper::{client, util};
use tower_util::MakeService;

pub mod metadata {
    include!(concat!(env!("OUT_DIR"), "/metadata.rs"));
}

pub fn main() {
    let _ = ::env_logger::init();

    let uri: http::Uri = format!("http://[::1]:50051").parse().unwrap();

    let dst = Destination::try_from_uri(uri.clone()).unwrap();
    let connector = util::Connector::new(HttpConnector::new(4));
    let settings = client::Builder::new().http2_only(true).clone();
    let mut make_client = client::Connect::new(connector, settings);

    let doorman = make_client
        .make_service(dst)
        .map_err(|e| panic!("connect error: {:?}", e))
        .and_then(move |conn| {
            use metadata::client::Doorman;

            let conn = tower_request_modifier::Builder::new()
                .set_origin(uri)
                .build(conn)
                .unwrap();

            // Wait until the client is ready...
            Doorman::new(conn).ready()
        })
        .and_then(|mut client| {
            use metadata::EnterRequest;

            let mut request = Request::new(EnterRequest {
                message: "Hello! Can I come in?".to_string(),
            });

            request
                .metadata_mut()
                .insert("metadata", "Here is a cookie".parse().unwrap());

            client.ask_to_enter(request)
        })
        .map(|response| {
            println!("RESPONSE = {:?}", response);
        })
        .map_err(|e| {
            println!("ERR = {:?}", e);
        });

    tokio::run(doorman);
}
