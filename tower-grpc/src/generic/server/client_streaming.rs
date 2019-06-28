use super::streaming;
use super::unary::Once;
use crate::generic::{Encode, Encoder};
use crate::Response;

use futures::ready;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Debug)]
pub struct ResponseFuture<T, E> {
    inner: streaming::ResponseFuture<Inner<T>, E>,
}

#[derive(Debug)]
struct Inner<T> {
    inner: T,
}

// ===== impl ResponseFuture ======

impl<T, E> ResponseFuture<T, E>
where
    T: Future<Output = Result<Response<E::Item>, crate::Status>>,
    E: Encoder,
{
    pub fn new(inner: T, encoder: E) -> Self {
        let inner = Inner { inner };
        let inner = streaming::ResponseFuture::new(inner, encoder);
        ResponseFuture { inner }
    }
}

impl<T, E> Future for ResponseFuture<T, E>
where
    T: Future<Output = Result<Response<E::Item>, crate::Status>>,
    E: Encoder,
{
    type Output = Result<http::Response<Encode<E, Once<E::Item>>>, crate::error::Never>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner).poll(cx)
    }
}

// ===== impl Inner ======

impl<T, U> Future for Inner<T>
where
    T: Future<Output = Result<Response<U>, crate::Status>>,
{
    type Output = Result<Response<Once<U>>, crate::Status>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let response = ready!(Pin::new(&mut self.inner).poll(cx));
        Ok(Once::map(response)).into()
    }
}
