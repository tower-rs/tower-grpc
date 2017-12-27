use ::Request;
use super::{streaming, server_streaming, client_streaming, unary};
use generic::{Codec, Streaming};
use generic::server::{StreamingService, ServerStreamingService, ClientStreamingService, UnaryService};

use http;
use tower_h2::{Body, Data};

#[derive(Debug, Clone)]
pub struct Grpc<T> {
    codec: T,
}

// ===== impl Grpc =====

impl<T> Grpc<T>
where T: Codec,
{
    pub fn new(codec: T) -> Self {
        Grpc { codec }
    }

    pub fn unary<S, B>(&mut self,
                       service: S,
                       request: http::Request<B>)
        -> unary::ResponseFuture<S, T::Encoder, Streaming<T::Decoder, B>>
    where S: UnaryService<Request = T::Decode,
                         Response = T::Encode>,
          B: Body<Data = Data>,
    {
        let request = self.map_request(request);
        unary::ResponseFuture::new(service, request, self.codec.encoder())
    }

    pub fn client_streaming<S, B>(&mut self,
                               service: &mut S,
                               request: http::Request<B>)
        -> client_streaming::ResponseFuture<S::Future, T::Encoder>
    where S: ClientStreamingService<Request = T::Decode,
                              RequestStream = Streaming<T::Decoder, B>,
                                   Response = T::Encode>,
          B: Body<Data = Data>,
    {
        let response = service.call(self.map_request(request));
        client_streaming::ResponseFuture::new(response, self.codec.encoder())
    }

    pub fn server_streaming<S, B>(&mut self,
                                  service: S,
                                  request: http::Request<B>)
        -> server_streaming::ResponseFuture<S, T::Encoder, Streaming<T::Decoder, B>>
    where S: ServerStreamingService<Request = T::Decode,
                                   Response = T::Encode>,
          B: Body<Data = Data>,
    {
        let request = self.map_request(request);
        server_streaming::ResponseFuture::new(service, request, self.codec.encoder())
    }

    pub fn streaming<S, B>(&mut self,
                           service: &mut S,
                           request: http::Request<B>)
        -> streaming::ResponseFuture<S::Future, T::Encoder>
    where S: StreamingService<Request = T::Decode,
                        RequestStream = Streaming<T::Decoder, B>,
                             Response = T::Encode>,
          B: Body<Data = Data>,
    {
        let response = service.call(self.map_request(request));
        streaming::ResponseFuture::new(response, self.codec.encoder())
    }

    /// Map an inbound HTTP request to a streaming decoded request
    fn map_request<B>(&mut self, request: http::Request<B>)
        -> Request<Streaming<T::Decoder, B>>
    where B: Body<Data = Data>,
    {
        use http::header::{self, HeaderValue};

        // Map the request body
        let (head, body) = request.into_parts();

        // Wrap the body stream with a decoder
        let body = Streaming::new(self.codec.decoder(), body, false);

        // Reconstruct the HTTP request
        let mut request = http::Request::from_parts(head, body);

        // Set the content type
        request.headers_mut().insert(header::CONTENT_TYPE, HeaderValue::from_static(T::CONTENT_TYPE));

        // Convert the HTTP request to a gRPC request
        Request::from_http(request)
    }
}
