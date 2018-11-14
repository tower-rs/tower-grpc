use std::fmt;

use bytes::{Bytes, IntoBuf};
use futures::Poll;
use http;

/// A body to send and receive gRPC messages.
pub trait Body {
    /// The body buffer type.
    type Data: IntoBuf;

    /// Returns `true` when the end of the stream has been reached.
    ///
    /// An end of stream means that both `poll_data` and `poll_metadata` will
    /// return `None`.
    ///
    /// A return value of `false` **does not** guarantee tht a value will be
    /// returned from `poll_data` or `poll_trailers. This is merely a hint.
    fn is_end_stream(&self) -> bool {
        false
    }

    /// Polls the stream for more data.
    fn poll_data(&mut self) -> Poll<Option<Self::Data>, ::Error>;

    /// Polls the stream for the ending metadata.
    fn poll_metadata(&mut self) -> Poll<Option<http::HeaderMap>, ::Error>;
}

/// Dynamic `Send` body object.
pub struct BoxBody<T = Bytes> {
    inner: Box<Body<Data = T> + Send>,
}

// ===== impl BoxBody =====

impl<T> BoxBody<T> {
    /// Create a new `BoxBody` backed by `inner`.
    pub fn new(inner: Box<Body<Data = T> + Send>) -> Self {
        BoxBody {
            inner,
        }
    }
}

impl<T> Body for BoxBody<T>
where
    T: IntoBuf,
{
    type Data = T;

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, ::Error> {
        self.inner.poll_data()
    }

    fn poll_metadata(&mut self) -> Poll<Option<http::HeaderMap>, ::Error> {
        self.inner.poll_metadata()
    }
}

#[cfg(feature = "tower-h2")]
impl<T> ::tower_h2::Body for BoxBody<T>
where
    T: IntoBuf + 'static,
{
    type Data = T;

    fn is_end_stream(&self) -> bool {
        Body::is_end_stream(self)
    }

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, ::h2::Error> {
        Body::poll_data(self)
            .map_err(::h2::Error::from)
    }

    fn poll_trailers(&mut self) -> Poll<Option<http::HeaderMap>, ::h2::Error> {
        Body::poll_metadata(self)
            .map_err(::h2::Error::from)
    }
}

impl<T> fmt::Debug for BoxBody<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BoxBody")
            .finish()
    }
}

// ===== impl tower_h2::RecvBody =====

#[cfg(feature = "tower-h2")]
impl Body for ::tower_h2::RecvBody {
    type Data = Bytes;

    fn is_end_stream(&self) -> bool {
        ::tower_h2::Body::is_end_stream(self)
    }

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, ::Error> {
        let data = try_ready!(::tower_h2::Body::poll_data(self));
        Ok(data.map(Bytes::from).into())
    }

    fn poll_metadata(&mut self) -> Poll<Option<http::HeaderMap>, ::Error> {
        ::tower_h2::Body::poll_trailers(self)
            .map_err(::Error::from)
    }
}
