use prost::DecodeError;
use http::HeaderMap;
use h2;
use std;
use std::fmt;

#[derive(Debug)]
pub enum Error<T = ()> {
    Grpc(::Status, HeaderMap),
    Protocol(ProtocolError),
    Decode(DecodeError),
    Inner(T),
}

#[derive(Debug)]
pub enum ProtocolError {
    MissingTrailers,
    MissingMessage,
    UnexpectedEof,
    Internal,
    UnsupportedCompressionFlag(u8),
}

impl<T> fmt::Display for Error<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Grpc(ref _status, ref _header_map) =>
                write!(f, "gRPC error"),
            Error::Protocol(ref _protocol_error) =>
                write!(f, "Protocol error"),
            Error::Decode(ref _decode_error) =>
                write!(f, "Message decode error"),
            Error::Inner(ref _inner) =>
                write!(f, "Inner error"),
        }
    }
}

impl fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ProtocolError::MissingTrailers =>
                write!(f, "Missing trailers"),
            ProtocolError::MissingMessage =>
                write!(f, "Missing message"),
            ProtocolError::UnexpectedEof =>
                write!(f, "Unexpected EOF"),
            ProtocolError::Internal =>
                write!(f, "Internal"),
            ProtocolError::UnsupportedCompressionFlag(flag) =>
                write!(f, "Unsupported compression flag: {}", flag),
        }
    }
}

impl std::error::Error for ProtocolError {
    fn description(&self) -> &str {
        match *self {
            ProtocolError::MissingTrailers =>
                "Missing trailers",
            ProtocolError::MissingMessage =>
                "Missing message",
            ProtocolError::UnexpectedEof =>
                "Unexpected EOF",
            ProtocolError::Internal =>
                "Internal",
            ProtocolError::UnsupportedCompressionFlag(_) =>
                "Unsupported compression flag",
        }
    }
}

impl<T> std::error::Error for Error<T> where T : fmt::Debug {
    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            Error::Grpc(_, _) => None,
            Error::Protocol(ref protocol_error) => Some(protocol_error),
            Error::Decode(ref decode_error) => Some(decode_error),
            Error::Inner(ref _inner) => None,
        }
    }
}

impl<T> From<T> for Error<T> {
    fn from(inner: T) -> Self {
        Error::Inner(inner)
    }
}

impl From<Error<()>> for h2::Error {
    fn from(_err: Error<()>) -> Self {
        // TODO: implement
        h2::Reason::INTERNAL_ERROR.into()
    }
}

impl From<h2::Error> for Error<()> {
    fn from(_: h2::Error) -> Self {
        // TODO: implement
        Error::Inner(())
    }
}
