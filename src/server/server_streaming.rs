use Body;
use codec::{Encode, Encoder, Streaming};
use generic::server::{ServerStreamingService, server_streaming};

use std::fmt;

use {http, prost};
use futures::{Future, Poll};

pub struct ResponseFuture<T, B, R>
where
    T: ServerStreamingService<R>,
    B: Body,
    R: prost::Message + Default,
{
    inner: Inner<T, T::Response, R, B>,
}

type Inner<T, U, V, B> =
    server_streaming::ResponseFuture<T, Encoder<U>, Streaming<V, B>>;

impl<T, B, R> ResponseFuture<T, B, R>
where T: ServerStreamingService<R>,
      R: prost::Message + Default,
      T::Response: prost::Message,
      B: Body,
{
    pub(crate) fn new(inner: Inner<T, T::Response, R, B>) -> Self {
        ResponseFuture { inner }
    }
}

impl<T, B, R> Future for ResponseFuture<T, B, R>
where T: ServerStreamingService<R>,
      R: prost::Message + Default,
      T::Response: prost::Message,
      B: Body,
{
    type Item = http::Response<Encode<T::ResponseStream>>;
    type Error = ::error::Never;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let response = try_ready!(self.inner.poll());
        let response = response.map(Encode::new);
        Ok(response.into())
    }
}

impl<T, B, R> fmt::Debug for ResponseFuture<T, B, R>
where T: ServerStreamingService<R> + fmt::Debug,
      T::Response: fmt::Debug,
      T::ResponseStream: fmt::Debug,
      T::Future: fmt::Debug,
      B: Body + fmt::Debug,
      B::Item: fmt::Debug,
      R: prost::Message + Default,
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("server_streaming::ResponseFuture")
            .field("inner", &self.inner)
            .finish()
    }
}
