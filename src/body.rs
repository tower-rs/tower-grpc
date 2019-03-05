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
    fn poll_data(&mut self) -> Poll<Option<Self::Data>, ::Status>;

    /// Polls the stream for the ending metadata.
    fn poll_metadata(&mut self) -> Poll<Option<http::HeaderMap>, ::Status>;
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

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, ::Status> {
        self.inner.poll_data()
    }

    fn poll_metadata(&mut self) -> Poll<Option<http::HeaderMap>, ::Status> {
        self.inner.poll_metadata()
    }
}

impl<T> ::tower_http_service::Body for BoxBody<T>
where
    T: IntoBuf,
{
    type Item = T::Buf;
    type Error = ::Status;

    fn poll_buf(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let item = try_ready!(self.inner.poll_data());
        Ok(item.map(IntoBuf::into_buf).into())
    }

    fn poll_trailers(&mut self) -> Poll<Option<http::HeaderMap>, Self::Error> {
        self.inner.poll_metadata()
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
        false
    }

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, ::Status> {
        let data = try_ready!(::tower_http_service::Body::poll_buf(self));
        Ok(data.map(Bytes::from).into())
    }

    fn poll_metadata(&mut self) -> Poll<Option<http::HeaderMap>, ::Status> {
        ::tower_h2::Body::poll_trailers(self)
            .map_err(::Status::from)
    }
}
