/// Type re-exports used by generated server code
pub mod server {
    /// Re-export types from this crate
    pub mod grpc {
        pub use codec::{Encode, Streaming};
        pub use generic::server::{
            ClientStreamingService, ServerStreamingService, StreamingService, UnaryService,
        };
        pub use server::{client_streaming, server_streaming, streaming, unary, unimplemented};
        pub use {error::Never, Body, BoxBody, Code, Request, Response, Status};
    }

    /// Re-export types from the `future` crate.
    pub mod futures {
        pub use futures::future::{ok, FutureResult};
        pub use futures::{Async, Future, Poll, Stream};
    }

    /// Re-exported types from the `http` crate.
    pub mod http {
        pub use http::{HeaderMap, Request, Response};
    }

    /// Re-exported types from the `tower` crate.
    pub mod tower {
        pub use tower_http::Body as HttpBody;
        pub use tower_service::Service;
        pub use tower_util::MakeService;
    }

    #[cfg(feature = "tower-h2")]
    /// Re-exported types from `tower-h2` crate.
    pub mod tower_h2 {
        pub use tower_h2::{Body, RecvBody};
    }
}

pub mod client {
    /// Re-export types from this crate
    pub mod grpc {
        pub use client::{client_streaming, server_streaming, streaming, unary, Encodable, Grpc};
        pub use generic::client::GrpcService;
        pub use {Body, Code, Request, Response, Status};
    }

    pub mod http {
        pub use http::uri::{PathAndQuery, Uri};
    }

    /// Re-export types from the `future` crate.
    pub mod futures {
        pub use futures::{Future, Poll, Stream};
    }

    pub mod tower {
        pub use tower_http::Body as HttpBody;
    }
}
