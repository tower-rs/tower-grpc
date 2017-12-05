pub use generic::server::unary::Once;

use codec::{Encode, Encoder, Decoder};
use generic::Streaming;
use generic::server::{UnaryService, unary};

use {h2, http, prost};
use futures::{Future, Poll};
use tower_h2::{Body, Data};

use std::fmt;

pub struct ResponseFuture<T, B>
where T: UnaryService,
      T::Request: prost::Message + Default,
      T::Response: prost::Message,
{
    inner: Inner<T, T::Response, T::Request, B>,
}

type Inner<T, U, V, B> =
    unary::ResponseFuture<T, Encoder<U>, Streaming<Decoder<V>, B>>;

impl<T, B> ResponseFuture<T, B>
where T: UnaryService,
      T::Request: prost::Message + Default,
      T::Response: prost::Message,
{
    pub(crate) fn new(inner: Inner<T, T::Response, T::Request, B>) -> Self {
        ResponseFuture { inner }
    }
}

impl<T, B> Future for ResponseFuture<T, B>
where T: UnaryService,
      T::Request: prost::Message + Default,
      T::Response: prost::Message,
      B: Body<Data = Data>,
{
    type Item = http::Response<Encode<Once<T::Response>>>;
    type Error = h2::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let response = try_ready!(self.inner.poll());
        let (head, body) = response.into_parts();
        let body = Encode::new(body);
        Ok(http::Response::from_parts(head, body).into())
    }
}

impl<T, B> fmt::Debug for ResponseFuture<T, B>
where T: UnaryService + fmt::Debug,
      T::Request: prost::Message + Default + fmt::Debug,
      T::Response: prost::Message + fmt::Debug,
      T::Future: fmt::Debug,
      B: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("unary::ResponseFuture")
            .field("inner", &self.inner)
            .finish()
    }
}
