//! Reproduction for https://github.com/tower-rs/tower-grpc/issues/56
#![deny(warnings)]

extern crate bytes;
extern crate prost;
#[macro_use]
extern crate prost_derive;
extern crate tower_h2;
extern crate tower_grpc;

pub mod hello {
    include!(concat!(env!("OUT_DIR"), "/hello.rs"));
}

#[cfg(test)]
mod tests {
    use std::mem;

    #[test]
    fn types_are_present() {
        mem::size_of::<::hello::HelloRequest>();
    }

    #[test]
    fn can_call() {
        use ::hello::{HelloRequest};
        use ::hello::client::Hello;
        use ::tower_h2::BoxBody;
        use ::tower_grpc::codegen::client::*;

        #[allow(dead_code)]
        fn zomg<T>(client: &mut Hello<T>)
        where T: tower_h2::HttpService<RequestBody = BoxBody>,
        {
            let request = HelloRequest {
                name: "hello".to_string(),
            };

            let _ = client.say_hello(grpc::Request::new(request.clone()));
        }
    }
}
