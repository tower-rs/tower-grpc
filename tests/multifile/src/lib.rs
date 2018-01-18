extern crate bytes;
extern crate prost;
#[macro_use]
extern crate prost_derive;
extern crate tower_grpc;

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
}
