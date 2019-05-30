use super::{client_streaming, server_streaming, streaming, unary};
use crate::generic::server::{
    ClientStreamingService, ServerStreamingService, StreamingService, UnaryService,
};
use crate::generic::{Codec, Direction, Streaming};
use crate::{Body, Request};

#[derive(Debug, Clone)]
pub(crate) struct Grpc<T> {
    codec: T,
}

// ===== impl Grpc =====

impl<T> Grpc<T>
where
    T: Codec,
{
    pub(crate) fn new(codec: T) -> Self {
        Grpc { codec }
    }

    pub(crate) fn unary<S, B>(
        &mut self,
        service: S,
        request: http::Request<B>,
    ) -> unary::ResponseFuture<S, T::Encoder, Streaming<T::Decoder, B>>
    where
        S: UnaryService<T::Decode, Response = T::Encode>,
        B: Body,
    {
        let request = self.map_request(request);
        unary::ResponseFuture::new(service, request, self.codec.encoder())
    }

    pub(crate) fn client_streaming<S, B>(
        &mut self,
        service: &mut S,
        request: http::Request<B>,
    ) -> client_streaming::ResponseFuture<S::Future, T::Encoder>
    where
        S: ClientStreamingService<Streaming<T::Decoder, B>, Response = T::Encode>,
        B: Body,
    {
        let response = service.call(self.map_request(request));
        client_streaming::ResponseFuture::new(response, self.codec.encoder())
    }

    pub(crate) fn server_streaming<S, B>(
        &mut self,
        service: S,
        request: http::Request<B>,
    ) -> server_streaming::ResponseFuture<S, T::Encoder, Streaming<T::Decoder, B>>
    where
        S: ServerStreamingService<T::Decode, Response = T::Encode>,
        B: Body,
    {
        let request = self.map_request(request);
        server_streaming::ResponseFuture::new(service, request, self.codec.encoder())
    }

    pub(crate) fn streaming<S, B>(
        &mut self,
        service: &mut S,
        request: http::Request<B>,
    ) -> streaming::ResponseFuture<S::Future, T::Encoder>
    where
        S: StreamingService<Streaming<T::Decoder, B>, Response = T::Encode>,
        B: Body,
    {
        let response = service.call(self.map_request(request));
        streaming::ResponseFuture::new(response, self.codec.encoder())
    }

    /// Map an inbound HTTP request to a streaming decoded request
    fn map_request<B>(&mut self, request: http::Request<B>) -> Request<Streaming<T::Decoder, B>>
    where
        B: Body,
    {
        Request::from_http(
            request.map(|body| Streaming::new(self.codec.decoder(), body, Direction::Request)),
        )
    }
}
