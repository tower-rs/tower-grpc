use Body;
use codec::{Direction, Streaming};

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
    pub(crate) fn new(inner: U) -> Self {
        ResponseFuture {
            inner,
            _m: PhantomData,
        }
    }
}

impl<T, U, B> Future for ResponseFuture<T, U>
where T: Message + Default,
      U: Future<Item = Response<B>>,
      B: Body,
{
    type Item = ::Response<Streaming<T, B>>;
    type Error = ::Error<U::Error>;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        use codec::Decoder;
        use generic::Streaming;

        let response = self.inner.poll()
            .map_err(::Error::Inner);

        // Get the response
        let response = try_ready!(response);

        let status_code = response.status();

        // Destructure into the head / body
        let (head, body) = response.into_parts();

        // Check the headers for `grpc-status`, in which case we should not parse the body.
        let trailers_only_status = ::Status::from_header_map(&head.headers);
        let expect_additional_trailers = trailers_only_status.is_none();
        if let Some(status) = trailers_only_status {
            if status.code() != Code::Ok {
                return Err(::Error::Grpc(status, head.headers));
            }
        }

        let streaming_direction = if expect_additional_trailers {
            Direction::Response(status_code)
        } else {
            Direction::EmptyResponse
        };
        let body = Streaming::new(Decoder::new(), body, streaming_direction);
        let response = Response::from_parts(head, body);

        Ok(::Response::from_http(response).into())
    }
}
