
extern crate console;
#[macro_use]
extern crate clap;
extern crate domain;
extern crate env_logger;
extern crate http;
extern crate futures;
#[macro_use]
extern crate log;
extern crate prost;
#[macro_use]
extern crate prost_derive;
extern crate tokio_core;
extern crate rustls;
extern crate tower_http;
extern crate tower_h2;
extern crate tower_grpc;

use std::error::Error;
use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use http::header::HeaderValue;
use http::uri::{self, Uri};
use futures::{future, Future, stream};
use tokio_core::reactor;
use tokio_core::net::TcpStream;
use tower_grpc::Request;
use tower_h2::client::Connection;

use pb::SimpleRequest;
use pb::StreamingInputCallRequest;
use pb::client::TestService;


mod pb {
    #![allow(dead_code)]
    #![allow(unused_imports)]
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

#[derive(Debug)]
enum ClientError {
    InvalidArgument(clap::Error),
    InvalidUri(uri::InvalidUri),
    Dns(DnsError),
}

#[derive(Debug)]
enum DnsError {
    ResolveError(domain::resolv::error::Error),
    NoHosts,
}

impl ClientError {
    fn exit(&self) -> ! {
        match *self {
            ClientError::InvalidArgument(ref clap_error) => clap_error.exit(),
            _ => unimplemented!()
        }
    }
}

impl From<clap::Error> for ClientError {
    fn from(clap_error: clap::Error) -> Self {
        ClientError::InvalidArgument(clap_error)
    }
}

impl From<uri::InvalidUri> for ClientError {
    fn from(invalid_uri: uri::InvalidUri) -> Self {
        ClientError::InvalidUri(invalid_uri)
    }
}

impl<T> From<T> for ClientError where DnsError: From<T> {
    fn from(t: T) -> Self {
        ClientError::Dns(DnsError::from(t))
    }
}

impl From<domain::resolv::error::Error> for DnsError {
    fn from(dns: domain::resolv::error::Error) -> Self {
        DnsError::ResolveError(dns)
    }
}
// pub struct TestResults {
//     name: String,
//     assertions: Vec<TestAssertion>,
// }

// impl TestResults {
//     pub fn passed(&self) -> bool {
//         self.assertions.iter().all(TestAssertion::passed)
//     }
// }

// impl fmt::Display for TestResults {
//     fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
//         use console::{Emoji, style};
//         let passed = self.is_passed();
//         write!(f, "{check} {name}\n",
//             check = if passed {
//                 style(Emoji("✔", "+")).green()
//             } else {
//                 style(Emoji("✖", "x")).red()
//             },
//             name = if passed {
//                 style(self.name).green()
//             } else {
//                 style(self.name).red()
//             },
//         )?;
//         for result in self.assertions {
//             write!(f, "  {}\n", result)?;
//         }
//     }
// }

impl Testcase {
    fn run(&self, server: &ServerInfo, core: &mut tokio_core::reactor::Core)
           -> Result<Vec<TestAssertion>, Box<Error>> {

        let reactor = core.handle();
        let mut client = core.run(
            TcpStream::connect(&server.addr, &reactor)
                .and_then(move |socket| {
                    // Bind the HTTP/2.0 connection
                    Connection::handshake(socket, reactor)
                        .map_err(|_| panic!("failed HTTP/2.0 handshake"))
                })
                .map(move |conn| {
                    use tower_http::add_origin;

                    let conn = add_origin::Builder::new()
                        .uri(server.uri.clone())
                        .build(conn)
                        .unwrap();

                    TestService::new(conn)
                })
        ).expect("client");

        match *self {
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
                use std::mem;
                let payload = util::client_payload(LARGE_REQ_SIZE);
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
                            let payload_len = body.payload.as_ref()
                                .map(|p| p.body.len())
                                .unwrap_or(0);

                            assertions.push(test_assert!(
                            "body must be 314159 bytes",
                            payload_len == LARGE_RSP_SIZE as usize,
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
                let req = SimpleRequest {
                    response_type: pb::PayloadType::Compressable as i32,
                    payload: Some(payload),
                    ..Default::default()
                };
                let mut req = Request::new(req);
                req.headers_mut()
                    .insert(" x-user-ip", HeaderValue::from_static("1.2.3.4"));
                // core.run(client.unary_call(req)
                //     .then(|result| {
                //         unimplemented!()
                //     })
                // )
                unimplemented!()
            },
            Testcase::client_streaming => {
                let requests = vec![27182, 8, 1828, 45904]
                    .into_iter()
                    .map(|len| StreamingInputCallRequest {
                        payload: Some(util::client_payload(len as usize)),
                        ..Default::default()
                    });
                let stream = stream::iter_ok(requests);
                core.run(
                    client.streaming_input_call(Request::new(stream))
                        .then(|result| {
                            let mut assertions = vec![
                                    test_assert!(
                                        "call must be successful",
                                        result.is_ok(),
                                        format!("result={:?}", result)
                                    )
                            ];
                            if let Ok(response) = result.map(|r| r.into_inner()) {
                                assertions.push(test_assert!(
                                "aggregated payload size must be 74922 bytes",
                                response.aggregated_payload_size == 74922,
                                format!("aggregated_payload_size={:?}",
                                    response.aggregated_payload_size
                                )));
                            }
                            future::ok::<Vec<TestAssertion>, Box<Error>>(assertions)
                        })
                )
            },
            Testcase::compute_engine_creds
            | Testcase::jwt_token_creds
            | Testcase::oauth2_auth_token
            | Testcase::per_rpc_creds =>
                unimplemented!(
                    "test case unimplemented: tower-grpc does not \
                     currently support gRPC authorization."
                ),
            Testcase::client_compressed_unary
            | Testcase::server_compressed_unary
            | Testcase::client_compressed_streaming
            | Testcase::server_compressed_streaming =>
                unimplemented!(
                    "test case unimplemented: tower-grpc does not \
                     currently support gRPC compression."
                ),

            _ => unimplemented!()
        }
    }
}
enum TestAssertion {
    Passed { description: &'static str },
    Failed { description: &'static str,
             expression: &'static str,
             why: Option<String> },
    Errored { description: &'static str, error: Box<Error> }
}

impl TestAssertion {
    fn passed(&self) -> bool {
        if let TestAssertion::Passed { .. } = *self {
            true
        } else {
            false
        }
    }
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
                    exp = style(expression).red(),
                    why = style(why).red(),
                ),
            TestAssertion::Failed {
                ref description,
                ref expression,
                why: None,
            } =>
                write!(f, "{check} {desc}\n  in `{exp}`",
                    check = style(Emoji("✖", "x")).red(),
                    desc = style(description).red(),
                    exp = style(expression).red(),
                ),
            _ => unimplemented!()
        }

    }
}

struct ServerInfo {
    addr: SocketAddr,
    uri: Uri,
    hostname_override: Option<String>,
}

impl ServerInfo {
    fn from_args<'a>(matches: &clap::ArgMatches<'a>,
                     core: &mut reactor::Core,)
                    -> Result<Self, ClientError>
    {
        use domain::bits::DNameBuf;
        use domain::resolv::{Resolver, lookup};

        let handle = core.handle();
        // XXX this could probably look neater if only the DNS query was run in
        //     a future...
        let ip_future = future::result(value_t!(matches, "server_host", IpAddr))
            .from_err::<ClientError>()
            .or_else(|_| {
                future::result(value_t!(
                    matches,
                    "server_host",
                    DNameBuf
                ))
                .from_err::<ClientError>()
                .and_then(move |name| {
                    let resolver = Resolver::new(&handle);
                    lookup::lookup_host(resolver, name)
                        .from_err::<ClientError>()
                        .and_then(|response| {
                            response.iter()
                                .next()
                                .ok_or(ClientError::from(DnsError::NoHosts))
                        })
                })
                .from_err::<ClientError>()
            })
            .from_err::<ClientError>();
        let info = ip_future.and_then(move |ip| {
            future::result(value_t!(matches, "server_port", u16))
                .from_err::<ClientError>()
                .and_then(move |port| {
                    let addr = SocketAddr::new(ip, port);
                    info!("server_address={:?};", addr);
                    let host = matches.value_of("server_host")
                        .expect("`server_host` argument was not present, clap \
                                 should have already validated it was present.")
                        ;
                    future::result(
                        Uri::from_str(&format!("http://{}:{}", host, port))
                    )
                        .from_err::<ClientError>()
                        .map(move |uri| ServerInfo {
                            addr,
                            uri,
                            hostname_override: None, // unimplemented
                        })
                })
                .from_err::<ClientError>()
        });
        core.run(info)
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
                .default_value("large_unary")
                .takes_value(true)
                .min_values(1)
                .use_delimiter(true)
            )
            .arg(Arg::with_name("use_tls")
                .long("use_tls")
                .help("Whether to use a plaintext or encrypted connection.")
                .takes_value(true)
                .value_name("BOOLEAN")
                .possible_values(&["true", "false"])
                .default_value("false")
                .validator(|s|
                    // use a Clap validator for unimplemented flags so we get a
                    // nicer error message than the panic from
                    // `unimplemented!()`.
                    if s == "true" {
                        // unsupported, always error for now.
                        Err(String::from(
                            "tower-grpc does not currently support TLS."
                        ))
                    } else {
                        Ok(())
                    }

                )
            )
            .arg(Arg::with_name("use_test_ca")
                .long("use_test_ca")
                .help("Whether to replace platform root CAs with ca.pem as the CA root.")
            )
            .arg(Arg::with_name("ca_file")
                .long("ca_file")
                .value_name("FILE")
                .help("The file containing the CA root cert file")
                .takes_value(true)
                .default_value("ca.pem")
            )
            .arg(Arg::with_name("oauth_scope")
                .long("oauth_scope")
                .value_name("SCOPE")
                .help("The scope for OAuth2 tokens. For example, \"https://www.googleapis.com/auth/xapi.zoo\".")
                .takes_value(true)
                .validator(|_|
                    // unsupported, always error for now.
                    Err(String::from(
                        "tower-grpc does not currently support GCE auth."
                    ))
                )
            )
            .arg(Arg::with_name("default_service_account")
                .long("default_service_account")
                .value_name("ACCOUNT_EMAIL")
                .help("Email of the GCE default service account.")
                .takes_value(true)
                .validator(|_|
                    // unsupported, always error for now.
                    Err(String::from(
                        "tower-grpc does not currently support GCE auth."
                    ))
                )
            )
            .arg(Arg::with_name("service_account_key_file")
                .long("service_account_key_file")
                .value_name("PATH")
                .help("The path to the service account JSON key file generated from GCE developer console.")
                .takes_value(true)
                .validator(|_|
                    // unsupported, always error for now.
                    Err(String::from(
                        "tower-grpc does not currently support GCE auth."
                    ))
                )
            )
            .get_matches();

    if matches.is_present("oauth_scope") ||
       matches.is_present("default_service_account") ||
       matches.is_present("service_account_key_file") {
        unimplemented!("tower-grpc does not currently support GCE auth.");
    }

    let mut core = reactor::Core::new()
        .expect("could not create reactor core!");

    let server = ServerInfo::from_args(&matches, &mut core)
        .unwrap_or_else(|e| e.exit())
    ;

    let test_cases = values_t!(matches, "test_case", Testcase)
        .unwrap_or_else(|e| e.exit());

    for test in test_cases {
        println!("{:?}:", test);
        let test_results = test
            .run(&server, &mut core)
            .expect("error running test!");
        for result in test_results {
            println!("  {}", result);
        }
    }
}
