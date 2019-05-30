pub mod client_streaming;
pub mod server_streaming;
pub mod streaming;
pub mod unary;

use crate::body::BoxBody;
use crate::generic::client::{GrpcService, IntoService};

use futures::{stream, Future, Poll, Stream};
use http::{uri, Uri};
use prost::Message;

#[derive(Debug, Clone)]
pub struct Grpc<T> {
    /// The inner HTTP/2.0 service.
    inner: T,
}

/// Convert a stream of protobuf messages to an HTTP body payload.
///
/// TODO: Rename to `IntoEncode` or something...
pub trait Encodable<T> {
    fn into_encode(self) -> T;
}

// ===== impl Grpc =====

impl<T> Grpc<T> {
    /// Create a new `Grpc` instance backed by the given HTTP service.
    pub fn new(inner: T) -> Self {
        Grpc { inner }
    }

    pub fn poll_ready<R>(&mut self) -> Poll<(), crate::Status>
    where
        T: GrpcService<R>,
    {
        self.inner
            .poll_ready()
            .map_err(|err| crate::Status::from_error(&*(err.into())))
    }

    pub fn ready<R>(self) -> impl Future<Item = Self, Error = crate::Status>
    where
        T: GrpcService<R>,
    {
        use tower_util::Ready;
        Ready::new(self.inner.into_service())
            .map(|IntoService(inner)| Grpc { inner })
            .map_err(|err| crate::Status::from_error(&*(err.into())))
    }

    pub fn unary<M1, M2, R>(
        &mut self,
        request: crate::Request<M1>,
        path: uri::PathAndQuery,
    ) -> unary::ResponseFuture<M2, T::Future, T::ResponseBody>
    where
        T: GrpcService<R>,
        unary::Once<M1>: Encodable<R>,
    {
        let request = request.map(|v| stream::once(Ok(v)));
        let response = self.client_streaming(request, path);

        unary::ResponseFuture::new(response)
    }

    pub fn client_streaming<B, M, R>(
        &mut self,
        request: crate::Request<B>,
        path: uri::PathAndQuery,
    ) -> client_streaming::ResponseFuture<M, T::Future, T::ResponseBody>
    where
        T: GrpcService<R>,
        B: Encodable<R>,
    {
        let response = self.streaming(request, path);
        client_streaming::ResponseFuture::new(response)
    }

    pub fn server_streaming<M1, M2, R>(
        &mut self,
        request: crate::Request<M1>,
        path: uri::PathAndQuery,
    ) -> server_streaming::ResponseFuture<M2, T::Future>
    where
        T: GrpcService<R>,
        unary::Once<M1>: Encodable<R>,
    {
        let request = request.map(|v| stream::once(Ok(v)));
        let response = self.streaming(request, path);

        server_streaming::ResponseFuture::new(response)
    }

    /// Initiate a full streaming gRPC request
    ///
    /// # Generics
    ///
    /// **B**: The request stream of gRPC message values.
    /// **M**: The response **message** (not stream) type.
    /// **R**: The type of the request body.
    pub fn streaming<B, M, R>(
        &mut self,
        request: crate::Request<B>,
        path: uri::PathAndQuery,
    ) -> streaming::ResponseFuture<M, T::Future>
    where
        T: GrpcService<R>,
        B: Encodable<R>,
    {
        use http::header::{self, HeaderValue};

        // TODO: validate the path

        // Get the gRPC's method URI
        let mut parts = uri::Parts::default();
        parts.path_and_query = Some(path);

        // Get the URI;
        let uri = Uri::from_parts(parts).expect("path_and_query only is valid Uri");

        // Convert the request body
        let request = request.map(Encodable::into_encode);

        // Convert to an HTTP request
        let mut request = request.into_http(uri);

        // Add the gRPC related HTTP headers
        request
            .headers_mut()
            .insert(header::TE, HeaderValue::from_static("trailers"));

        // Set the content type
        // TODO: Don't hard code this here
        let content_type = "application/grpc+proto";
        request
            .headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));

        // Call the inner HTTP service
        let response = self.inner.call(request);

        streaming::ResponseFuture::new(response)
    }
}

// ===== impl Encodable =====

impl<T, U> Encodable<BoxBody> for T
where
    T: Stream<Item = U, Error = crate::Status> + Send + 'static,
    U: Message + 'static,
{
    fn into_encode(self) -> BoxBody {
        use crate::codec::Encoder;
        use crate::generic::Encode;

        let encode = Encode::request(Encoder::new(), self);
        BoxBody::new(Box::new(encode))
    }
}
