use bytes::Bytes;
use futures::Poll;
use h2;
use http::HeaderMap;
use tower_h2::Body;

use std::fmt;

pub struct BoxBody {
    inner: Box<Body<Data = Bytes> + Send + 'static>,
}

pub struct UnsyncBoxBody {
    inner: Box<Body<Data = Bytes> + 'static>,
}

impl BoxBody {
    pub fn new(inner: Box<Body<Data = Bytes> + Send + 'static>) -> Self {
        BoxBody { inner }
    }
}

impl Body for BoxBody {
    type Data = Bytes;

    fn is_end_stream(&self) -> bool {
        self.inner.is_end_stream()
    }

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, h2::Error> {
        self.inner.poll_data()
    }

    fn poll_trailers(&mut self) -> Poll<Option<HeaderMap>, h2::Error> {
        self.inner.poll_trailers()
    }
}

impl fmt::Debug for BoxBody {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("BoxBody")
            .finish()
    }
}

impl fmt::Debug for UnsyncBoxBody {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("UnsyncBoxBody")
            .finish()
    }
}
