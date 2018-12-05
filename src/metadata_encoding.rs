use bytes::Bytes;
use http::header::HeaderValue;
use std::fmt;
use std::error::Error;
use std::hash::Hash;

/// A possible error when converting a `MetadataValue` from a string or byte
/// slice.
#[derive(Debug, Default)]
pub struct InvalidMetadataValue {
    _priv: (),
}

// TODO(pgron): Make sealed
pub trait ValueEncoding: Clone + Eq + PartialEq + Hash {
    #[doc(hidden)]
    fn is_valid_key(key: &str) -> bool;

    #[doc(hidden)]
    fn is_empty(value: &[u8]) -> bool;

    #[doc(hidden)]
    fn from_shared(value: Bytes) -> Result<HeaderValue, InvalidMetadataValueBytes>;

    #[doc(hidden)]
    fn decode(value: &[u8]) -> Result<Bytes, InvalidMetadataValueBytes>;
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Ascii {}
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Binary {}

// ===== impl ValueEncoding =====

impl ValueEncoding for Ascii {
    fn is_valid_key(key: &str) -> bool {
        !Binary::is_valid_key(key)
    }

    fn is_empty(value: &[u8]) -> bool {
        value.is_empty()
    }

    fn from_shared(value: Bytes) -> Result<HeaderValue, InvalidMetadataValueBytes> {
       HeaderValue::from_shared(value)
            .map_err(|_| {
                InvalidMetadataValueBytes::new()
            })
    }

    #[doc(hidden)]
    fn decode(value: &[u8]) -> Result<Bytes, InvalidMetadataValueBytes> {
        Ok(Bytes::from(value))
    }
}

impl ValueEncoding for Binary {
    fn is_valid_key(key: &str) -> bool {
        key.ends_with("-bin")
    }

    fn is_empty(value: &[u8]) -> bool {
        // TODO(pgron): Do this properly for base64
        value.is_empty()
    }

    fn from_shared(value: Bytes) -> Result<HeaderValue, InvalidMetadataValueBytes> {
        // TODO(pgron): Do this properly for base64
       HeaderValue::from_shared(value)
            .map_err(|_| {
                InvalidMetadataValueBytes::new()
            })
    }

    #[doc(hidden)]
    fn decode(value: &[u8]) -> Result<Bytes, InvalidMetadataValueBytes> {
        // TODO(pgron): Do this properly for base64
        Ok(Bytes::from(value))
    }
}

// ===== impl InvalidMetadataValue =====

impl InvalidMetadataValue {
    pub fn new() -> Self {
        Default::default()
    }
}

impl fmt::Display for InvalidMetadataValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.description().fmt(f)
    }
}

impl Error for InvalidMetadataValue {
    fn description(&self) -> &str {
        "failed to parse metadata value"
    }
}

/// A possible error when converting a `MetadataValue` from a string or byte
/// slice.
#[derive(Debug, Default)]
pub struct InvalidMetadataValueBytes(InvalidMetadataValue);

// ===== impl InvalidMetadataValueBytes =====

impl InvalidMetadataValueBytes {
    pub fn new() -> Self {
        Default::default()
    }
}

impl fmt::Display for InvalidMetadataValueBytes {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl Error for InvalidMetadataValueBytes {
    fn description(&self) -> &str {
        self.0.description()
    }
}
