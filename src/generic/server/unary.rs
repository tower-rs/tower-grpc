use {Request, Response};
use super::server_streaming;
use generic::{Encoder, Encode};
use generic::server::UnaryService;

use {h2, http};
use futures::{Future, Stream, Poll};
use tower_service::Service;

use std::fmt;

pub struct ResponseFuture<T, E, S>
where
    T: UnaryService<S::Item>,
    S: Stream,
{
    inner: server_streaming::ResponseFuture<Inner<T>, E, S>,
}

// TODO: Use type in futures-rs instead
#[derive(Debug)]
pub struct Once<T> {
    inner: Option<T>,
}

/// Maps inbound requests
#[derive(Debug, Clone)]
struct Inner<T>(pub T);

#[derive(Debug)]
struct InnerFuture<T>(T);

// ===== impl ResponseFuture ======

impl<T, E, S> ResponseFuture<T, E, S>
where T: UnaryService<S::Item, Response = E::Item>,
      E: Encoder,
      S: Stream<Error = ::Status>,
{
    pub fn new(inner: T, request: Request<S>, encoder: E) -> Self {
        let inner = server_streaming::ResponseFuture::new(Inner(inner), request, encoder);
        ResponseFuture { inner }
    }
}

impl<T, E, S> Future for ResponseFuture<T, E, S>
where T: UnaryService<S::Item, Response = E::Item>,
      E: Encoder,
      S: Stream<Error = ::Status>,
{
    type Item = http::Response<Encode<E, Once<T::Response>>>;
    type Error = h2::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.inner.poll()
    }
}

// ===== impl Inner =====

impl<T, R> Service<Request<R>> for Inner<T>
where T: UnaryService<R>,
{
    type Response = Response<Once<T::Response>>;
    type Error = ::Status;
    type Future = InnerFuture<T::Future>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Ok(().into())
    }

    fn call(&mut self, request: Request<R>) -> Self::Future {
        let inner = self.0.call(request);
        InnerFuture(inner)
    }
}

// ===== impl InnerFuture ======

impl<T, U> Future for InnerFuture<T>
where T: Future<Item = Response<U>, Error = ::Status> {
    type Item = Response<Once<U>>;
    type Error = ::Status;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let response = try_ready!(self.0.poll());
        Ok(Once::map(response).into())
    }
}

// ===== impl Once =====

impl<T> Once<T> {
    /// Map a response to a response of a `Once` stream
    pub(super) fn map(response: Response<T>) -> Response<Self> {
        // A bunch of junk to map the body type
        let http = response.into_http();
        let (head, body) = http.into_parts();

        // Wrap with `Once`
        let body = Once { inner: Some(body) };

        let http = ::http::Response::from_parts(head, body);
        Response::from_http(http)
    }
}

impl<T> Stream for Once<T> {
    type Item = T;
    type Error = ::Status;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        Ok(self.inner.take().into())
    }
}

impl<T, E, S> fmt::Debug for ResponseFuture<T, E, S>
where T: UnaryService<S::Item> + fmt::Debug,
      T::Response: fmt::Debug,
      T::Future: fmt::Debug,
      E: fmt::Debug,
      S: Stream + fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("unary::ResponseFuture")
            .field("inner", &self.inner)
            .finish()
    }
}
