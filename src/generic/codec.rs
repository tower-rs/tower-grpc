use Status;

use bytes::{Buf, BufMut, BytesMut, Bytes};
use futures::{Stream, Poll, Async};
use h2;
use http::HeaderMap;
use tower_h2::{self, Body, Data};

use std::collections::VecDeque;

use error::ProtocolError;

/// Encodes and decodes gRPC message types
pub trait Codec {
    /// The content-type header for messages using this encoding.
    ///
    /// Should be `application/grpc+yourencoding`.
    const CONTENT_TYPE: &'static str;

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
#[derive(Debug)]
pub struct Streaming<T, U = tower_h2::RecvBody> {
    /// The decoder
    decoder: T,

    /// The source of encoded messages
    inner: U,

    /// buffer
    bufs: BytesList,

    /// Decoding state
    state: State,

    /// Set to true when expecting trailers
    expect_trailers: bool,
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
#[derive(Debug)]
pub struct DecodeBuf<'a> {
    bufs: &'a mut BytesList,
    len: usize,
}

#[derive(Debug)]
pub struct BytesList {
    bufs: VecDeque<Bytes>,
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

impl<T, U> tower_h2::Body for Encode<T, U>
where T: Encoder<Item = U::Item>,
      U: Stream,
{
    type Data = Bytes;

    fn is_end_stream(&self) -> bool {
        false
    }

    fn poll_data(&mut self) -> Poll<Option<Self::Data>, h2::Error> {
        match self.inner {
            EncodeInner::Ok { ref mut inner, ref mut encoder } => {
                let item = try_ready!(inner.poll().map_err(|_| h2_err()));

                if let Some(item) = item {
                    self.buf.reserve(5);
                    unsafe { self.buf.advance_mut(5); }
                    encoder.encode(item, &mut EncodeBuf {
                        bytes: &mut self.buf,
                    }).map_err(|_| h2_err())?;

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

    fn poll_trailers(&mut self) -> Poll<Option<HeaderMap>, h2::Error> {
        if !self.return_trailers {
            return Ok(Async::Ready(None));
        }

        let mut map = HeaderMap::new();

        let status = match self.inner {
            EncodeInner::Ok { .. } => Status::OK.to_header_value(),
            EncodeInner::Err(ref status) => status.to_header_value(),
        };

        // Success
        map.insert("grpc-status", status);

        Ok(Some(map).into())
    }
}

// ===== impl Streaming =====

impl<T, U> Streaming<T, U>
where T: Decoder,
      U: Body<Data = Data>,
{
    pub(crate) fn new(decoder: T, inner: U, expect_trailers: bool) -> Self {
        Streaming {
            decoder,
            inner,
            bufs: BytesList {
                bufs: VecDeque::new(),
            },
            state: State::ReadHeader,
            expect_trailers,
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
                    return Err(::Error::Protocol(ProtocolError::UnsupportedCompressionFlag(1)));
                },
                f => {
                    trace!("unexpected compression flag");
                    return Err(::Error::Protocol(ProtocolError::UnsupportedCompressionFlag(f)));
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
      U: Body<Data = Data>,
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
                self.bufs.bufs.push_back(data.into());
            } else {
                if self.bufs.has_remaining() {
                    trace!("unexpected EOF decoding stream");
                    return Err(::Error::Protocol(ProtocolError::UnexpectedEof))
                } else {
                    self.state = State::Done;
                    break;
                }
            }
        }

        if self.expect_trailers {
            if let Some(trailers) = try_ready!(self.inner.poll_trailers()) {
                grpc_status(trailers)?;
                Ok(Async::Ready(None))
            } else {
                trace!("receive body ended without trailers");
                Err(::Error::Protocol(ProtocolError::MissingTrailers))
            }
        } else {
            Ok(Async::Ready(None))
        }
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

impl<'a> Drop for DecodeBuf<'a> {
    fn drop(&mut self) {
        if self.len > 0 {
            warn!("DecodeBuf was not advanced to end");
            self.bufs.advance(self.len);
        }
    }
}

// ===== impl BytesList =====

impl Buf for BytesList {
    #[inline]
    fn remaining(&self) -> usize {
        self.bufs.iter()
            .map(|buf| buf.len())
            .sum()
    }

    #[inline]
    fn bytes(&self) -> &[u8] {
        if self.bufs.is_empty() {
            &[]
        } else {
            &self.bufs[0][..]
        }
    }

    #[inline]
    fn advance(&mut self, mut cnt: usize) {
        while cnt > 0 {
            {
                let front = &mut self.bufs[0];
                if front.len() > cnt {
                    front.advance(cnt);
                    return;
                } else {
                    cnt -= front.len();
                }
            }
            self.bufs.pop_front();
        }
    }
}

// ===== impl utils =====

fn h2_err() -> h2::Error {
    unimplemented!("EncodingBody map_err")
}

fn grpc_status(mut trailers: HeaderMap) -> Result<(), ::Error> {
    if let Some(status) = trailers.remove("grpc-status") {
        let status = Status::from_bytes(status.as_ref());
        if status.code() == ::Code::OK {
            Ok(())
        } else {
            Err(::Error::Grpc(status, trailers))
        }
    } else {
        trace!("trailers missing grpc-status");
        Err(::Error::Protocol(ProtocolError::MissingTrailers))
    }
}
