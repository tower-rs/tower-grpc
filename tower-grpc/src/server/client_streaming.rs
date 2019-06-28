use crate::codec::{Encode, Encoder};
use crate::generic::server::{client_streaming, unary, ClientStreamingService};

use futures::{ready, Stream};
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

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
    type Output = Result<http::Response<Encode<unary::Once<T::Response>>>, crate::error::Never>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let response = ready!(Pin::new(&mut self.inner).poll_next(cx));
        let response = response.map(Encode::new);
        Ok(response).into()
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
