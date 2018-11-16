use body::{Body, BoxBody};
use generic::{EncodeBuf, DecodeBuf};

use futures::{Stream, Poll};
use bytes::BufMut;
use http;
use prost::Message;

use std::fmt;
use std::marker::PhantomData;

/// Protobuf codec
#[derive(Debug)]
pub struct Codec<T, U>(PhantomData<(T, U)>);

#[derive(Debug)]
pub struct Encoder<T>(PhantomData<T>);

#[derive(Debug)]
pub struct Decoder<T>(PhantomData<T>);

/// A stream of inbound gRPC messages
pub type Streaming<T, B = BoxBody> = ::generic::Streaming<Decoder<T>, B>;

pub use ::generic::Direction;

/// A protobuf encoded gRPC response body
pub struct Encode<T>
where T: Stream,
{
    inner: ::generic::Encode<Encoder<T::Item>, T>,
}

// ===== impl Codec =====

impl<T, U> Codec<T, U>
where T: Message,
      U: Message + Default,
{
    /// Create a new protobuf codec
    pub fn new() -> Self {
        Codec(PhantomData)
    }
}

impl<T, U> ::generic::Codec for Codec<T, U>
where T: Message,
      U: Message + Default,
{
    type Encode = T;
    type Encoder = Encoder<T>;
    type Decode = U;
    type Decoder = Decoder<U>;

    fn encoder(&mut self) -> Self::Encoder {
        Encoder(PhantomData)
    }

    fn decoder(&mut self) -> Self::Decoder {
        Decoder(PhantomData)
    }
}

impl<T, U> Clone for Codec<T, U> {
    fn clone(&self) -> Self {
        Codec(PhantomData)
    }
}

// ===== impl Encoder =====

impl<T> Encoder<T>
where T: Message
{
    pub fn new() -> Self {
        Encoder(PhantomData)
    }
}

impl<T> ::generic::Encoder for Encoder<T>
where T: Message,
{
    type Item = T;

    /// Protocol buffer gRPC content type
    const CONTENT_TYPE: &'static str = "application/grpc+proto";

    fn encode(&mut self, item: T, buf: &mut EncodeBuf) -> Result<(), ::Error> {
        let len = item.encoded_len();

        if buf.remaining_mut() < len {
            buf.reserve(len);
        }

        item.encode(buf)
            .map_err(|_| unreachable!("Message only errors if not enough space"))
    }
}

impl<T> Clone for Encoder<T> {
    fn clone(&self) -> Self {
        Encoder(PhantomData)
    }
}

// ===== impl Decoder =====

impl<T> Decoder<T>
where T: Message + Default,
{
    /// Returns a new decoder
    pub fn new() -> Self {
        Decoder(PhantomData)
    }
}

impl<T> ::generic::Decoder for Decoder<T>
where T: Message + Default,
{
    type Item = T;

    fn decode(&mut self, buf: &mut DecodeBuf) -> Result<T, ::Error> {
        Message::decode(buf)
            .map_err(::Error::Decode)
    }
}

impl<T> Clone for Decoder<T> {
    fn clone(&self) -> Self {
        Decoder(PhantomData)
    }
}

// ===== impl Encode =====

impl<T> Encode<T>
where T: Stream<Error = ::Error>,
      T::Item: ::prost::Message,
{
    pub(crate) fn new(inner: ::generic::Encode<Encoder<T::Item>, T>) -> Self {
        Encode { inner }
    }
}

impl<T> Body for Encode<T>
where T: Stream<Error = ::Error>,
      T::Item: ::prost::Message,
{
    type Data = ::bytes::Bytes;

    fn is_end_stream(&self) -> bool {
        false
    }

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, ::Error> {
        self.inner.poll_data()
    }

    fn poll_metadata(&mut self) -> Poll<Option<http::HeaderMap>, ::Error> {
        self.inner.poll_metadata()
    }
}

#[cfg(feature = "tower-h2")]
impl<T> ::tower_h2::Body for Encode<T>
where T: Stream<Error = ::Error>,
      T::Item: ::prost::Message,
{
    type Data = ::bytes::Bytes;

    fn is_end_stream(&self) -> bool {
        Body::is_end_stream(self)
    }

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, ::h2::Error> {
        Body::poll_data(self)
            .map_err(From::from)
    }

    fn poll_trailers(&mut self) -> Poll<Option<http::HeaderMap>, ::h2::Error> {
        Body::poll_metadata(self)
            .map_err(From::from)
    }
}

impl<T> fmt::Debug for Encode<T>
where T: Stream + fmt::Debug,
      T::Item: fmt::Debug,
      T::Error: fmt::Debug,
{
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("Encode")
            .field("inner", &self.inner)
            .finish()
    }
}

