extern crate tower_grpc_build;

fn main() {
    // Build grpc-interop
    tower_grpc_build::Config::new()
        .enable_server(true)
        .enable_client(true)
        .build(&["proto/grpc/testing/test.proto"], &["proto/grpc/testing"])
        .unwrap_or_else(|e| panic!("protobuf compilation failed: {}", e));
}