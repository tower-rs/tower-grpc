/// Type re-exports used by generated server code
pub mod server {
    /// Re-export types from this crate
    pub mod grpc {
        pub use crate::codec::{Encode, Streaming};
        pub use crate::generic::server::{
            ClientStreamingService, ServerStreamingService, StreamingService, UnaryService,
        };
        pub use crate::server::{
            client_streaming, server_streaming, streaming, unary, unimplemented,
        };
        pub use crate::{error::Never, Body, BoxBody, Code, Request, Response, Status};
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
        pub use http_body::Body as HttpBody;
        pub use tower_service::Service;
        pub use tower_util::MakeService;
    }

    #[cfg(feature = "tower-hyper")]
    /// Re-exported types from `tower-hyper` crate.
    pub mod tower_hyper {
        pub use tower_hyper::Body;
    }
}

pub mod client {
    /// Re-export types from this crate
    pub mod grpc {
        pub use crate::client::{
            client_streaming, server_streaming, streaming, unary, Encodable, Grpc,
        };
        pub use crate::generic::client::GrpcService;
        pub use crate::{Body, Code, Request, Response, Status};
    }

    pub mod http {
        pub use http::uri::{PathAndQuery, Uri};
    }

    /// Re-export types from the `future` crate.
    pub mod futures {
        pub use futures::{Future, Poll, Stream};
    }

    pub mod tower {
        pub use http_body::Body as HttpBody;
    }
}
