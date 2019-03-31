extern crate bytes;
extern crate prost;
extern crate tower_grpc;
extern crate tower_h2;

pub mod hello {
    include!(concat!(env!("OUT_DIR"), "/hello.rs"));
}

pub mod world {
    include!(concat!(env!("OUT_DIR"), "/world.rs"));
}

#[cfg(test)]
mod tests {
    use std::mem;

    #[test]
    fn types_are_present() {
        mem::size_of::<::hello::HelloRequest>();
        mem::size_of::<::world::WorldRequest>();
    }

    #[test]
    fn can_call() {
        use hello::client::Hello;
        use hello::HelloRequest;
        use tower_grpc::codegen::client::*;
        use tower_h2::BoxBody;

        #[allow(dead_code)]
        fn zomg<T>(client: &mut Hello<T>)
        where
            T: tower_h2::HttpService<BoxBody>,
        {
            let request = HelloRequest {
                name: "hello".to_string(),
            };

            let _ = client.say_hello(grpc::Request::new(request.clone()));
            let _ = client.say_hello2(grpc::Request::new(request.clone()));
        }
    }
}
