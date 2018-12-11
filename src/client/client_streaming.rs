use Body;
use super::streaming;
use codec::{Streaming};

use std::fmt;

use bytes::IntoBuf;
use futures::{Future, Stream, Poll};
use http::{response, Response};
use prost::Message;
use error::ProtocolError;

pub struct ResponseFuture<T, U, B: Body> {
    state: State<T, U, B>,
}

enum State<T, U, B: Body> {
    WaitResponse(streaming::ResponseFuture<T, U>),
    WaitMessage {
        head: Option<response::Parts>,
        stream: Streaming<T, B>,
    },
}

impl<T, U, B: Body> ResponseFuture<T, U, B> {
    /// Create a new client-streaming response future.
    pub(crate) fn new(inner: streaming::ResponseFuture<T, U>) -> Self {
        let state = State::WaitResponse(inner);
        ResponseFuture { state }
    }
}

impl<T, U, B> Future for ResponseFuture<T, U, B>
where T: Message + Default,
      U: Future<Item = Response<B>>,
      B: Body,
{
    type Item = ::Response<T>;
    type Error = ::Error<U::Error>;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        use self::State::*;

        loop {
            let response = match self.state {
                WaitResponse(ref mut inner) => {
                    try_ready!(inner.poll())
                }
                WaitMessage { ref mut head, ref mut stream } => {
                    let res = stream.poll()
                        .map_err(|e| match e {
                            ::Error::Protocol(p) => ::Error::Protocol(p),
                            ::Error::Inner(()) => ::Error::Protocol(ProtocolError::Internal),
                            ::Error::Grpc(s) => ::Error::Grpc(s),
                        });

                    let message = match try_ready!(res) {
                        Some(message) => message,
                        None => return Err(::Error::Protocol(ProtocolError::MissingMessage)),
                    };

                    let head = head.take().unwrap();
                    let response = Response::from_parts(head, message);

                    return Ok(::Response::from_http(response).into());
                }
            };

            let (head, body) = response
                .into_http()
                .into_parts();

            self.state = WaitMessage {
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
    <B::Data as IntoBuf>::Buf: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
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
    <B::Data as IntoBuf>::Buf: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            State::WaitResponse(ref future) => f.debug_tuple("WaitResponse")
                .field(future)
                .finish(),
            State::WaitMessage { ref head, ref stream } => f.debug_struct("WaitMessage")
                .field("head", head)
                .field("stream", stream)
                .finish(),
        }
    }
}
