use crate::error::{Error, Never};
use crate::generic::{Encode, Encoder};
use crate::Response;

use futures::{ready, FutureExt, TryStream};
use http::header;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

#[derive(Debug)]
pub struct ResponseFuture<T, E> {
    inner: T,
    encoder: Option<E>,
}

// ===== impl ResponseFuture =====

impl<T, E, S> ResponseFuture<T, E>
where
    T: Future<Output = Result<Response<S>, crate::Status>>,
    E: Encoder,
    S: TryStream<Ok = E::Item>,
{
    pub fn new(inner: T, encoder: E) -> Self {
        ResponseFuture {
            inner,
            encoder: Some(encoder),
        }
    }
}

impl<T, E, S> Future for ResponseFuture<T, E>
where
    T: Future<Output = Result<Response<S>, crate::Status>> + Unpin,
    E: Encoder + Unpin,
    S: TryStream<Ok = E::Item>,
    S::Error: Into<Error>,
{
    type Output = Result<http::Response<Encode<E, S>>, Never>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Get the gRPC response
        let response = match ready!(self.inner.poll_unpin(cx)) {
            Ok(response) => response,
            Err(status) => {
                // Construct http response
                let mut response = Response::new(Encode::error(status)).into_http();
                // Set the content type
                response.headers_mut().insert(
                    header::CONTENT_TYPE,
                    header::HeaderValue::from_static(E::CONTENT_TYPE),
                );

                // Early return
                return Ok(response).into();
            }
        };

        // Convert to an HTTP response
        let mut response = response.into_http();
        // Set the content type
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static(E::CONTENT_TYPE),
        );

        // Get the encoder
        let encoder = self.encoder.take().expect("encoder consumed");

        // Map the response body
        let response = response.map(move |body| Encode::response(encoder, body));

        Ok(response).into()
    }
}
