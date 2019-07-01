use super::streaming;
use crate::codec::Streaming;
use crate::error::Error;
use crate::Body;

use futures::future::TryFuture;
use http::Response;
use prost::Message;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Debug)]
pub struct ResponseFuture<T, U> {
    inner: streaming::ResponseFuture<T, U>,
}

impl<T, U> ResponseFuture<T, U> {
    /// Create a new client-streaming response future.
    pub(crate) fn new(inner: streaming::ResponseFuture<T, U>) -> Self {
        ResponseFuture { inner }
    }
}

impl<T, U, B> Future for ResponseFuture<T, U>
where
    T: Message + Default + Unpin,
    U: TryFuture<Ok = Response<B>> + Unpin,
    U::Error: Into<Error>,
    B: Body + Unpin,
{
    type Output = Result<crate::Response<Streaming<T, B>>, crate::Status>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner).try_poll(cx)
    }
}
