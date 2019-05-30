use crate::codec::{Encode, Encoder};
use crate::generic::server::{client_streaming, unary, ClientStreamingService};

use futures::{try_ready, Future, Poll, Stream};
use std::fmt;

pub struct ResponseFuture<T, S>
where
    T: ClientStreamingService<S>,
    S: Stream<Error = crate::Status>,
{
    inner: Inner<T::Future, T::Response>,
}

type Inner<T, U> = client_streaming::ResponseFuture<T, Encoder<U>>;

impl<T, S> ResponseFuture<T, S>
where
    T: ClientStreamingService<S>,
    S: Stream<Error = crate::Status>,
    S::Item: prost::Message + Default,
    T::Response: prost::Message,
{
    pub(crate) fn new(inner: Inner<T::Future, T::Response>) -> Self {
        ResponseFuture { inner }
    }
}

impl<T, S> Future for ResponseFuture<T, S>
where
    T: ClientStreamingService<S>,
    S: Stream<Error = crate::Status>,
    S::Item: prost::Message + Default,
    T::Response: prost::Message,
{
    type Item = http::Response<Encode<unary::Once<T::Response>>>;
    type Error = crate::error::Never;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let response = try_ready!(self.inner.poll());
        let response = response.map(Encode::new);
        Ok(response.into())
    }
}

impl<T, S> fmt::Debug for ResponseFuture<T, S>
where
    T: ClientStreamingService<S> + fmt::Debug,
    S: Stream<Error = crate::Status> + fmt::Debug,
    T::Response: fmt::Debug,
    T::Future: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("client_streaming::ResponseFuture")
            .field("inner", &self.inner)
            .finish()
    }
}
