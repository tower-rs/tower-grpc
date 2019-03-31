extern crate tower_grpc_build;

fn main() {
    // Build unused-imports
    tower_grpc_build::Config::new()
        .enable_server(true)
        .enable_client(true)
        .build(
            &[
                "proto/client_streaming.proto",
                &"proto/server_streaming.proto",
                &"proto/bidi.proto",
            ],
            &["proto"],
        )
        .unwrap_or_else(|e| panic!("protobuf compilation failed: {}", e));
}
