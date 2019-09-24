use self::sealed::Sealed;
use crate::error::{Error, Never};
use crate::Status;

use bytes::{Buf, Bytes, IntoBuf};
use futures::{try_ready, Async, Poll, Stream};
pub use http_body::Body as HttpBody;
use std::fmt;
use std::marker::PhantomData;

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

#[derive(Debug)]
pub struct NoBody<T> {
    pub(crate) _marker: PhantomData<T>,
}

#[derive(Debug)]
pub struct NoData;

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

// ===== impl NoBody =====

impl<T> Body for NoBody<T> {
    type Data = NoData;
    type Error = Error;

    fn is_end_stream(&self) -> bool {
        true
    }

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, Self::Error> {
        Ok(None.into())
    }

    fn poll_trailers(&mut self) -> Poll<Option<http::HeaderMap>, Self::Error> {
        Ok(None.into())
    }
}

impl<T> Stream for NoBody<T> {
    type Item = T;
    type Error = Never;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        Ok(Async::Ready(None))
    }
}

impl<T> Sealed for NoBody<T> {}

// ===== impl NoData =====

impl Buf for NoData {
    fn remaining(&self) -> usize {
        0
    }

    fn bytes(&self) -> &[u8] {
        &[]
    }

    fn advance(&mut self, _cnt: usize) {}
}

mod sealed {
    pub trait Sealed {}
}
