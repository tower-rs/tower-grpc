extern crate console;
#[macro_use]
extern crate clap;
extern crate domain;
extern crate futures;
extern crate http;
extern crate pretty_env_logger;
#[macro_use]
extern crate log;
extern crate prost;
extern crate rustls;
extern crate tokio_core;
extern crate tower_add_origin;
extern crate tower_grpc;
extern crate tower_h2;

use std::error::Error;
use std::fmt;
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;

use futures::{future, stream, Future, Stream};
use http::uri::{self, Uri};
use tokio_core::net::TcpStream;
use tokio_core::reactor;
use tower_grpc::metadata::MetadataValue;
use tower_grpc::Request;
use tower_h2::client::Connection;

use pb::client::TestService;
use pb::client::UnimplementedService;
use pb::SimpleRequest;
use pb::StreamingInputCallRequest;

mod pb {
    #![allow(dead_code)]
    #![allow(unused_imports)]
    include!(concat!(env!("OUT_DIR"), "/grpc.testing.rs"));
}

impl pb::ResponseParameters {
    fn with_size(size: i32) -> Self {
        pb::ResponseParameters {
            size,
            ..Default::default()
        }
    }
}
mod util;

const LARGE_REQ_SIZE: usize = 271828;
const LARGE_RSP_SIZE: i32 = 314159;
const REQUEST_LENGTHS: &'static [i32] = &[27182, 8, 1828, 45904];
const RESPONSE_LENGTHS: &'static [i32] = &[31415, 9, 2653, 58979];
const TEST_STATUS_MESSAGE: &'static str = "test status message";
const SPECIAL_TEST_STATUS_MESSAGE: &'static str =
    "\t\ntest with whitespace\r\nand Unicode BMP ☺ and non-BMP 😈\t\n";

arg_enum! {
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
        special_status_message,
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
            TestAssertion::Passed {
                description: $description,
            }
        } else {
            TestAssertion::Failed {
                description: $description,
                expression: stringify!($assertion),
                why: None,
            }
        }
    };
    ($description:expr, $assertion:expr, $why:expr) => {
        if $assertion {
            TestAssertion::Passed {
                description: $description,
            }
        } else {
            TestAssertion::Failed {
                description: $description,
                expression: stringify!($assertion),
                why: Some($why),
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
            _ => unimplemented!(),
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

impl<T> From<T> for ClientError
where
    DnsError: From<T>,
{
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

fn response_length(response: &pb::StreamingOutputCallResponse) -> i32 {
    match &response.payload {
        Some(ref payload) => payload.body.len() as i32,
        None => 0,
    }
}

fn response_lengths(responses: &Vec<pb::StreamingOutputCallResponse>) -> Vec<i32> {
    responses.iter().map(&response_length).collect()
}

/// Helper function that can be used with .then to assert that the RPC performed
/// in a test was successful.
fn assert_success(
    result: Result<Vec<TestAssertion>, Box<Error>>,
) -> future::FutureResult<Vec<TestAssertion>, Box<Error>> {
    let assertion = test_assert!(
        "call must be successful",
        result.is_ok(),
        format!("result={:?}", result)
    );
    future::ok(if let Ok(mut assertions) = result {
        assertions.push(assertion);
        assertions
    } else {
        vec![assertion]
    })
}

struct TestClients {
    test_client: TestService<
        tower_add_origin::AddOrigin<
            tower_h2::client::Connection<
                tokio_core::net::TcpStream,
                tokio_core::reactor::Handle,
                tower_grpc::BoxBody,
            >,
        >,
    >,

    unimplemented_client: UnimplementedService<
        tower_add_origin::AddOrigin<
            tower_h2::client::Connection<
                tokio_core::net::TcpStream,
                tokio_core::reactor::Handle,
                tower_grpc::BoxBody,
            >,
        >,
    >,
}

fn make_ping_pong_request(idx: usize) -> pb::StreamingOutputCallRequest {
    let req_len = REQUEST_LENGTHS[idx];
    let resp_len = RESPONSE_LENGTHS[idx];
    pb::StreamingOutputCallRequest {
        response_parameters: vec![pb::ResponseParameters::with_size(resp_len)],
        payload: Some(util::client_payload(req_len as usize)),
        ..Default::default()
    }
}

/// Contains the state that is threaded through the recursive "loop"
/// of futures that performs the actual ping-pong with the server.
struct PingPongState {
    sender: futures::sync::mpsc::UnboundedSender<pb::StreamingOutputCallRequest>,
    stream: Box<Stream<Item = pb::StreamingOutputCallResponse, Error = tower_grpc::Status>>,
    responses: Vec<pb::StreamingOutputCallResponse>,
    assertions: Vec<TestAssertion>,
}

type PingPongResponsesFuture = Box<
    Future<Item = (Vec<pb::StreamingOutputCallResponse>, Vec<TestAssertion>), Error = Box<Error>>,
>;

impl PingPongState {
    /// Recursive function that takes the current PingPongState of the ping-pong
    /// operation and performs the rest of it. It returns a future of
    /// responses from the server and assertions that were performed in
    /// the process.
    fn perform_ping_pong(self) -> PingPongResponsesFuture {
        let PingPongState {
            sender,
            stream,
            mut responses,
            mut assertions,
        } = self;
        Box::new(
            stream
                .into_future() // Take one element from the stream.
                .map_err(|(err, _stream)| -> Box<Error> { Box::new(err) })
                .and_then(|(resp, stream)| -> PingPongResponsesFuture {
                    if let Some(resp) = resp {
                        responses.push(resp);
                        if responses.len() == REQUEST_LENGTHS.len() {
                            // Close the request stream. This tells the server that there
                            // won't be any more requests.
                            drop(sender);
                            Box::new(
                                stream
                                    .into_future()
                                    .map_err(|err| -> Box<Error> { Box::new(err.0) })
                                    .map(|(resp, _stream)| {
                                        assertions.push(test_assert!(
                                            "server should close stream after client closes it.",
                                            resp.is_none(),
                                            format!("resp={:?}", resp)
                                        ));
                                        (responses, assertions)
                                    }),
                            )
                        } else {
                            sender
                                .unbounded_send(make_ping_pong_request(responses.len()))
                                .unwrap();
                            PingPongState {
                                sender,
                                stream,
                                responses,
                                assertions,
                            }
                            .perform_ping_pong()
                        }
                    } else {
                        assertions.push(TestAssertion::Failed {
                            description:
                                "server should keep the stream open until the client closes it",
                            expression: "Stream terminated unexpectedly early",
                            why: None,
                        });
                        Box::new(future::ok((responses, assertions)))
                    }
                }),
        )
    }
}

impl TestClients {
    fn empty_unary_test(&mut self) -> impl Future<Item = Vec<TestAssertion>, Error = Box<Error>> {
        use pb::Empty;
        self.test_client
            .empty_call(Request::new(Empty {}))
            .then(|result| {
                let mut assertions = vec![test_assert!(
                    "call must be successful",
                    result.is_ok(),
                    format!("result={:?}", result)
                )];
                if let Ok(body) = result.map(|r| r.into_inner()) {
                    assertions.push(test_assert!(
                        "body must not be null",
                        body == Empty {},
                        format!("body={:?}", body)
                    ))
                }
                future::ok::<Vec<TestAssertion>, Box<Error>>(assertions)
            })
    }

    fn large_unary_test(&mut self) -> impl Future<Item = Vec<TestAssertion>, Error = Box<Error>> {
        use std::mem;
        let payload = util::client_payload(LARGE_REQ_SIZE);
        let req = SimpleRequest {
            response_type: pb::PayloadType::Compressable as i32,
            response_size: LARGE_RSP_SIZE,
            payload: Some(payload),
            ..Default::default()
        };
        self.test_client
            .unary_call(Request::new(req))
            .then(|result| {
                let mut assertions = vec![test_assert!(
                    "call must be successful",
                    result.is_ok(),
                    format!("result={:?}", result)
                )];
                if let Ok(body) = result.map(|r| r.into_inner()) {
                    let payload_len = body.payload.as_ref().map(|p| p.body.len()).unwrap_or(0);

                    assertions.push(test_assert!(
                        "body must be 314159 bytes",
                        payload_len == LARGE_RSP_SIZE as usize,
                        format!("mem::size_of_val(&body)={:?}", mem::size_of_val(&body))
                    ));
                }
                future::ok::<Vec<TestAssertion>, Box<Error>>(assertions)
            })
    }

    fn cacheable_unary_test(
        &mut self,
    ) -> impl Future<Item = Vec<TestAssertion>, Error = Box<Error>> {
        let payload = pb::Payload {
            r#type: pb::PayloadType::Compressable as i32,
            body: format!("{:?}", std::time::Instant::now()).into_bytes(),
        };
        let req = SimpleRequest {
            response_type: pb::PayloadType::Compressable as i32,
            payload: Some(payload),
            ..Default::default()
        };
        let mut req = Request::new(req);
        req.metadata_mut()
            .insert(" x-user-ip", MetadataValue::from_static("1.2.3.4"));
        // core.run(client.unary_call(req)
        //     .then(|result| {
        //         unimplemented!()
        //     })
        // )
        unimplemented!();
        // This line is just a hint for the type checker
        #[allow(unreachable_code)]
        {
            future::ok::<Vec<TestAssertion>, Box<Error>>(vec![])
        }
    }

    fn client_streaming_test(
        &mut self,
    ) -> impl Future<Item = Vec<TestAssertion>, Error = Box<Error>> {
        let requests = REQUEST_LENGTHS.iter().map(|len| StreamingInputCallRequest {
            payload: Some(util::client_payload(*len as usize)),
            ..Default::default()
        });
        let stream = stream::iter_ok(requests);
        self.test_client
            .streaming_input_call(Request::new(stream))
            .then(|result| {
                let mut assertions = vec![test_assert!(
                    "call must be successful",
                    result.is_ok(),
                    format!("result={:?}", result)
                )];
                if let Ok(response) = result.map(|r| r.into_inner()) {
                    assertions.push(test_assert!(
                        "aggregated payload size must be 74922 bytes",
                        response.aggregated_payload_size == 74922,
                        format!(
                            "aggregated_payload_size={:?}",
                            response.aggregated_payload_size
                        )
                    ));
                }
                future::ok::<Vec<TestAssertion>, Box<Error>>(assertions)
            })
    }

    fn server_streaming_test(
        &mut self,
    ) -> impl Future<Item = Vec<TestAssertion>, Error = Box<Error>> {
        use pb::ResponseParameters;
        let req = pb::StreamingOutputCallRequest {
            response_parameters: RESPONSE_LENGTHS
                .iter()
                .map(|len| ResponseParameters::with_size(*len))
                .collect(),
            ..Default::default()
        };
        let req = Request::new(req);
        self.test_client
            .streaming_output_call(req)
            .map_err(|tower_error| -> Box<Error> { Box::new(tower_error) })
            .and_then(|response_stream| {
                // Convert the stream into a plain Vec
                response_stream
                    .into_inner()
                    .collect()
                    .map_err(|tower_error| -> Box<Error> { Box::new(tower_error) })
            })
            .map(
                |responses: Vec<pb::StreamingOutputCallResponse>| -> Vec<TestAssertion> {
                    let actual_response_lengths = response_lengths(&responses);
                    vec![
                        test_assert!(
                            "there should be four responses",
                            responses.len() == 4,
                            format!("responses.len()={:?}", responses.len())
                        ),
                        test_assert!(
                            "the response payload sizes should match input",
                            RESPONSE_LENGTHS == actual_response_lengths.as_slice(),
                            format!("{:?}={:?}", RESPONSE_LENGTHS, actual_response_lengths)
                        ),
                    ]
                },
            )
            .then(&assert_success)
    }

    fn ping_pong_test(&mut self) -> impl Future<Item = Vec<TestAssertion>, Error = Box<Error>> {
        let (sender, receiver) = futures::sync::mpsc::unbounded::<pb::StreamingOutputCallRequest>();

        // Kick off the initial ping; without this the server does not
        // even start responding.
        sender.unbounded_send(make_ping_pong_request(0)).unwrap();

        self.test_client
            .full_duplex_call(Request::new(
                receiver.map_err(|_error| panic!("Receiver stream should not error!")),
            ))
            .map_err(|tower_error| -> Box<Error> { Box::new(tower_error) })
            .and_then(|response_stream| {
                PingPongState {
                    sender,
                    stream: Box::new(response_stream.into_inner()),
                    responses: vec![],
                    assertions: vec![],
                }
                .perform_ping_pong()
            })
            .map(|(responses, mut assertions)| {
                let actual_response_lengths = response_lengths(&responses);
                assertions.push(test_assert!(
                    "there should be four responses",
                    responses.len() == RESPONSE_LENGTHS.len(),
                    format!("{:?}={:?}", responses.len(), RESPONSE_LENGTHS.len())
                ));
                assertions.push(test_assert!(
                    "the response payload sizes should match input",
                    RESPONSE_LENGTHS == actual_response_lengths.as_slice(),
                    format!("{:?}={:?}", RESPONSE_LENGTHS, actual_response_lengths)
                ));
                assertions
            })
            .then(&assert_success)
    }

    fn empty_stream_test(&mut self) -> impl Future<Item = Vec<TestAssertion>, Error = Box<Error>> {
        let stream = stream::iter_ok(Vec::<pb::StreamingOutputCallRequest>::new());
        self.test_client
            .full_duplex_call(Request::new(stream))
            .map_err(|tower_error| -> Box<Error> { Box::new(tower_error) })
            .and_then(|response_stream| {
                // Convert the stream into a plain Vec
                response_stream
                    .into_inner()
                    .collect()
                    .map_err(|tower_error| -> Box<Error> { Box::new(tower_error) })
            })
            .map(
                |responses: Vec<pb::StreamingOutputCallResponse>| -> Vec<TestAssertion> {
                    vec![test_assert!(
                        "there should be no responses",
                        responses.len() == 0,
                        format!("responses.len()={:?}", responses.len())
                    )]
                },
            )
            .then(&assert_success)
    }

    fn status_code_and_message_test(
        &mut self,
    ) -> impl Future<Item = Vec<TestAssertion>, Error = Box<Error>> {
        fn validate_response<T>(
            result: Result<T, tower_grpc::Status>,
        ) -> future::FutureResult<Vec<TestAssertion>, Box<Error>>
        where
            T: fmt::Debug,
        {
            let assertions = vec![
                test_assert!(
                    "call must fail with unknown status code",
                    match &result {
                        Err(status) => status.code() == tower_grpc::Code::Unknown,
                        _ => false,
                    },
                    format!("result={:?}", result)
                ),
                test_assert!(
                    "call must repsond with expected status message",
                    match &result {
                        Err(status) => status.message() == TEST_STATUS_MESSAGE,
                        _ => false,
                    },
                    format!("result={:?}", result)
                ),
            ];
            future::ok::<Vec<TestAssertion>, Box<Error>>(assertions)
        }

        let simple_req = SimpleRequest {
            response_status: Some(pb::EchoStatus {
                code: 2,
                message: TEST_STATUS_MESSAGE.to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let duplex_req = pb::StreamingOutputCallRequest {
            response_status: Some(pb::EchoStatus {
                code: 2,
                message: TEST_STATUS_MESSAGE.to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        let unary_call = self
            .test_client
            .unary_call(Request::new(simple_req))
            .then(&validate_response);

        let full_duplex_call = self
            .test_client
            .full_duplex_call(Request::new(stream::iter_ok(vec![duplex_req])))
            .and_then(|response_stream| {
                // Convert the stream into a plain Vec
                response_stream.into_inner().map_err(From::from).collect()
            })
            .then(&validate_response);

        unary_call
            .join(full_duplex_call)
            .map(|(mut unary_assertions, mut streaming_assertions)| {
                unary_assertions.append(&mut streaming_assertions);
                unary_assertions
            })
    }

    fn special_status_message_test(
        &mut self,
    ) -> impl Future<Item = Vec<TestAssertion>, Error = Box<Error>> {
        fn validate_response<T>(
            result: Result<T, tower_grpc::Status>,
        ) -> future::FutureResult<Vec<TestAssertion>, Box<Error>>
        where
            T: fmt::Debug,
        {
            let assertions = vec![
                test_assert!(
                    "call must fail with unknown status code",
                    match &result {
                        Err(status) => status.code() == tower_grpc::Code::Unknown,
                        _ => false,
                    },
                    format!("result={:?}", result)
                ),
                test_assert!(
                    "call must repsond with expected status message",
                    match &result {
                        Err(status) => status.message() == SPECIAL_TEST_STATUS_MESSAGE,
                        _ => false,
                    },
                    format!("result={:?}", result)
                ),
            ];
            future::ok::<Vec<TestAssertion>, Box<Error>>(assertions)
        }

        let req = SimpleRequest {
            response_status: Some(pb::EchoStatus {
                code: 2,
                message: SPECIAL_TEST_STATUS_MESSAGE.to_string(),
                ..Default::default()
            }),
            ..Default::default()
        };

        self.test_client
            .unary_call(Request::new(req.clone()))
            .then(&validate_response)
    }

    fn unimplemented_method_test(
        &mut self,
    ) -> impl Future<Item = Vec<TestAssertion>, Error = Box<Error>> {
        use pb::Empty;

        self.test_client
            .unimplemented_call(Request::new(Empty {}))
            .then(|result| {
                let assertions = vec![test_assert!(
                    "call must fail with unimplemented status code",
                    match &result {
                        Err(status) => status.code() == tower_grpc::Code::Unimplemented,
                        _ => false,
                    },
                    format!("result={:?}", result)
                )];
                future::ok::<Vec<TestAssertion>, Box<Error>>(assertions)
            })
    }

    fn unimplemented_service_test(
        &mut self,
    ) -> impl Future<Item = Vec<TestAssertion>, Error = Box<Error>> {
        use pb::Empty;

        self.unimplemented_client
            .unimplemented_call(Request::new(Empty {}))
            .then(|result| {
                let assertions = vec![test_assert!(
                    "call must fail with unimplemented status code",
                    match &result {
                        Err(status) => status.code() == tower_grpc::Code::Unimplemented,
                        _ => false,
                    },
                    format!("result={:?}", result)
                )];
                future::ok::<Vec<TestAssertion>, Box<Error>>(assertions)
            })
    }
}

impl Testcase {
    fn run(
        &self,
        server: &ServerInfo,
        core: &mut tokio_core::reactor::Core,
    ) -> Result<Vec<TestAssertion>, Box<Error>> {
        let open_connection = |core: &mut tokio_core::reactor::Core| {
            let reactor = core.handle();
            core.run(
                TcpStream::connect(&server.addr, &reactor)
                    .and_then(move |socket| {
                        // Bind the HTTP/2.0 connection
                        Connection::handshake(socket, reactor)
                            .map_err(|_| panic!("failed HTTP/2.0 handshake"))
                    })
                    .map(move |conn| {
                        tower_add_origin::Builder::new()
                            .uri(server.uri.clone())
                            .build(conn)
                            .unwrap()
                    }),
            )
            .expect("connection")
        };

        // TODO(#42): This opens two separate TCP connections to the server. It
        // would be better to open only one.
        let mut clients = TestClients {
            test_client: TestService::new(open_connection(core)),
            unimplemented_client: UnimplementedService::new(open_connection(core)),
        };

        match *self {
            Testcase::empty_unary => core.run(clients.empty_unary_test()),
            Testcase::large_unary => core.run(clients.large_unary_test()),
            Testcase::cacheable_unary => core.run(clients.cacheable_unary_test()),
            Testcase::client_streaming => core.run(clients.client_streaming_test()),
            Testcase::server_streaming => core.run(clients.server_streaming_test()),
            Testcase::ping_pong => core.run(clients.ping_pong_test()),
            Testcase::empty_stream => core.run(clients.empty_stream_test()),
            Testcase::status_code_and_message => core.run(clients.status_code_and_message_test()),
            Testcase::special_status_message => core.run(clients.special_status_message_test()),
            Testcase::unimplemented_method => core.run(clients.unimplemented_method_test()),
            Testcase::unimplemented_service => core.run(clients.unimplemented_service_test()),
            Testcase::compute_engine_creds
            | Testcase::jwt_token_creds
            | Testcase::oauth2_auth_token
            | Testcase::per_rpc_creds => unimplemented!(
                "test case unimplemented: tower-grpc does not \
                 currently support gRPC authorization."
            ),
            Testcase::client_compressed_unary
            | Testcase::server_compressed_unary
            | Testcase::client_compressed_streaming
            | Testcase::server_compressed_streaming => unimplemented!(
                "test case unimplemented: tower-grpc does not \
                 currently support gRPC compression."
            ),

            _ => unimplemented!("test case unimplemented: {}", *self),
        }
    }
}
#[derive(Debug)]
enum TestAssertion {
    Passed {
        description: &'static str,
    },
    Failed {
        description: &'static str,
        expression: &'static str,
        why: Option<String>,
    },
    Errored {
        description: &'static str,
        error: Box<Error>,
    },
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
        use console::{style, Emoji};
        match *self {
            TestAssertion::Passed { ref description } => write!(
                f,
                "{check} {desc}",
                check = style(Emoji("✔", "+")).green(),
                desc = style(description).green(),
            ),
            TestAssertion::Failed {
                ref description,
                ref expression,
                why: Some(ref why),
            } => write!(
                f,
                "{check} {desc}\n  in `{exp}`: {why}",
                check = style(Emoji("✖", "x")).red(),
                desc = style(description).red(),
                exp = style(expression).red(),
                why = style(why).red(),
            ),
            TestAssertion::Failed {
                ref description,
                ref expression,
                why: None,
            } => write!(
                f,
                "{check} {desc}\n  in `{exp}`",
                check = style(Emoji("✖", "x")).red(),
                desc = style(description).red(),
                exp = style(expression).red(),
            ),
            _ => unimplemented!(),
        }
    }
}

struct ServerInfo {
    addr: SocketAddr,
    uri: Uri,
    hostname_override: Option<String>,
}

impl ServerInfo {
    fn from_args<'a>(
        matches: &clap::ArgMatches<'a>,
        core: &mut reactor::Core,
    ) -> Result<Self, ClientError> {
        use domain::bits::DNameBuf;
        use domain::resolv::{lookup, Resolver};

        let handle = core.handle();
        // XXX this could probably look neater if only the DNS query was run in
        //     a future...
        let ip_future = future::result(value_t!(matches, "server_host", IpAddr))
            .from_err::<ClientError>()
            .or_else(|_| {
                future::result(value_t!(matches, "server_host", DNameBuf))
                    .from_err::<ClientError>()
                    .and_then(move |name| {
                        let resolver = Resolver::new(&handle);
                        lookup::lookup_host(resolver, name)
                            .from_err::<ClientError>()
                            .and_then(|response| {
                                response
                                    .iter()
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
                    let host = matches.value_of("server_host").expect(
                        "`server_host` argument was not present, clap \
                         should have already validated it was present.",
                    );
                    future::result(Uri::from_str(&format!("http://{}:{}", host, port)))
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
    use clap::{App, Arg};
    let _ = ::pretty_env_logger::init();

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

    if matches.is_present("oauth_scope")
        || matches.is_present("default_service_account")
        || matches.is_present("service_account_key_file")
    {
        unimplemented!("tower-grpc does not currently support GCE auth.");
    }

    let mut core = reactor::Core::new().expect("could not create reactor core!");

    let server = ServerInfo::from_args(&matches, &mut core).unwrap_or_else(|e| e.exit());

    let test_cases = values_t!(matches, "test_case", Testcase).unwrap_or_else(|e| e.exit());

    for test in test_cases {
        println!("{:?}:", test);
        let test_results = test.run(&server, &mut core).expect("error running test!");
        for result in test_results {
            println!("  {}", result);
        }
    }
}
