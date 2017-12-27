#[macro_use]
extern crate clap;
#[macro_use]
extern crate log;
extern crate env_logger;
extern crate prost;
#[macro_use]
extern crate prost_derive;
extern crate tokio_core;
extern crate tower;
extern crate tower_h2;
extern crate tower_grpc;

use std::net::{IpAddr, SocketAddr};
use futures::Future;
use tokio_core::reactor::Core;
use tokio_core::net::TcpStream;
use tower_grpc::Request;
use tower_h2::client::Connection;

mod test {
    include!(concat!(env!("OUT_DIR"), "/grpc.testing.rs"));
}

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

impl Testcase {
    fn run(&self, server_addr: SocketAddr) -> io::Result<()> {

        let mut core = Core::new()?;
        let reactor = core.handle();
        match *self {
            t => unimplemented!("test case {:?} is not yet implemented.", t),
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
                .default_value("8080")
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

    let server_addr = value_t!(matches.value_of("server_host"), IpAddr)
        .and_then(|ip| 
            value_t!(matches.value_of("server_port"), u16)
                .map(|port| 
                    SocketAddr::new(ip, port)
                )
        )
        .unwrap_or_else(|e| e.exit());
    
    
    let server_addr = unimplemented!();

    let test_case = value_t!(matches.value_of("test_case"), Testcase)
        .unwrap_or_else(|e| e.exit());

    test_case.run(server_addr).unwrap();
}