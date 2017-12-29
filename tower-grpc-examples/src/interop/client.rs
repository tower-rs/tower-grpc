
extern crate console;
#[macro_use]
extern crate clap;
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

use std::error::Error;
use std::fmt;
use std::net::{IpAddr, SocketAddr};

use futures::{future, Future};
use tokio_core::reactor::Core;
use tokio_core::net::TcpStream;
use tower_grpc::{Request, Response};
use tower_h2::client::Connection;

mod pb {
    #![allow(dead_code)]
    include!(concat!(env!("OUT_DIR"), "/grpc.testing.rs"));
}

mod util;

const LARGE_REQ_SIZE: usize = 271828;
const LARGE_RSP_SIZE: i32 = 314159;
arg_enum!{
    #[derive(Debug, Copy, Clone)]
    #[allow(non_camel_case_types)]
    enum Testcase {
        empty_unary,
        cacheable_unary,
        large_unary,
        client_compressed_unary,
        server_compressed_unary,
        client_streaming,
        client_compressed_streaming,
        server_streaming,
        server_compressed_streaming,
        ping_pong,
        empty_stream,
        compute_engine_creds,
        jwt_token_creds,
        oauth2_auth_token,
        per_rpc_creds,
        custom_metadata,
        status_code_and_message,
        unimplemented_method,
        unimplemented_service,
        cancel_after_begin,
        cancel_after_first_response,
        timeout_on_sleeping_server,
        concurrent_large_unary
    }
}

enum TestAssertion {
    Passed { description: &'static str },
    Failed { description: &'static str, 
             expression: &'static str, 
             why: Option<String> },
    Errored { description: &'static str, error: Box<Error> }
}

impl fmt::Display for TestAssertion {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use console::{Emoji, style};
        match *self {
            TestAssertion::Passed { ref description } =>
                write!(f, "{check} {desc}", 
                    check = style(Emoji("✔", "+")).green(),
                    desc = style(description).green(),
                ),
            TestAssertion::Failed { 
                ref description,
                ref expression,
                why: Some(ref why),
            } =>
                write!(f, "{check} {desc}\n  in `{exp}`: {why}",
                    check = style(Emoji("✖", "x")).red(),
                    desc = style(description).red(),
                    exp = expression,
                    why = why,
                ),
            TestAssertion::Failed { 
                ref description,
                ref expression,
                why: None,
            } =>
                write!(f, "{check} {desc}\n  in `{exp}`",
                    check = style(Emoji("✖", "x")).red(),
                    desc = style(description).red(),
                    exp = expression,
                ),
            _ => unimplemented!()
        }
        
    }
}

// impl TestAssertion {
//     fn from_err<E: Error>(e: E) -> Self {
//         TestAssertion::Errored
//     }
// }

macro_rules! test_assert {
    ($description:expr, $assertion:expr) => {
        if $assertion {
            TestAssertion::Passed { description: $description }
        } else {
            TestAssertion::Failed { 
                description: $description,
                expression: stringify!($assertion),
                why: None
            }
        }
    };
    ($description:expr, $assertion:expr, $why:expr) => {
        if $assertion {
            TestAssertion::Passed { description: $description }
        } else {
            TestAssertion::Failed { 
                description: $description,
                expression: stringify!($assertion),
                why: Some($why)
            }
        }
    }; 
}


// impl Testcase {
//     fn future<S>(self, client: &mut pb::client::TestService<S>) -> TestFuture
//     where 
//         S: tower_h2::HttpService,
//         S::Future: Future,
//     {

//     }
        
// }

struct ServerInfo {
    addr: SocketAddr,
    uri: http::Uri,
    hostname_override: Option<String>,
}

impl<'a> From<&'a clap::ArgMatches<'a>> for ServerInfo {
    fn from(matches: &'a clap::ArgMatches<'a>) -> Self {
        let ip = value_t!(matches, "server_host", IpAddr)
            .unwrap_or_else(|e| e.exit());
        let port = value_t!(matches, "server_port", u16)
            .unwrap_or_else(|e| e.exit());

        let addr = SocketAddr::new(ip, port);
        info!("server_address={:?};", addr);

        let ip_str = matches
            .value_of("server_host")
            .expect("server_host was None unexpected!")
            ;
        let port_str = matches
            .value_of("server_port")
            .expect("server_port was None unexpectedly!")
            ;
        let uri: http::Uri = format!("http://{}:{}", ip_str, port_str)
            .parse()
            .expect("invalid uri")
            ;

        ServerInfo {
            addr,
            uri,
            hostname_override: None, // unimplemented
        }
    }
}

fn main() {
    use clap::{Arg, App};
    let _ = ::env_logger::init();

    let matches = 
        App::new("interop-client")
            .author("Eliza Weisman <eliza@buoyant.io>")
            .arg(Arg::with_name("server_host")
                .long("server_host")
                .value_name("HOSTNAME")
                .help("The server host to connect to. For example, \"localhost\" or \"127.0.0.1\"")
                .takes_value(true)
                .default_value("127.0.0.1")
            )
            .arg(Arg::with_name("server_host_override")
                .long("server_host_override")
                .value_name("HOSTNAME")
                .help("The server host to claim to be connecting to, for use in TLS and HTTP/2 :authority header. If unspecified, the value of `--server_host` will be used")
                .takes_value(true)
            )
            .arg(Arg::with_name("server_port")
                .long("server_port")
                .value_name("PORT")
                .help("The server port to connect to. For example, \"8080\".")
                .takes_value(true)
                .default_value("10000")
            )
            .arg(Arg::with_name("test_case")
                .long("test_case")
                .value_name("TESTCASE")
                .help("The name of the test case to execute. For example, 
                \"empty_unary\".")
                .possible_values(&Testcase::variants())
                .takes_value(true)
            )
            .get_matches();

    let ServerInfo { addr, uri, .. } = ServerInfo::from(&matches);

    let test_case = value_t!(matches, "test_case", Testcase)
        .unwrap_or_else(|e| e.exit());

    let mut core = Core::new().expect("could not create reactor core!");
    let reactor = core.handle();

    let mut client = core.run(
            TcpStream::connect(&addr, &reactor)
                .and_then(move |socket| {
                    // Bind the HTTP/2.0 connection
                    Connection::handshake(socket, reactor)
                        .map_err(|_| panic!("failed HTTP/2.0 handshake"))
                })
                .and_then(move |conn| {
                    use pb::client::TestService;
                    let client = TestService::new(conn, uri)
                        .expect("TestService::new");
                    Ok(client)
                })
        ).expect("client");
    
    let test = match test_case {
        Testcase::empty_unary => {
            use pb::Empty;
            core.run(client.empty_call(Request::new(Empty {}))
                .then(|result| {
                    let mut assertions = vec![
                        test_assert!(
                            "call must be successful",
                            result.is_ok(),
                            format!("result={:?}", result)
                        )
                    ];
                    if let Ok(body) = result.map(|r| r.into_inner()) {
                        assertions.push(test_assert!(
                            "body must not be null",
                            body == Empty{},
                            format!("body={:?}", body)
                        ))
                    }
                    future::ok::<Vec<TestAssertion>, Box<Error>>(assertions)
                }))
        },
        Testcase::large_unary => {
            use pb::SimpleRequest;
            use std::mem;
            let payload = util::client_payload(
                pb::PayloadType::Compressable,
                LARGE_REQ_SIZE,
            );
            let req = SimpleRequest {
                response_type: pb::PayloadType::Compressable as i32,
                response_size: LARGE_RSP_SIZE,
                payload: Some(payload),
                ..Default::default()
            };
            core.run(client.unary_call(Request::new(req))
                .then(|result| {
                   let mut assertions = vec![
                        test_assert!(
                            "call must be successful",
                            result.is_ok(),
                            format!("result={:?}", result)
                        )
                   ];
                    if let Ok(body) = result.map(|r| r.into_inner()) {
                        assertions.push(test_assert!(
                        "body must be 314159 bytes",
                        mem::size_of_val(&body) == LARGE_RSP_SIZE as usize,
                        format!("mem::size_of_val(&body)={:?}", 
                            mem::size_of_val(&body))
                        ));
                    }
                    future::ok::<Vec<TestAssertion>, Box<Error>>(assertions)
                }))
        },
        Testcase::cacheable_unary => {
            let payload = pb::Payload {
                type_: pb::PayloadType::Compressable as i32,
                body: format!("{:?}", std::time::Instant::now()).into_bytes(),
            };
            // let req = SimpleRequest {
            //     response_type: pb
            // };
            unimplemented!()
        },
        _ => unimplemented!()
    };
    
    let test_results = test.expect("error running test!");

    println!("{:?}:", test_case);
    for result in test_results {
        println!("  {}", result);
    }
    

    // match test_case {
    //     Testcase::empty_unary => {
    //         let test = 
    //         core.run(test).expect("run test");
    //     },
    //     // cacheable_unary => {
    //     //     let test = 
    //     //         TcpStream::connect(&addr, &reactor)
    //     //             .and_then(move |socket| {
    //     //                 // Bind the HTTP/2.0 connection
    //     //                 Connection::handshake(socket, reactor)
    //     //                     .map_err(|_| panic!("failed HTTP/2.0 handshake"))
    //     //             })
    //     //             .and_then(move |conn| {
    //     //                 use testing::client::TestService;
    //     //                 let client = TestService::new(conn, uri)
    //     //                     .expect("TestService::new");
    //     //                 Ok(client)
    //     //             })
    //     //             .and_then(|mut client| {
    //     //                 use testing::SimpleRequest;

    //     //                 client.cacheable_unary(Request::new(SimpleRequest {
                            
    //     //                 }))
    //     //                     .map_err(|e| panic!("gRPC request failed; err={:?}", e))
    //     //             })
    //     //             .and_then(|response| {
    //     //                 println!("RESPONSE = {:?}", response);
    //     //                 Ok(())
    //     //             })
    //     //             .map_err(|e| {
    //     //                 println!("ERR = {:?}", e);
    //     //             });
    //     //     core.run(test).expect("run test");

    //     // },
    //     Testcase::large_unary => {
    //         let test = 
    //             TcpStream::connect(&addr, &reactor)
    //                 .and_then(move |socket| {
    //                     // Bind the HTTP/2.0 connection
    //                     Connection::handshake(socket, reactor)
    //                         .map_err(|_| panic!("failed HTTP/2.0 handshake"))
    //                 })
    //                 .and_then(move |conn| {
    //                     use pb::client::TestService;
    //                     let client = TestService::new(conn, uri)
    //                         .expect("TestService::new");
    //                     Ok(client)
    //                 })
    //                 .and_then(|mut client| {
    //                     use pb::SimpleRequest;
    //                     let payload = util::client_payload(
    //                         pb::PayloadType::Compressable,
    //                         LARGE_REQ_SIZE,
    //                     );
    //                     let req = SimpleRequest {
    //                         response_type: pb::PayloadType::Compressable as i32,
    //                         response_size: LARGE_RSP_SIZE,
    //                         payload: Some(payload),
    //                         ..Default::default()
    //                     };
    //                     client.unary_call(Request::new(req))
    //                         .map_err(|e| panic!("gRPC request failed; err={:?}", e))
    //                 })
    //                 .and_then(|response| {
    //                     println!("RESPONSE = {:?}", response);
    //                     Ok(())
    //                 })
    //                 .map_err(|e| {
    //                     println!("ERR = {:?}", e);
    //                 });
    //         core.run(test).expect("run test");
    //     },
    //     t => unimplemented!("test case {:?} is not yet implemented.", t),
    // };


}