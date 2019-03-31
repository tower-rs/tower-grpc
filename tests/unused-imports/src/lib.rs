//! Reproduction for https://github.com/tower-rs/tower-grpc/issues/56
#![deny(warnings)]

extern crate bytes;
extern crate prost;
extern crate tower_grpc;
extern crate tower_h2;

pub mod server_streaming {
    include!(concat!(env!("OUT_DIR"), "/server_streaming.rs"));
}
pub mod client_streaming {
    include!(concat!(env!("OUT_DIR"), "/client_streaming.rs"));
}
pub mod bidi {
    include!(concat!(env!("OUT_DIR"), "/bidi.rs"));
}

#[cfg(test)]
mod tests {
    use std::mem;

    #[test]
    fn types_are_present() {
        mem::size_of::<::server_streaming::HelloRequest>();
        mem::size_of::<::client_streaming::HelloRequest>();
        mem::size_of::<::bidi::HelloRequest>();
    }
}
