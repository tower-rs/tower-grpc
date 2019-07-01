use super::client_streaming;
use crate::error::Error;
use crate::Body;

use futures::{future, stream, TryFuture};
use http::Response;
use prost::Message;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct ResponseFuture<T, U, B: Body> {
    inner: client_streaming::ResponseFuture<T, U, B>,
}

pub type Once<T> = stream::Once<future::Ready<Result<T, crate::Status>>>;

impl<T, U, B: Body> ResponseFuture<T, U, B> {
    /// Create a new client-streaming response future.
    pub(crate) fn new(inner: client_streaming::ResponseFuture<T, U, B>) -> Self {
        ResponseFuture { inner }
    }
}

impl<T, U, B> Future for ResponseFuture<T, U, B>
where
    T: Message + Default + Unpin,
    U: TryFuture<Ok = Response<B>> + Unpin,
    U::Error: Into<Error>,
    B: Body + Unpin,
    B::Data: Unpin,
    B::Error: Into<Error>,
{
    type Output = Result<crate::Response<T>, crate::Status>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner).try_poll(cx)
    }
}

impl<T, U, B> fmt::Debug for ResponseFuture<T, U, B>
where
    T: fmt::Debug,
    U: fmt::Debug,
    B: Body + fmt::Debug,
    B::Data: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResponseFuture")
            .field("inner", &self.inner)
            .finish()
    }
}
