#![allow(warnings, missing_debug_implementations)]

use util::BoxBody;

use futures::{stream, Future, Poll};
use http::{Request, Response};
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
    pub fn poll_ready(&mut self) -> Poll<(), ::Error> {
        unimplemented!();
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
    where B: IntoBody<U>,
    {
        // Convert the request
        let request = request.into_http();
        let (head, body) = request.into_parts();
        let body = body.into_body();
        let request = Request::from_parts(head, body);

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
