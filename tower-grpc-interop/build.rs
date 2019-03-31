extern crate tower_grpc_build;

fn main() {
    let files = &["proto/grpc/testing/test.proto"];
    let dirs = &["proto/grpc/testing"];

    // Build grpc-interop
    tower_grpc_build::Config::new()
        .enable_server(true)
        .enable_client(true)
        .build(files, dirs)
        .unwrap_or_else(|e| panic!("protobuf compilation failed: {}", e));

    // prevent needing to rebuild if files (or deps) haven't changed
    for file in files {
        println!("cargo:rerun-if-changed={}", file);
    }
}
