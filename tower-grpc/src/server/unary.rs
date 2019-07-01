pub use crate::generic::server::unary::Once;

use crate::codec::{Decoder, Encode, Encoder};
use crate::generic::server::{unary, UnaryService};
use crate::generic::Streaming;
use crate::Body;

use futures::ready;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct ResponseFuture<T, B, R>
where
    T: UnaryService<R>,
    R: prost::Message + Default + Unpin,
    T::Response: prost::Message,
    B: Body + Unpin,
{
    inner: Inner<T, T::Response, R, B>,
}

type Inner<T, U, V, B> = unary::ResponseFuture<T, Encoder<U>, Streaming<Decoder<V>, B>>;

impl<T, B, R> ResponseFuture<T, B, R>
where
    T: UnaryService<R>,
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
    T: UnaryService<R>,
    R: prost::Message + Default + Unpin,
    T::Response: prost::Message,
    B: Body + Unpin,
    B::Data: Unpin,
{
    type Output = Result<http::Response<Encode<Once<T::Response>>>, crate::error::Never>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let response = ready!(Pin::new(&mut self.inner).poll(cx)).unwrap();
        let response = response.map(Encode::new);
        Ok(response).into()
    }
}

impl<T, B, R> fmt::Debug for ResponseFuture<T, B, R>
where
    T: UnaryService<R> + fmt::Debug,
    R: prost::Message + Default + fmt::Debug + Unpin,
    T::Response: prost::Message + fmt::Debug,
    T::Future: fmt::Debug,
    B: Body + fmt::Debug + Unpin,
    B::Data: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("unary::ResponseFuture")
            .field("inner", &self.inner)
            .finish()
    }
}
