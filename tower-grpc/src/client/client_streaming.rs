use super::streaming;
use crate::codec::Streaming;
use crate::error::Error;
use crate::Body;

use futures::{ready, Stream, TryFuture};
use http::{response, Response};
use prost::Message;
use std::fmt;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct ResponseFuture<T, U, B: Body> {
    state: State<T, U, B>,
}

enum State<T, U, B: Body> {
    /// Waiting for the HTTP response
    WaitResponse(streaming::ResponseFuture<T, U>),
    /// Waiting for the gRPC Proto message in the Response body
    WaitMessage {
        head: Option<response::Parts>,
        stream: Streaming<T, B>,
    },
}

impl<T, U, B: Body> ResponseFuture<T, U, B> {
    /// Create a new client-streaming response future.
    pub(super) fn new(inner: streaming::ResponseFuture<T, U>) -> Self {
        let state = State::WaitResponse(inner);
        ResponseFuture { state }
    }
}

impl<T, U, B> Future for ResponseFuture<T, U, B>
where
    T: Message + Default + Unpin,
    U: TryFuture<Ok = Response<B>> + Unpin,
    U::Error: Into<Error>,
    B: Body + Unpin,
    B::Data: Unpin,
    B::Error: Into<Error>,
{
    type Output = Result<crate::Response<T>, crate::Status>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let me = &mut self;

            let response = match &mut me.state {
                State::WaitResponse(inner) => ready!(Pin::new(inner).try_poll(cx))?,
                State::WaitMessage { head, stream } => {
                    let message = match ready!(Pin::new(stream).poll_next(cx)) {
                        Some(Ok(message)) => message,
                        Some(Err(e)) => return Err(e).into(),
                        None => {
                            return Err(crate::Status::new(
                                crate::Code::Internal,
                                "Missing response message.",
                            ))
                            .into();
                        }
                    };

                    let head = head.take().unwrap();
                    let response = Response::from_parts(head, message);

                    return Poll::Ready(Ok(crate::Response::from_http(response)));
                }
            };

            let (head, body) = response.into_http().into_parts();

            me.state = State::WaitMessage {
                head: Some(head),
                stream: body,
            };
        }
    }
}

impl<T, U, B> fmt::Debug for ResponseFuture<T, U, B>
where
    T: fmt::Debug,
    U: fmt::Debug,
    B: Body + fmt::Debug,
    B::Data: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ResponseFuture")
            .field("state", &self.state)
            .finish()
    }
}

impl<T, U, B> fmt::Debug for State<T, U, B>
where
    T: fmt::Debug,
    U: fmt::Debug,
    B: Body + fmt::Debug,
    B::Data: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            State::WaitResponse(ref future) => f.debug_tuple("WaitResponse").field(future).finish(),
            State::WaitMessage {
                ref head,
                ref stream,
            } => f
                .debug_struct("WaitMessage")
                .field("head", head)
                .field("stream", stream)
                .finish(),
        }
    }
}
