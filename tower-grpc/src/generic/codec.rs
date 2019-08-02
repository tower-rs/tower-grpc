use crate::body::{Body, HttpBody};
use crate::error::Error;
use crate::status::infer_grpc_status;
use crate::Status;

use bytes::{Buf, BufMut, Bytes, BytesMut, IntoBuf};
use futures::{ready, Stream, TryStream};
use http::{HeaderMap, StatusCode};
use log::{debug, trace, warn};
use std::collections::VecDeque;
use std::fmt;
use std::pin::Pin;
use std::task::{Context, Poll};

type BytesBuf = <Bytes as IntoBuf>::Buf;

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
    fn encode(&mut self, item: Self::Item, buf: &mut EncodeBuf<'_>) -> Result<(), Status>;
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
    fn decode(&mut self, buf: &mut DecodeBuf<'_>) -> Result<Self::Item, Status>;
}

/// Encodes gRPC message types
#[must_use = "futures do nothing unless polled"]
#[derive(Debug)]
pub struct Encode<T, U> {
    inner: EncodeInner<T, U>,

    /// Destination buffer
    buf: BytesMut,

    role: Role,
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

#[derive(Debug)]
enum Role {
    Client,
    Server,
}

/// An stream of inbound gRPC messages
#[must_use = "futures do nothing unless polled"]
pub struct Streaming<T, B: Body> {
    /// The decoder
    decoder: T,

    /// The source of encoded messages
    inner: B,

    /// buffer
    bufs: BufList<B::Data>,

    /// Decoding state
    state: State,

    direction: Direction,
}

/// Whether this is a request or a response stream value.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Direction {
    /// For requests, we expect only headers and the streaming body.
    Request,
    /// For responses, the received HTTP status code must be provided.
    /// We also expect to receive trailers after the streaming body.
    Response(StatusCode),
    /// For streaming responses with zero response payloads, the HTTP
    /// status is provided immediately. In this case no additional
    /// trailers are expected.
    EmptyResponse,
}

#[derive(Debug)]
enum State {
    ReadHeader,
    ReadBody { compression: bool, len: usize },
    Done,
}

/// A buffer to encode a message into.
#[derive(Debug)]
pub struct EncodeBuf<'a> {
    bytes: &'a mut BytesMut,
}

/// A buffer to decode messages from.
pub struct DecodeBuf<'a> {
    bufs: &'a mut dyn Buf,
    len: usize,
}

#[derive(Debug)]
pub struct BufList<B> {
    bufs: VecDeque<B>,
}

// ===== impl Encode =====

impl<T, U> Encode<T, U>
where
    T: Encoder<Item = U::Ok>,
    U: TryStream,
    U::Error: Into<Error>,
{
    fn new(encoder: T, inner: U, role: Role) -> Self {
        Encode {
            inner: EncodeInner::Ok { encoder, inner },
            buf: BytesMut::new(),
            role,
        }
    }

    pub(crate) fn request(encoder: T, inner: U) -> Self {
        Encode::new(encoder, inner, Role::Client)
    }

    pub(crate) fn response(encoder: T, inner: U) -> Self {
        Encode::new(encoder, inner, Role::Server)
    }

    pub(crate) fn error(status: Status) -> Self {
        Encode {
            inner: EncodeInner::Err(status),
            buf: BytesMut::new(),
            role: Role::Server,
        }
    }
}

impl<T, U> HttpBody for Encode<T, U>
where
    T: Encoder<Item = U::Ok> + Unpin,
    U: TryStream + Unpin,
    U::Error: Into<Error>,
{
    type Data = BytesBuf;
    type Error = Status;

    fn is_end_stream(&self) -> bool {
        false
    }

    fn poll_data(&mut self, cx: &mut Context<'_>) -> Poll<Option<Result<Self::Data, Status>>> {
        let me = &mut *self;

        match ready!(Pin::new(&mut me.inner).poll_encode(cx, &mut me.buf)) {
            Some(Ok(ok)) => Some(Ok(ok)).into(),
            Some(Err(status)) => {
                match self.role {
                    // clients don't send statuses as trailers, so just return
                    // this error directly to allow an HTTP2 rst_stream to be
                    // sent.
                    Role::Client => Some(Err(status)).into(),
                    // otherwise, its better to send this status in the
                    // trailers, instead of a RST_STREAM as the server...
                    Role::Server => {
                        self.inner = EncodeInner::Err(status);
                        None.into()
                    }
                }
            }
            None => None.into(),
        }
    }

    fn poll_trailers(&mut self, _cx: &mut Context<'_>) -> Poll<Result<Option<HeaderMap>, Status>> {
        if let Role::Client = self.role {
            return Ok(None).into();
        }

        let map = match self.inner {
            EncodeInner::Ok { .. } => Status::new(crate::Code::Ok, "").to_header_map(),
            EncodeInner::Err(ref status) => status.to_header_map(),
        };

        Ok(Some(map?)).into()
    }
}

impl<T, U> EncodeInner<T, U>
where
    T: Encoder<Item = U::Ok> + Unpin,
    U: TryStream + Unpin,
    U::Error: Into<Error>,
{
    fn poll_encode(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut BytesMut,
    ) -> Poll<Option<Result<BytesBuf, Status>>> {
        match &mut *self {
            EncodeInner::Ok { inner, encoder } => {
                let item = ready!(Pin::new(inner).try_poll_next(cx)).map(|r| {
                    r.map_err(|err| {
                        let err = err.into();
                        debug!("encoder inner stream error: {:?}", err);
                        Status::from_error(&*err)
                    })
                });

                let item = if let Some(Ok(item)) = item {
                    buf.reserve(5);
                    unsafe {
                        buf.advance_mut(5);
                    }
                    encoder.encode(item, &mut EncodeBuf { bytes: buf })?;

                    // now that we know length, we can write the header
                    let len = buf.len() - 5;
                    assert!(len <= ::std::u32::MAX as usize);
                    {
                        let mut cursor = ::std::io::Cursor::new(&mut buf[..5]);
                        cursor.put_u8(0); // byte must be 0, reserve doesn't auto-zero
                        cursor.put_u32_be(len as u32);
                    }

                    Ok(buf.split_to(len + 5).freeze().into_buf())
                } else if let Some(Err(e)) = item {
                    Err(e).into()
                } else {
                    return None.into();
                };

                Some(item).into()
            }

            _ => None.into(),
        }
    }
}

// ===== impl Streaming =====
impl<T, U: Body> Unpin for Streaming<T, U> {}
impl<T, U> Streaming<T, U>
where
    T: Decoder + Unpin,
    U: Body + Unpin,
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

    fn decode(&mut self) -> Result<Option<T::Item>, crate::Status> {
        if let State::ReadHeader = self.state {
            if self.bufs.remaining() < 5 {
                return Ok(None);
            }

            let is_compressed = match self.bufs.get_u8() {
                0 => false,
                1 => {
                    trace!("message compressed, compression not supported yet");
                    return Err(crate::Status::new(
                        crate::Code::Unimplemented,
                        "Message compressed, compression not supported yet.".to_string(),
                    ));
                }
                f => {
                    trace!("unexpected compression flag");
                    return Err(crate::Status::new(
                        crate::Code::Internal,
                        format!("Unexpected compression flag: {}", f),
                    ));
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
                }
                Err(e) => {
                    return Err(e);
                }
            }
        }

        Ok(None)
    }
}

impl<T, U> Stream for Streaming<T, U>
where
    T: Decoder + Unpin,
    U: Body + Unpin,
{
    type Item = Result<T::Item, Status>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        loop {
            if let State::Done = self.state {
                break;
            }

            let me = &mut *self;
            match me.decode()? {
                Some(val) => return Some(Ok(val)).into(),
                None => (),
            }

            let chunk = ready!(Pin::new(&mut self.inner).poll_data(cx)).map(|r| {
                r.map_err(|err| {
                    let err = err.into();
                    debug!("decoder inner stream error: {:?}", err);
                    Status::from_error(&*err)
                })
            });

            if let Some(chunk) = chunk {
                match chunk {
                    Ok(data) => self.bufs.bufs.push_back(data.into_buf()),
                    Err(e) => return Some(Err(e)).into(),
                }
            } else {
                if self.bufs.has_remaining() {
                    trace!("unexpected EOF decoding stream");
                    return Some(Err(crate::Status::new(
                        crate::Code::Internal,
                        "Unexpected EOF decoding stream.".to_string(),
                    )))
                    .into();
                } else {
                    self.state = State::Done;
                    break;
                }
            }
        }

        if let Direction::Response(status_code) = self.direction {
            let trailers = ready!(Pin::new(&mut self.inner).poll_trailers(cx).map_err(|err| {
                let err = err.into();
                debug!("decoder inner trailers error: {:?}", err);
                Status::from_error(&*err)
            }))?;
            match infer_grpc_status(trailers, status_code) {
                Ok(_) => None.into(),
                Err(err) => Some(Err(err)).into(),
            }
        } else {
            None.into()
        }
    }
}

impl<T, B> fmt::Debug for Streaming<T, B>
where
    T: fmt::Debug,
    B: Body + fmt::Debug,
    B::Data: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Streaming").finish()
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
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DecodeBuf").finish()
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
        self.bufs.iter().map(|buf| buf.remaining()).sum()
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
