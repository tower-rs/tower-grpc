use codec::{Encode, Encoder};
use generic::server::{ClientStreamingService, client_streaming, unary};

use {h2, http, prost};
use futures::{Future, Poll, Stream};

use std::fmt;

pub struct ResponseFuture<T, S>
where
    T: ClientStreamingService<S>,
    S: Stream<Error = ::Error>,
{
    inner: Inner<T::Future, T::Response>,
}

type Inner<T, U> =
    client_streaming::ResponseFuture<T, Encoder<U>>;

impl<T, S> ResponseFuture<T, S>
where
    T: ClientStreamingService<S>,
    S: Stream<Error = ::Error>,
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
    S: Stream<Error = ::Error>,
    S::Item: prost::Message + Default,
    T::Response: prost::Message,
{
    type Item = http::Response<Encode<unary::Once<T::Response>>>;
    type Error = h2::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let response = try_ready!(self.inner.poll());
        let (head, body) = response.into_parts();
        let body = Encode::new(body);
        Ok(http::Response::from_parts(head, body).into())
    }
}

impl<T, S> fmt::Debug for ResponseFuture<T, S>
where
    T: ClientStreamingService<S> + fmt::Debug,
    S: Stream<Error = ::Error> + fmt::Debug,
    T::Response: fmt::Debug,
    T::Future: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("client_streaming::ResponseFuture")
            .field("inner", &self.inner)
            .finish()
    }
}
