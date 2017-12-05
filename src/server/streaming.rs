use codec::{Encode, Encoder};
use generic::server::{StreamingService, streaming};

use {h2, http, prost};
use futures::{Future, Poll};

use std::fmt;

pub struct ResponseFuture<T>
where T: StreamingService,
      T::Request: prost::Message + Default,
      T::Response: prost::Message,
{
    inner: Inner<T::Future, T::Response>,
}

type Inner<T, U> =
    streaming::ResponseFuture<T, Encoder<U>>;

impl<T> ResponseFuture<T>
where T: StreamingService,
      T::Request: prost::Message + Default,
      T::Response: prost::Message,
{
    pub(crate) fn new(inner: Inner<T::Future, T::Response>) -> Self {
        ResponseFuture { inner }
    }
}

impl<T> Future for ResponseFuture<T>
where T: StreamingService,
      T::Request: prost::Message + Default,
      T::Response: prost::Message,
{
    type Item = http::Response<Encode<T::ResponseStream>>;
    type Error = h2::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let response = try_ready!(self.inner.poll());
        let (head, body) = response.into_parts();
        let body = Encode::new(body);
        Ok(http::Response::from_parts(head, body).into())
    }
}

impl<T> fmt::Debug for ResponseFuture<T>
where T: StreamingService + fmt::Debug,
      T::Request: prost::Message + Default + fmt::Debug,
      T::Response: prost::Message + fmt::Debug,
      T::RequestStream: fmt::Debug,
      T::ResponseStream: fmt::Debug,
      T::Future: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("streaming::ResponseFuture")
            .field("inner", &self.inner)
            .finish()
    }
}
