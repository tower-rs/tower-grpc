use super::client_streaming;

use futures::{stream, Future, Poll};
use http::{Response};
use prost::Message;
use tower_h2::{Body, Data};

#[derive(Debug)]
pub struct ResponseFuture<T, U, B> {
    inner: client_streaming::ResponseFuture<T, U, B>,
}

pub type Once<T> = stream::Once<T, ::Error>;

impl<T, U, B> ResponseFuture<T, U, B> {
    /// Create a new client-streaming response future.
    pub(crate) fn new(inner: client_streaming::ResponseFuture<T, U, B>) -> Self {
        ResponseFuture { inner }
    }
}

impl<T, U, B> Future for ResponseFuture<T, U, B>
where T: Message + Default,
      U: Future<Item = Response<B>>,
      B: Body<Data = Data>,
{
    type Item = ::Response<T>;
    type Error = ::Error<U::Error>;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        self.inner.poll()
    }
}
