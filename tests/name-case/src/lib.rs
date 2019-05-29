extern crate bytes;
extern crate prost;
extern crate tower_grpc;
extern crate tower_hyper;

pub mod hello {
    include!(concat!(env!("OUT_DIR"), "/hello.rs"));
}
