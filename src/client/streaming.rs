use Body;
use codec::{Direction, Streaming};
use error::Error;

use futures::{Future, Poll};
use http::Response;
use prost::Message;

use Code;

use std::marker::PhantomData;

#[derive(Debug)]
pub struct ResponseFuture<T, U> {
    inner: U,
    _m: PhantomData<T>,
}

impl<T, U> ResponseFuture<T, U> {
    /// Create a new client-streaming response future.
    pub(super) fn new(inner: U) -> Self {
        ResponseFuture {
            inner,
            _m: PhantomData,
        }
    }
}

impl<T, U, B> Future for ResponseFuture<T, U>
where T: Message + Default,
      U: Future<Item = Response<B>>,
      U::Error: Into<Error>,
      B: Body,
{
    type Item = ::Response<Streaming<T, B>>;
    type Error = ::Status;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        use codec::Decoder;
        use generic::Streaming;

        // Get the response
        let response = try_ready!(self.inner.poll().map_err(|err| {
            ::Status::from_error(&*(err.into()))
        }));

        let status_code = response.status();


        // Check the headers for `grpc-status`, in which case we should not parse the body.
        let trailers_only_status = ::Status::from_header_map(response.headers());
        let expect_additional_trailers = trailers_only_status.is_none();
        if let Some(status) = trailers_only_status {
            if status.code() != Code::Ok {
                return Err(status);
            }
        }

        let streaming_direction = if expect_additional_trailers {
            Direction::Response(status_code)
        } else {
            Direction::EmptyResponse
        };

        let response = response.map(move |body| {
            Streaming::new(Decoder::new(), body, streaming_direction)
        });

        Ok(::Response::from_http(response).into())
    }
}
