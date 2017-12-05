pub mod client_streaming;
pub mod server_streaming;
pub mod streaming;
pub mod unary;

use codec::{Codec, Streaming};
use generic::server::{UnaryService, ClientStreamingService, ServerStreamingService, StreamingService};

use http;
use prost;
use tower_h2::{Body, Data};

#[derive(Debug, Clone)]
pub struct Grpc {
    _p: (),
}

// ===== impl Grpc =====

impl Grpc {
    pub fn unary<T, B>(service: T,
                       request: http::Request<B>)
        -> unary::ResponseFuture<T, B>
    where T: UnaryService,
          T::Request: prost::Message + Default,
          T::Response: prost::Message,
          B: Body<Data = Data>,
    {
        use generic::server::Grpc;

        let mut grpc = Grpc::new(Codec::new());
        let inner = grpc.unary(service, request);
        unary::ResponseFuture::new(inner)
    }

    pub fn client_streaming<T, R, B>(service: &mut T,
                                     request: http::Request<B>)
        -> client_streaming::ResponseFuture<T>
    where T: ClientStreamingService<Request = R, RequestStream = Streaming<R, B>>,
          T::Request: prost::Message + Default,
          T::Response: prost::Message,
          B: Body<Data = Data>,
    {
        use generic::server::Grpc;

        let mut grpc = Grpc::new(Codec::new());
        let inner = grpc.client_streaming(service, request);
        client_streaming::ResponseFuture::new(inner)
    }

    pub fn server_streaming<T, B>(service: T,
                                  request: http::Request<B>)
        -> server_streaming::ResponseFuture<T, B>
    where T: ServerStreamingService,
          T::Request: prost::Message + Default,
          T::Response: prost::Message,
          B: Body<Data = Data>,
    {
        use generic::server::Grpc;

        let mut grpc = Grpc::new(Codec::new());
        let inner = grpc.server_streaming(service, request);
        server_streaming::ResponseFuture::new(inner)
    }

    pub fn streaming<T, R, B>(service: &mut T,
                              request: http::Request<B>)
        -> streaming::ResponseFuture<T>
    where T: StreamingService<Request = R, RequestStream = Streaming<R, B>>,
          T::Request: prost::Message + Default,
          T::Response: prost::Message,
          B: Body<Data = Data>,
    {
        use generic::server::Grpc;

        let mut grpc = Grpc::new(Codec::new());
        let inner = grpc.streaming(service, request);
        streaming::ResponseFuture::new(inner)
    }
}
