use self::sealed::Sealed;
use crate::error::Error;
use crate::Status;

use bytes::{Buf, Bytes, IntoBuf};
use futures::{try_ready, Poll};
pub use http_body::Body as HttpBody;
use std::fmt;

type BytesBuf = <Bytes as IntoBuf>::Buf;

/// A "trait alias" for `tower_http_service::Body` with bounds required by
/// tower-grpc.
///
/// Not to be implemented directly, but instead useful for reducing bounds
/// boilerplate.
pub trait Body: Sealed {
    type Data: Buf;
    type Error: Into<Error>;

    fn is_end_stream(&self) -> bool;

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, Self::Error>;

    fn poll_trailers(&mut self) -> Poll<Option<http::HeaderMap>, Self::Error>;
}

impl<T> Body for T
where
    T: HttpBody,
    T::Error: Into<Error>,
{
    type Data = T::Data;
    type Error = T::Error;

    fn is_end_stream(&self) -> bool {
        HttpBody::is_end_stream(self)
    }

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, Self::Error> {
        HttpBody::poll_data(self)
    }

    fn poll_trailers(&mut self) -> Poll<Option<http::HeaderMap>, Self::Error> {
        HttpBody::poll_trailers(self)
    }
}

impl<T> Sealed for T
where
    T: HttpBody,
    T::Error: Into<Error>,
{
}

/// Dynamic `Send` body object.
pub struct BoxBody {
    inner: Box<dyn Body<Data = BytesBuf, Error = Status> + Send>,
}

struct MapBody<B>(B);

// ===== impl BoxBody =====

impl BoxBody {
    /// Create a new `BoxBody` backed by `inner`.
    pub fn new(inner: Box<dyn Body<Data = BytesBuf, Error = Status> + Send>) -> Self {
        BoxBody { inner }
    }

    /// Create a new `BoxBody` mapping item and error to the default types.
    pub fn map_from<B>(inner: B) -> Self
    where
        B: Body + Send + 'static,
        B::Data: Into<Bytes>,
    {
        BoxBody::new(Box::new(MapBody(inner)))
    }
}

impl HttpBody for BoxBody {
    type Data = BytesBuf;
    type Error = Status;

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, Self::Error> {
        self.inner.poll_data()
    }

    fn poll_trailers(&mut self) -> Poll<Option<http::HeaderMap>, Self::Error> {
        self.inner.poll_trailers()
    }
}

impl fmt::Debug for BoxBody {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("BoxBody").finish()
    }
}

// ===== impl MapBody =====

impl<B> HttpBody for MapBody<B>
where
    B: Body,
    B::Data: Into<Bytes>,
{
    type Data = BytesBuf;
    type Error = Status;

    fn is_end_stream(&self) -> bool {
        self.0.is_end_stream()
    }

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, Self::Error> {
        let item = try_ready!(self.0.poll_data().map_err(Status::map_error));
        Ok(item.map(|buf| buf.into().into_buf()).into())
    }

    fn poll_trailers(&mut self) -> Poll<Option<http::HeaderMap>, Self::Error> {
        self.0.poll_trailers().map_err(Status::map_error)
    }
}

mod sealed {
    pub trait Sealed {}
}
