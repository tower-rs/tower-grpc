pub mod unary;
pub mod client_streaming;
pub mod server_streaming;
pub mod streaming;

use Status;

use futures::{stream, Stream, Poll};
use http::{uri, HeaderMap, Uri};
use prost::Message;
use tower_h2::{HttpService, BoxBody};

#[derive(Debug)]
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

impl<T> Grpc<T>
where T: HttpService,
{
    /// Create a new `Grpc` instance backed by the given HTTP service.
    pub fn new(inner: T) -> Self {
        Grpc { inner }
    }

    pub fn poll_ready(&mut self) -> Poll<(), ::Error<T::Error>> {
        self.inner.poll_ready()
            .map_err(::Error::Inner)
    }

    pub fn unary<M1, M2>(&mut self,
                         request: ::Request<M1>,
                         path: uri::PathAndQuery)
        -> unary::ResponseFuture<M2, T::Future, T::ResponseBody>
    where unary::Once<M1>: Encodable<T::RequestBody>,
    {
        let request = request.map(|v| stream::once(Ok(v)));
        let response = self.client_streaming(request, path);

        unary::ResponseFuture::new(response)
    }

    pub fn client_streaming<B, M>(&mut self,
                                  request: ::Request<B>,
                                  path: uri::PathAndQuery)
        -> client_streaming::ResponseFuture<M, T::Future, T::ResponseBody>
    where B: Encodable<T::RequestBody>,
    {
        let response = self.streaming(request, path);
        client_streaming::ResponseFuture::new(response)
    }

    pub fn server_streaming<M1, M2>(&mut self,
                                    request: ::Request<M1>,
                                    path: uri::PathAndQuery)
        -> server_streaming::ResponseFuture<M2, T::Future>
    where unary::Once<M1>: Encodable<T::RequestBody>,
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
    pub fn streaming<B, M>(&mut self,
                           request: ::Request<B>,
                           path: uri::PathAndQuery)
        -> streaming::ResponseFuture<M, T::Future>
    where B: Encodable<T::RequestBody>,
    {
        use http::header::{self, HeaderValue};

        // TODO: validate the path

        // Get the gRPC's method URI
        let mut parts = uri::Parts::default();
        parts.path_and_query = Some(path);

        // Get the URI;
        let uri = match Uri::from_parts(parts) {
            Ok(uri) => uri,
            Err(_) => unimplemented!(),
        };

        // Convert the request body
        let request = request.map(Encodable::into_encode);

        // Convert to an HTTP request
        let mut request = request.into_http(uri);

        // Add the gRPC related HTTP headers
        request.headers_mut()
            .insert(header::TE, HeaderValue::from_static("trailers"));

        // Set the content type
        // TODO: Don't hard code this here
        let content_type = "application/grpc+proto";
        request.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(content_type));

        // Call the inner HTTP service
        let response = self.inner.call(request);

        streaming::ResponseFuture::new(response)
    }
}

// ===== impl Encodable =====

impl<T, U> Encodable<BoxBody> for T
where T: Stream<Item = U, Error = ::Error> + Send + 'static,
      U: Message + 'static,
{
    fn into_encode(self) -> BoxBody {
        use codec::Encoder;
        use generic::Encode;

        let encode = Encode::new(Encoder::new(), self, false);
        BoxBody::new(Box::new(encode))
    }
}

// ===== utility fns =====

fn check_grpc_status(trailers: &HeaderMap) -> Option<Status> {
    trailers.get("grpc-status").map(|s| {
        Status::from_bytes(s.as_ref())
    })
}
