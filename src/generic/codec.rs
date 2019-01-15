use {Body, Status};

use bytes::{Buf, BufMut, BytesMut, Bytes, IntoBuf};
use futures::{Stream, Poll, Async};
use http::{HeaderMap, StatusCode};

use std::collections::VecDeque;
use std::fmt;

use status::infer_grpc_status;

/// Encodes and decodes gRPC message types
pub trait Codec {
    /// The encode type
    type Encode;

    /// Encoder type
    type Encoder: Encoder<Item = Self::Encode>;

    /// The decode type
    type Decode;

    /// Decoder type
    type Decoder: Decoder<Item = Self::Decode>;

    /// Returns a new encoder
    fn encoder(&mut self) -> Self::Encoder;

    /// Returns a new decoder
    fn decoder(&mut self) -> Self::Decoder;
}

/// Encodes gRPC message types
pub trait Encoder {
    /// Type that is encoded
    type Item;

    /// The content-type header for messages using this encoding.
    ///
    /// Should be `application/grpc+yourencoding`.
    const CONTENT_TYPE: &'static str;

    /// Encode a message into the provided buffer.
    fn encode(&mut self, item: Self::Item, buf: &mut EncodeBuf) -> Result<(), ::Error>;
}

/// Decodes gRPC message types
pub trait Decoder {
    /// Type that is decoded
    type Item;

    /// Decode a message from the buffer.
    ///
    /// The buffer will contain exactly the bytes of a full message. There
    /// is no need to get the length from the bytes, gRPC framing is handled
    /// for you.
    fn decode(&mut self, buf: &mut DecodeBuf) -> Result<Self::Item, ::Error>;
}

/// Encodes gRPC message types
#[must_use = "futures do nothing unless polled"]
#[derive(Debug)]
pub struct Encode<T, U> {
    inner: EncodeInner<T, U>,

    /// Destination buffer
    buf: BytesMut,

    /// Set to true when trailers should be generated.
    return_trailers: bool,
}

#[derive(Debug)]
enum EncodeInner<T, U> {
    Ok {
        /// The encoder
        encoder: T,

        /// The source of messages to encode
        inner: U,
    },
    Err(Status),
}

/// An stream of inbound gRPC messages
#[must_use = "futures do nothing unless polled"]
pub struct Streaming<T, B: Body> {
    /// The decoder
    decoder: T,

    /// The source of encoded messages
    inner: B,

    /// buffer
    bufs: BufList<<B::Data as IntoBuf>::Buf>,

    /// Decoding state
    state: State,

    direction: Direction,
}

/// Whether this is reading a request or a response stream value.
#[derive(Clone, Copy, Debug)]
pub enum Direction {
    /// For requests, we expect only headers and the streaming body.
    Request,
    /// For responses, the received HTTP status code must be provided.
    /// We also expect to receive trailers after the streaming body.
    Response(StatusCode),
    /// For streaming responses with zero response payloads, the HTTP
    /// status is provided immediately. In this case no additional
    /// trailers are expected.
    EmptyResponse
}

#[derive(Debug)]
enum State {
    ReadHeader,
    ReadBody {
        compression: bool,
        len: usize,
    },
    Done,
}

/// A buffer to encode a message into.
#[derive(Debug)]
pub struct EncodeBuf<'a> {
    bytes: &'a mut BytesMut,
}

/// A buffer to decode messages from.
pub struct DecodeBuf<'a> {
    bufs: &'a mut Buf,
    len: usize,
}

#[derive(Debug)]
pub struct BufList<B> {
    bufs: VecDeque<B>,
}

// ===== impl Encode =====

impl<T, U> Encode<T, U>
where T: Encoder<Item = U::Item>,
      U: Stream,
{
    pub(crate) fn new(encoder: T, inner: U, return_trailers: bool) -> Self {
        Encode {
            inner: EncodeInner::Ok { encoder, inner },
            buf: BytesMut::new(),
            return_trailers,
        }
    }

    pub(crate) fn error(status: Status) -> Self {
        Encode {
            inner: EncodeInner::Err(status),
            buf: BytesMut::new(),
            return_trailers: true,
        }
    }
}

impl<T, U> Body for Encode<T, U>
where T: Encoder<Item = U::Item>,
      U: Stream,
{
    type Data = Bytes;

    fn is_end_stream(&self) -> bool {
        false
    }

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, ::Error> {
        match self.inner {
            EncodeInner::Ok { ref mut inner, ref mut encoder } => {
                let item = try_ready!(inner.poll().map_err(|_| ::Error::Inner(())));

                if let Some(item) = item {
                    self.buf.reserve(5);
                    unsafe { self.buf.advance_mut(5); }
                    encoder.encode(item, &mut EncodeBuf {
                        bytes: &mut self.buf,
                    })?;

                    // now that we know length, we can write the header
                    let len = self.buf.len() - 5;
                    assert!(len <= ::std::u32::MAX as usize);
                    {
                        let mut cursor = ::std::io::Cursor::new(&mut self.buf[..5]);
                        cursor.put_u8(0); // byte must be 0, reserve doesn't auto-zero
                        cursor.put_u32_be(len as u32);
                    }

                    Ok(Async::Ready(Some(self.buf.split_to(len + 5).freeze())))
                } else {
                    Ok(Async::Ready(None))
                }
            }
            _ => Ok(Async::Ready(None)),
        }
    }

    fn poll_metadata(&mut self) -> Poll<Option<HeaderMap>, ::Error> {
        if !self.return_trailers {
            return Ok(Async::Ready(None));
        }

        let map = match self.inner {
            EncodeInner::Ok { .. } => Status::with_code(::Code::Ok).to_header_map(),
            EncodeInner::Err(ref status) => status.to_header_map(),
        };
        Ok(Some(map?).into())
    }
}

#[cfg(feature = "tower-h2")]
impl<T, U> ::tower_h2::Body for Encode<T, U>
where T: Encoder<Item = U::Item>,
      U: Stream,
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

// ===== impl Streaming =====

impl<T, U> Streaming<T, U>
where T: Decoder,
      U: Body,
{
    pub(crate) fn new(decoder: T, inner: U, direction: Direction) -> Self {
        Streaming {
            decoder,
            inner,
            bufs: BufList {
                bufs: VecDeque::new(),
            },
            state: State::ReadHeader,
            direction,
        }
    }

    fn decode(&mut self) -> Result<Option<T::Item>, ::Error> {
        if let State::ReadHeader = self.state {
            if self.bufs.remaining() < 5 {
                return Ok(None);
            }

            let is_compressed = match self.bufs.get_u8() {
                0 => false,
                1 => {
                    trace!("message compressed, compression not supported yet");
                    return Err(::Error::Grpc(::Status::with_code_and_message(
                        ::Code::Unimplemented,
                        "Message compressed, compression not supported yet.".to_string())));
                },
                f => {
                    trace!("unexpected compression flag");
                    return Err(::Error::Grpc(::Status::with_code_and_message(
                        ::Code::Internal,
                        format!("Unexpected compression flag: {}", f))));
                }
            };
            let len = self.bufs.get_u32_be() as usize;

            self.state = State::ReadBody {
                compression: is_compressed,
                len,
            }
        }

        if let State::ReadBody { len, .. } = self.state {
            if self.bufs.remaining() < len {
                return Ok(None);
            }

            match self.decoder.decode(&mut DecodeBuf {
                bufs: &mut self.bufs,
                len,
            }) {
                Ok(msg) => {
                    self.state = State::ReadHeader;
                    return Ok(Some(msg));
                },
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(None)
    }
}

impl<T, U> Stream for Streaming<T, U>
where T: Decoder,
      U: Body,
{
    type Item = T::Item;
    type Error = ::Error;

    fn poll(&mut self) -> Poll<Option<Self::Item>, Self::Error> {
        loop {
            if let State::Done = self.state {
                break;
            }

            match self.decode()? {
                Some(val) => return Ok(Async::Ready(Some(val))),
                None => (),
            }

            let chunk = try_ready!(self.inner.poll_data());

            if let Some(data) = chunk {
                self.bufs.bufs.push_back(data.into_buf());
            } else {
                if self.bufs.has_remaining() {
                    trace!("unexpected EOF decoding stream");
                    return Err(::Error::Grpc(::Status::with_code_and_message(
                        ::Code::Internal,
                        "Unexpected EOF decoding stream.".to_string())))
                } else {
                    self.state = State::Done;
                    break;
                }
            }
        }

        if let Direction::Response(status_code) = self.direction {
            match infer_grpc_status(try_ready!(self.inner.poll_metadata()), status_code) {
                Ok(_) => Ok(Async::Ready(None)),
                Err(err) => Err(err),
            }
        } else {
            Ok(Async::Ready(None))
        }
    }
}

impl<T, B> fmt::Debug for Streaming<T, B>
where
    T: fmt::Debug,
    B: Body + fmt::Debug,
    <B::Data as IntoBuf>::Buf: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Streaming")
            .finish()
    }
}

// ===== impl EncodeBuf =====

impl<'a> EncodeBuf<'a> {
    #[inline]
    pub fn reserve(&mut self, capacity: usize) {
        self.bytes.reserve(capacity);
    }
}

impl<'a> BufMut for EncodeBuf<'a> {
    #[inline]
    fn remaining_mut(&self) -> usize {
        self.bytes.remaining_mut()
    }

    #[inline]
    unsafe fn advance_mut(&mut self, cnt: usize) {
        self.bytes.advance_mut(cnt)
    }

    #[inline]
    unsafe fn bytes_mut(&mut self) -> &mut [u8] {
        self.bytes.bytes_mut()
    }
}

// ===== impl DecodeBuf =====

impl<'a> Buf for DecodeBuf<'a> {
    #[inline]
    fn remaining(&self) -> usize {
        self.len
    }

    #[inline]
    fn bytes(&self) -> &[u8] {
        let ret = self.bufs.bytes();

        if ret.len() > self.len {
            &ret[..self.len]
        } else {
            ret
        }
    }

    #[inline]
    fn advance(&mut self, cnt: usize) {
        assert!(cnt <= self.len);
        self.bufs.advance(cnt);
        self.len -= cnt;
    }
}

impl<'a> fmt::Debug for DecodeBuf<'a> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("DecodeBuf")
            .finish()
    }
}

impl<'a> Drop for DecodeBuf<'a> {
    fn drop(&mut self) {
        if self.len > 0 {
            warn!("DecodeBuf was not advanced to end");
            self.bufs.advance(self.len);
        }
    }
}

// ===== impl BufList =====

impl<T: Buf> Buf for BufList<T> {
    #[inline]
    fn remaining(&self) -> usize {
        self.bufs.iter()
            .map(|buf| buf.remaining())
            .sum()
    }

    #[inline]
    fn bytes(&self) -> &[u8] {
        if self.bufs.is_empty() {
            &[]
        } else {
            self.bufs[0].bytes()
        }
    }

    #[inline]
    fn advance(&mut self, mut cnt: usize) {
        while cnt > 0 {
            {
                let front = &mut self.bufs[0];
                if front.remaining() > cnt {
                    front.advance(cnt);
                    return;
                } else {
                    cnt -= front.remaining();
                }
            }
            self.bufs.pop_front();
        }
    }
}

