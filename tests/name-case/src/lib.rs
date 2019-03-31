extern crate bytes;
extern crate prost;
extern crate tower_grpc;
extern crate tower_h2;

pub mod hello {
    include!(concat!(env!("OUT_DIR"), "/hello.rs"));
}
