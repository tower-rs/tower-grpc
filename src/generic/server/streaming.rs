use {Response};
use generic::{Encoder, Encode};

use {http, h2};
use futures::{Future, Stream, Poll, Async};
use http::header;

#[derive(Debug)]
pub struct ResponseFuture<T, E> {
    inner: T,
    encoder: Option<E>,
}

// ===== impl ResponseFuture =====

impl<T, E, S> ResponseFuture<T, E>
where T: Future<Item = Response<S>,
               Error = ::Error>,
      E: Encoder,
      S: Stream<Item = E::Item>,
{
    pub fn new(inner: T, encoder: E) -> Self {
        ResponseFuture {
            inner,
            encoder: Some(encoder),
        }
    }
}

impl<T, E, S> Future for ResponseFuture<T, E>
where T: Future<Item = Response<S>,
               Error = ::Error>,
      E: Encoder,
      S: Stream<Item = E::Item>,
{
    type Item = http::Response<Encode<E, S>>;
    type Error = h2::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        // Get the gRPC response
        let response = match self.inner.poll() {
            Ok(Async::Ready(response)) => response,
            Ok(Async::NotReady) => return Ok(Async::NotReady),
            Err(e) => {
                match e {
                    ::Error::Grpc(status, _) => {
                        let response = Response::new(Encode::error(status));
                        return Ok(response.into_http().into());
                    }
                    // TODO: Is this correct?
                    _ => return Err(h2::Reason::INTERNAL_ERROR.into()),
                }
            }
        };

        // Convert to an HTTP response
        let response = response.into_http();

        // Map the response body
        let (mut head, body) = response.into_parts();

        // Set the content type
        head.headers.insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static(E::CONTENT_TYPE),
        );

        // Get the encoder
        let encoder = self.encoder.take().expect("encoder consumed");

        // Encode the body
        let body = Encode::new(encoder, body, true);

        // Success
        Ok(http::Response::from_parts(head, body).into())
    }
}
