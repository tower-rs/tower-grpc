use super::streaming;
use crate::generic::server::ServerStreamingService;
use crate::generic::{Encode, Encoder};
use crate::{Request, Response};

use futures::{ready, Stream, TryStream};
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

/// A server streaming response future
pub struct ResponseFuture<T, E, S>
where
    T: ServerStreamingService<S::Item>,
    S: Stream,
{
    inner: streaming::ResponseFuture<Inner<T, S>, E>,
}

struct Inner<T, S>
where
    T: ServerStreamingService<S::Item>,
    S: Stream,
{
    inner: T,
    state: Option<State<T::Future, S>>,
}

#[derive(Debug)]
enum State<T, S> {
    /// Waiting for the request to be received
    Requesting(Request<S>),

    /// Waiting for the response future to resolve
    Responding(T),
}

// ===== impl ResponseFuture ======

impl<T, E, S> ResponseFuture<T, E, S>
where
    T: ServerStreamingService<S::Ok, Response = E::Item>,
    E: Encoder,
    S: TryStream<Error = crate::Status>,
{
    pub fn new(inner: T, request: Request<S>, encoder: E) -> Self {
        let inner = Inner {
            inner,
            state: Some(State::Requesting(request)),
        };

        let inner = streaming::ResponseFuture::new(inner, encoder);
        ResponseFuture { inner }
    }
}

impl<T, E, S> Future for ResponseFuture<T, E, S>
where
    T: ServerStreamingService<S::Ok, Response = E::Item>,
    E: Encoder,
    S: TryStream<Error = crate::Status>,
{
    type Output = Result<http::Response<Encode<E, T::ResponseStream>>, crate::error::Never>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut self.inner).poll(cx)
    }
}

// ===== impl Inner =====

impl<T, S> Future for Inner<T, S>
where
    T: ServerStreamingService<S::Ok>,
    S: TryStream<Error = crate::Status>,
{
    type Output = Result<Response<T::ResponseStream>, crate::Status>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        use self::State::*;

        loop {
            let msg = match *self.state.as_mut().unwrap() {
                Requesting(ref mut request) => ready!(Pin::new(&mut request).poll(cx)),
                Responding(ref mut fut) => {
                    return Pin::new(&mut fut).poll(cx);
                }
            };

            match msg {
                Some(msg) => match self.state.take().unwrap() {
                    Requesting(request) => {
                        let request = request.map(|_| msg);
                        let response = self.inner.call(request);

                        self.state = Some(Responding(response));
                    }
                    _ => unreachable!(),
                },
                None => {
                    return Err(crate::Status::new(
                        crate::Code::Internal,
                        "Missing request message.",
                    ))
                }
            }
        }
    }
}

impl<T, E, S> fmt::Debug for ResponseFuture<T, E, S>
where
    T: ServerStreamingService<S::Item> + fmt::Debug,
    T::Response: fmt::Debug,
    T::ResponseStream: fmt::Debug,
    T::Future: fmt::Debug,
    E: fmt::Debug,
    S: Stream + fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("server_streaming::ResponseFuture")
            .field("inner", &self.inner)
            .finish()
    }
}

impl<T, S> fmt::Debug for Inner<T, S>
where
    T: ServerStreamingService<S::Item> + fmt::Debug,
    T::Response: fmt::Debug,
    T::ResponseStream: fmt::Debug,
    T::Future: fmt::Debug,
    S: Stream + fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt.debug_struct("Inner")
            .field("inner", &self.inner)
            .field("state", &self.state)
            .finish()
    }
}
