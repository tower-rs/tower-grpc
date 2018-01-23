/// Type re-exports used by generated server code
pub mod server {
    /// Re-export types from this crate
    pub mod grpc {
        pub use ::{Request, Response, Error, Status};
        pub use ::generic::server::{
            StreamingService,
            UnaryService,
            ClientStreamingService,
            ServerStreamingService,
        };
        pub use ::server::{
            Grpc,
            unary,
            client_streaming,
            server_streaming,
            streaming,
        };
        pub use ::codec::{
            Encode,
            Streaming,
        };
    }

    /// Re-export types from the `bytes` crate.
    pub mod bytes {
        pub use ::bytes::Bytes;
    }

    /// Re-export types from the `future` crate.
    pub mod futures {
        pub use ::futures::{Future, Stream, Poll, Async};
        pub use ::futures::future::{FutureResult, ok};
    }

    /// Re-exported types from the `http` crate.
    pub mod http {
        pub use ::http::{Request, Response, HeaderMap};
    }

    /// Re-exported types from the `h2` crate.
    pub mod h2 {
        pub use ::h2::Error;
    }

    /// Re-export types from the `tower_h2` crate
    pub mod tower_h2 {
        pub use ::tower_h2::{Body, RecvBody};
    }

    /// Re-exported types from the `tower` crate.
    pub mod tower {
        pub use ::tower::{Service, ReadyService, NewService};
    }
}

pub mod client {
    /// Re-export types from this crate
    pub mod grpc {
        pub use ::client::{
            Grpc,
            Encodable,
            unary,
            client_streaming,
            server_streaming,
            streaming,
        };
        pub use ::{Request, Response, Error, Status};
    }

    pub mod http {
        pub use ::http::uri::{Uri, PathAndQuery};
    }

    /// Re-export types from the `future` crate.
    pub mod futures {
        pub use ::futures::{Future, Poll};
    }

    pub mod tower_h2 {
        pub use ::tower_h2::HttpService;
    }
}
