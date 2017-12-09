#![allow(warnings, missing_debug_implementations)]

use Status;
use util::BoxBody;

use futures::{stream, Future, Stream, Poll};
use http::{Request, Response, HeaderMap};
use tower::Service;
use tower_h2::Body;

use std::marker::PhantomData;

#[derive(Debug)]
pub struct Grpc<T, B = BoxBody> {
    inner: T,
    _m: PhantomData<B>,
}

/// An HTTP (2.0) service that backs the gRPC client
pub trait HttpService {
    type RequestBody: Body;
    type ResponseBody: Body;
    type Error;
    type Future: Future<Item = Response<Self::ResponseBody>, Error = Self::Error>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error>;

    fn call(&mut self, request: Request<Self::RequestBody>) -> Self::Future;
}

impl<T, B1, B2> HttpService for T
where T: Service<Request = Request<B1>,
                Response = Response<B2>>,
      B1: Body,
      B2: Body,
{
    type RequestBody = B1;
    type ResponseBody = B2;
    type Error = T::Error;
    type Future = T::Future;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Service::poll_ready(self)
    }

    fn call(&mut self, request: Request<Self::RequestBody>) -> Self::Future {
        Service::call(self, request)
    }
}

/// Convert a stream of protobuf messages to an HTTP body payload.
pub trait IntoBody<T>
{
    fn into_body(self) -> T;
}

pub type Once<T> = stream::Once<T, ::Error>;

impl<T> Grpc<T>
{
    pub fn new(inner: T) -> Self {
        Grpc {
            inner,
            _m: PhantomData,
        }
    }
}

impl<T, U> Grpc<T, U>
where T: HttpService<RequestBody = U>,
      U: Body,
{
    pub fn poll_ready(&mut self) -> Poll<(), ::Error<T::Error>> {
        self.inner.poll_ready()
            .map_err(::Error::Inner)
    }

    /*
    pub fn unary<M>(&mut self, request: ::Request<M>) -> unary::ResponseFuture
    where Once<M>: IntoBody<U>,
    {
        let response = self.client_streaming(
            request.map(|v| stream::once(Ok(v))));

        unary::ResponseFuture { inner: response }
    }

    pub fn client_streaming<B, M>(&mut self, request: ::Request<B>)
        -> client_streaming::ResponseFuture<M, T::Future>
    where B: IntoBody<U>,
    {
        // Convert the request
        let request = request.into_http();
        let (head, body) = request.into_parts();
        let body = body.into_body();
        let request = Request::from_parts(head, body);

        // Call the inner HTTP service
        let response = self.inner.call(request);

        client_streaming::ResponseFuture::new(response)
    }
    */

    /// Initiate a full streaming gRPC request
    ///
    /// # Generics
    ///
    /// **B**: The request stream of gRPC message values.
    /// **M**: The response **message** (not stream) type.
    pub fn streaming<B, M>(&mut self, request: ::Request<B>)
        -> streaming::ResponseFuture<M, T::Future>
    where B: Stream + IntoBody<U>,
    {
        use http::header::{self, HeaderValue};

        // Convert the request
        let request = request.into_http();
        let (head, body) = request.into_parts();
        let body = body.into_body();
        let mut request = Request::from_parts(head, body);

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

/*
pub mod unary {
    use super::client_streaming;

    pub struct ResponseFuture {
        pub(super) inner: client_streaming::ResponseFuture,
    }
}

pub mod client_streaming {
    use codec::Streaming;

    use futures::{Future, Poll};
    use http::Response;
    use tower_h2::Body;

    use std::marker::PhantomData;

    pub struct ResponseFuture<T, U> {
        inner: U,
        _m: PhantomData<T>,
    }

    impl<T, U> ResponseFuture<T, U> {
        /// Create a new client-streaming response future.
        pub(crate) fn new(inner: U) -> Self {
            ResponseFuture {
                inner,
                _m: PhantomData,
            }
        }
    }

    impl<T, U, B> Future for ResponseFuture<T, U>
    where U: Future<Item = Response<B>>,
          B: Body,
    {
        type Item = ::Response<Streaming<U>>;
        type Error = ::Error;

        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            unimplemented!();
        }
    }
}
*/

pub mod streaming {
    use codec::Streaming;

    use futures::{Future, Poll};
    use http::Response;
    use prost::Message;
    use tower_h2::{Body, Data};

    use std::marker::PhantomData;

    pub struct ResponseFuture<T, U> {
        inner: U,
        _m: PhantomData<T>,
    }

    impl<T, U> ResponseFuture<T, U> {
        /// Create a new client-streaming response future.
        pub(crate) fn new(inner: U) -> Self {
            ResponseFuture {
                inner,
                _m: PhantomData,
            }
        }
    }

    impl<T, U, B> Future for ResponseFuture<T, U>
    where T: Message + Default,
          U: Future<Item = Response<B>>,
          B: Body<Data = Data>,
    {
        type Item = ::Response<Streaming<T, B>>;
        type Error = ::Error<U::Error>;

        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            use codec::Decoder;
            use generic::Streaming;

            let response = self.inner.poll()
                .map_err(::Error::Inner);

            // Get the response
            let response = try_ready!(response);

            // Destructure into the head / body
            let (head, body) = response.into_parts();

            if let Some(status) = super::check_grpc_status(&head.headers) {
                return Err(::Error::Grpc(status));
            }

            let body = Streaming::new(Decoder::new(), body, true);
            let response = Response::from_parts(head, body);

            Ok(::Response::from_http(response).into())
        }
    }
}

fn check_grpc_status(trailers: &HeaderMap) -> Option<Status> {
    trailers.get("grpc-status").map(|s| {
        Status::from_bytes(s.as_ref())
    })
}
