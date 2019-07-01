use super::server_streaming;
use crate::generic::server::UnaryService;
use crate::generic::{Encode, Encoder};
use crate::{Request, Response};

use futures::{ready, TryStream};
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};
use tower_service::Service;

pub struct ResponseFuture<T, E, S>
where
    T: UnaryService<S::Ok>,
    T::Response: Unpin,
    S: TryStream,
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
where
    T: UnaryService<S::Ok, Response = E::Item>,
    E: Encoder,
    E::Item: Unpin,
    S: TryStream<Error = crate::Status> + Unpin,
{
    pub fn new(inner: T, request: Request<S>, encoder: E) -> Self {
        let inner = server_streaming::ResponseFuture::new(Inner(inner), request, encoder);
        ResponseFuture { inner }
    }
}

impl<T, E, S> Future for ResponseFuture<T, E, S>
where
    T: UnaryService<S::Ok, Response = E::Item>,
    E: Encoder + Unpin,
    E::Item: Unpin,
    S: TryStream<Error = crate::Status> + Unpin,
{
    type Output = Result<http::Response<Encode<E, Once<T::Response>>>, crate::error::Error>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner).poll(cx).map_err(Into::into)
    }
}

// ===== impl Inner =====

impl<T, R> Service<Request<R>> for Inner<T>
where
    T: UnaryService<R>,
{
    type Response = Response<Once<T::Response>>;
    type Error = crate::Status;
    type Future = InnerFuture<T::Future>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Ok(()).into()
    }

    fn call(&mut self, request: Request<R>) -> Self::Future {
        let inner = self.0.call(request);
        InnerFuture(inner)
    }
}

// ===== impl InnerFuture ======

impl<T, U> Future for InnerFuture<T>
where
    T: Future<Output = Result<Response<U>, crate::Status>> + Unpin,
{
    type Output = Result<Response<Once<U>>, crate::Status>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let response = ready!(Pin::new(&mut self.0).poll(cx))?;
        Ok(Once::map(response)).into()
    }
}

// ===== impl Once =====

impl<T> Once<T> {
    /// Map a response to a response of a `Once` stream
    pub(super) fn map(response: Response<T>) -> Response<Self> {
        response.map(|body| Once { inner: Some(body) })
    }
}

impl<T: Unpin> TryStream for Once<T> {
    type Ok = T;
    type Error = crate::Status;

    fn try_poll_next(
        mut self: Pin<&mut Self>,
        _cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Self::Ok, Self::Error>>> {
        Pin::new(&mut self.inner).take().map(|v| Ok(v)).into()
    }
}

impl<T, E, S> fmt::Debug for ResponseFuture<T, E, S>
where
    T: UnaryService<S::Ok> + fmt::Debug,
    T::Response: fmt::Debug + Unpin,
    T::Future: fmt::Debug,
    E: fmt::Debug,
    S: TryStream + fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("unary::ResponseFuture")
            .field("inner", &self.inner)
            .finish()
    }
}
