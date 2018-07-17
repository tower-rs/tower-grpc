use codec::Streaming;

use futures::{Future, Poll};
use http::Response;
use prost::Message;
use tower_h2::{Body, Data};

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
      B: Body<Data = Data>,
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

        // Destructure into the head / body
        let (head, body) = response.into_parts();

        let trailers_only_status = super::check_grpc_status(&head.headers);
        let expect_additional_trailers = trailers_only_status.is_none();
        if let Some(status) = trailers_only_status {
            if !status.ok() {
                return Err(::Error::Grpc(status, head.headers));
            }
        }

        let body = Streaming::new(Decoder::new(), body, expect_additional_trailers);
        let response = Response::from_parts(head, body);

        Ok(::Response::from_http(response).into())
    }
}
