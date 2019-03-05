/// Type re-exports used by generated server code
pub mod server {
    /// Re-export types from this crate
    pub mod grpc {
        pub use ::{Body, BoxBody, Request, Response, Code, Status};
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
        pub use ::bytes::{Bytes, IntoBuf};
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

    /// Re-exported types from the `tower` crate.
    pub mod tower {
        pub use ::tower_service::Service;
        pub use ::tower_util::MakeService;
        pub use ::tower_http_service::{Body as HttpBody};
    }

    #[cfg(feature = "tower-h2")]
    /// Re-exported types from `tower-h2` crate.
    pub mod tower_h2 {
        pub use ::tower_h2::{Body, RecvBody};
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
        pub use ::{Body, Request, Response, Code, Status};
    }

    pub mod http {
        pub use ::http::uri::{Uri, PathAndQuery};
    }

    /// Re-export types from the `future` crate.
    pub mod futures {
        pub use ::futures::{Future, Poll};
    }

    pub mod tower {
        pub use ::tower_http_service::{Body as HttpBody, HttpService};
    }
}
