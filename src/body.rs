use std::fmt;

use bytes::{Bytes, Buf, IntoBuf};
use futures::Poll;
use http;
pub use tower_http_service::Body as HttpBody;

use self::sealed::Sealed;

type BytesBuf = <Bytes as IntoBuf>::Buf;
type Error = Box<dyn std::error::Error + Send + Sync>;

/// A "trait alias" for `tower_http_service::Body` with bounds required by
/// tower-grpc.
///
/// Not to be implemented directly, but instead useful for reducing bounds
/// boilerplate.
pub trait Body: Sealed {
    type Item: Buf;
    type Error: Into<Error>;

    fn is_end_stream(&self) -> bool;

    fn poll_buf(&mut self) -> Poll<Option<Self::Item>, Self::Error>;

    fn poll_trailers(&mut self) -> Poll<Option<http::HeaderMap>, Self::Error>;
}

impl<T> Body for T
where
    T: HttpBody,
    T::Error: Into<Error>,
{
    type Item = T::Item;
    type Error = T::Error;

    fn is_end_stream(&self) -> bool {
        HttpBody::is_end_stream(self)
    }

    fn poll_buf(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        HttpBody::poll_buf(self)
    }

    fn poll_trailers(&mut self) -> Poll<Option<http::HeaderMap>, Self::Error> {
        HttpBody::poll_trailers(self)
    }
}

impl<T> Sealed for T
where
    T: HttpBody,
    T::Error: Into<Error>,
{}

/// Dynamic `Send` body object.
pub struct BoxBody<T = BytesBuf, E = ::Status> {
    inner: Box<Body<Item = T, Error = E> + Send>,
}

struct MapBody<B>(B);

// ===== impl BoxBody =====

impl<T, E> BoxBody<T, E> {
    /// Create a new `BoxBody` backed by `inner`.
    pub fn new(inner: Box<Body<Item = T, Error = E> + Send>) -> Self {
        BoxBody {
            inner,
        }
    }
}

impl BoxBody {
    /// Create a new `BoxBody` mapping item and error to the default types.
    pub fn map_from<B>(inner: B) -> Self
    where
        B: Body + Send + 'static,
        Bytes: From<B::Item>,
        ::Status: From<B::Error>,
    {
        BoxBody::new(Box::new(MapBody(inner)))
    }
}

impl<T, E> HttpBody for BoxBody<T, E>
where
    T: Buf,
    E: Into<Error>,
{
    type Item = T;
    type Error = E;

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn poll_buf(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        self.inner.poll_buf()
    }

    fn poll_trailers(&mut self) -> Poll<Option<http::HeaderMap>, Self::Error> {
        self.inner.poll_trailers()
    }
}

impl<T> fmt::Debug for BoxBody<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("BoxBody")
            .finish()
    }
}

// ===== impl MapBody =====

impl<B> HttpBody for MapBody<B>
where
    B: Body,
    Bytes: From<B::Item>,
    ::Status: From<B::Error>,
{
    type Item = BytesBuf;
    type Error = ::Status;

    fn is_end_stream(&self) -> bool {
        self.0.is_end_stream()
    }

    fn poll_buf(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        let item = try_ready!(self.0.poll_buf());
        Ok(item.map(|buf| Bytes::from(buf).into_buf()).into())
    }

    fn poll_trailers(&mut self) -> Poll<Option<http::HeaderMap>, Self::Error> {
        self.0.poll_trailers().map_err(From::from)
    }
}

mod sealed {
    pub trait Sealed {}
}
