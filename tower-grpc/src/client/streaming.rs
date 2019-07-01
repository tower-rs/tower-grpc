use crate::codec::{Direction, Streaming};
use crate::error::Error;
use crate::Body;
use crate::Code;

use futures::{ready, TryFuture};
use http::Response;
use prost::Message;
use std::future::Future;
use std::marker::PhantomData;
use std::pin::Pin;
use std::task::{Context, Poll};

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
where
    T: Message + Default + Unpin,
    U: TryFuture<Ok = Response<B>> + Unpin,
    U::Error: Into<Error>,
    B: Body + Unpin,
{
    type Output = Result<crate::Response<Streaming<T, B>>, crate::Status>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        use crate::codec::Decoder;
        use crate::generic::Streaming;

        // Get the response
        let response = ready!(Pin::new(&mut self.inner)
            .try_poll(cx)
            .map_err(|err| crate::Status::from_error(&*(err.into()))))?;

        let status_code = response.status();

        // Check the headers for `grpc-status`, in which case we should not parse the body.
        let trailers_only_status = crate::Status::from_header_map(response.headers());
        let expect_additional_trailers = trailers_only_status.is_none();
        if let Some(status) = trailers_only_status {
            if status.code() != Code::Ok {
                return Err(status).into();
            }
        }

        let streaming_direction = if expect_additional_trailers {
            Direction::Response(status_code)
        } else {
            Direction::EmptyResponse
        };

        let response =
            response.map(move |body| Streaming::new(Decoder::new(), body, streaming_direction));

        Ok(crate::Response::from_http(response)).into()
    }
}
