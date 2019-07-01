use crate::codec::{Encode, Encoder, Streaming};
use crate::generic::server::{server_streaming, ServerStreamingService};
use crate::Body;

use futures::ready;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct ResponseFuture<T, B, R>
where
    T: ServerStreamingService<R>,
    B: Body + Unpin,
    R: prost::Message + Default + Unpin,
{
    inner: Inner<T, T::Response, R, B>,
}

type Inner<T, U, V, B> = server_streaming::ResponseFuture<T, Encoder<U>, Streaming<V, B>>;

impl<T, B, R> ResponseFuture<T, B, R>
where
    T: ServerStreamingService<R>,
    R: prost::Message + Default + Unpin,
    T::Response: prost::Message,
    B: Body + Unpin,
{
    pub(crate) fn new(inner: Inner<T, T::Response, R, B>) -> Self {
        ResponseFuture { inner }
    }
}

impl<T, B, R> Future for ResponseFuture<T, B, R>
where
    T: ServerStreamingService<R> + Unpin,
    T::Future: Unpin,
    T::Response: Unpin,
    R: prost::Message + Default + Unpin,
    T::Response: prost::Message,
    B: Body + Unpin,
    B::Data: Unpin,
{
    type Output = Result<http::Response<Encode<T::ResponseStream>>, crate::error::Never>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let response = ready!(Pin::new(&mut self.inner).poll(cx))?;
        let response = response.map(Encode::new);
        Ok(response).into()
    }
}

impl<T, B, R> fmt::Debug for ResponseFuture<T, B, R>
where
    T: ServerStreamingService<R> + fmt::Debug,
    T::Response: fmt::Debug,
    T::ResponseStream: fmt::Debug,
    T::Future: fmt::Debug,
    B: Body + fmt::Debug + Unpin,
    B::Data: fmt::Debug,
    R: prost::Message + Default + Unpin,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("server_streaming::ResponseFuture")
            .field("inner", &self.inner)
            .finish()
    }
}
