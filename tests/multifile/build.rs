extern crate tower_grpc_build;

fn main() {
    // Build multifile
    tower_grpc_build::Config::new()
        .enable_server(true)
        .enable_client(true)
        .build(&["proto/hello.proto",
                 "proto/world.proto"],
               &["proto"])
        .unwrap_or_else(|e| panic!("protobuf compilation failed: {}", e));
}

