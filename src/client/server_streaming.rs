use super::streaming;
use codec::Streaming;

use futures::{Future, Stream, Poll};
use http::{response, Response};
use prost::Message;
use tower_h2::{Body, Data};

use std::marker::PhantomData;

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
where T: Message + Default,
      U: Future<Item = Response<B>>,
      B: Body<Data = Data>,
{
    type Item = ::Response<Streaming<T, B>>;
    type Error = ::Error<U::Error>;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.inner.poll()
    }
}
