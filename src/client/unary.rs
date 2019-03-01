use Body;
use super::client_streaming;

use std::fmt;

use bytes::IntoBuf;
use futures::{stream, Future, Poll};
use http::{Response};
use prost::Message;

pub struct ResponseFuture<T, U, B: Body> {
    inner: client_streaming::ResponseFuture<T, U, B>,
}

pub type Once<T> = stream::Once<T, ::Status>;

impl<T, U, B: Body> ResponseFuture<T, U, B> {
    /// Create a new client-streaming response future.
    pub(crate) fn new(inner: client_streaming::ResponseFuture<T, U, B>) -> Self {
        ResponseFuture { inner }
    }
}

impl<T, U, B> Future for ResponseFuture<T, U, B>
where T: Message + Default,
      U: Future<Item = Response<B>>,
      U::Error: Into<Box<dyn std::error::Error>>,
      B: Body,
{
    type Item = ::Response<T>;
    type Error = ::Status;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.inner.poll()
    }
}

impl<T, U, B> fmt::Debug for ResponseFuture<T, U, B>
where
    T: fmt::Debug,
    U: fmt::Debug,
    B: Body + fmt::Debug,
    <B::Data as IntoBuf>::Buf: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("ResponseFuture")
            .field("inner", &self.inner)
            .finish()
    }
}
