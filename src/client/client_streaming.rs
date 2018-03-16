use super::streaming;
use codec::Streaming;

use futures::{Future, Stream, Poll};
use http::{response, Response};
use prost::Message;
use tower_h2::{Body, Data};
use error::ProtocolError;

#[derive(Debug)]
pub struct ResponseFuture<T, U, B> {
    state: State<T, U, B>,
}

#[derive(Debug)]
enum State<T, U, B> {
    WaitResponse(streaming::ResponseFuture<T, U>),
    WaitMessage {
        head: Option<response::Parts>,
        stream: Streaming<T, B>,
    },
}

impl<T, U, B> ResponseFuture<T, U, B> {
    /// Create a new client-streaming response future.
    pub(crate) fn new(inner: streaming::ResponseFuture<T, U>) -> Self {
        let state = State::WaitResponse(inner);
        ResponseFuture { state }
    }
}

impl<T, U, B> Future for ResponseFuture<T, U, B>
where T: Message + Default,
      U: Future<Item = Response<B>>,
      B: Body<Data = Data>,
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
                            ::Error::Decode(e) => ::Error::Decode(e),
                            ::Error::Grpc(s, h) => ::Error::Grpc(s, h),
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
