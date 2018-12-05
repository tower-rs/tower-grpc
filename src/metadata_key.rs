use bytes::Bytes;
use http;
use http::header::HeaderName;

use std::borrow::Borrow;
use std::error::Error;
use std::fmt;
use std::str::FromStr;

/// Represents a custom metadata field name.
///
/// `MetadataKey` is used as the [`MetadataMap`] key.
///
/// [`HeaderMap`]: struct.HeaderMap.html
#[derive(Clone, Eq, PartialEq, Hash)]
#[repr(transparent)]
pub struct MetadataKey {
    // Note: There are unsafe transmutes that assume that the memory layout
    // of MetadataValue is identical to HeaderName
    pub(crate) inner: http::header::HeaderName,
}

/// A possible error when converting a `MetadataKey` from another type.
#[derive(Debug)]
pub struct InvalidMetadataKey {
    _priv: (),
}

impl MetadataKey {
    /// Converts a slice of bytes to a `MetadataKey`.
    ///
    /// This function normalizes the input.
    pub fn from_bytes(src: &[u8]) -> Result<MetadataKey, InvalidMetadataKey> {
        match HeaderName::from_bytes(src) {
            Ok(name) => Ok(MetadataKey { inner: name }),
            Err(_) => Err(InvalidMetadataKey::new())
        }
    }

    /// Converts a static string to a `MetadataKey`.
    ///
    /// This function panics when the static string is a invalid metadata key.
    /// 
    /// This function requires the static string to only contain lowercase 
    /// characters, numerals and symbols, as per the HTTP/2.0 specification 
    /// and header names internal representation within this library.
    /// 
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// // Parsing a metadata key
    /// let CUSTOM_KEY: &'static str = "custom-key";
    /// 
    /// let a = MetadataKey::from_bytes(b"custom-key").unwrap();
    /// let b = MetadataKey::from_static(CUSTOM_KEY);
    /// assert_eq!(a, b);
    /// ```
    /// 
    /// ```should_panic
    /// # use tower_grpc::metadata::*;
    /// #
    /// // Parsing a metadata key that contains invalid symbols(s):
    /// MetadataKey::from_static("content{}{}length"); // This line panics!
    /// 
    /// // Parsing a metadata key that contains invalid uppercase characters.
    /// let a = MetadataKey::from_static("foobar");
    /// let b = MetadataKey::from_static("FOOBAR"); // This line panics!
    /// ```
    pub fn from_static(src: &'static str) -> MetadataKey {
        MetadataKey { inner: HeaderName::from_static(src) }
    }

    /// Returns a `str` representation of the metadata key.
    ///
    /// The returned string will always be lower case.
    #[inline]
    pub fn as_str(&self) -> &str {
        self.inner.as_str()
    }

    #[inline]
    pub(crate) fn from_header_name(header_name: &HeaderName) -> &Self {
        unsafe { &*(header_name as *const HeaderName as *const Self) }
    }
}

impl FromStr for MetadataKey {
    type Err = InvalidMetadataKey;

    fn from_str(s: &str) -> Result<MetadataKey, InvalidMetadataKey> {
        MetadataKey::from_bytes(s.as_bytes())
            .map_err(|_| InvalidMetadataKey::new())
    }
}

impl AsRef<str> for MetadataKey {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}

impl AsRef<[u8]> for MetadataKey {
    fn as_ref(&self) -> &[u8] {
        self.as_str().as_bytes()
    }
}

impl Borrow<str> for MetadataKey {
    fn borrow(&self) -> &str {
        self.as_str()
    }
}

impl fmt::Debug for MetadataKey {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(self.as_str(), fmt)
    }
}

impl fmt::Display for MetadataKey {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt::Display::fmt(self.as_str(), fmt)
    }
}

impl InvalidMetadataKey {
    pub fn new() -> InvalidMetadataKey {
        InvalidMetadataKey { _priv: () }
    }
}

impl<'a> From<&'a MetadataKey> for MetadataKey {
    fn from(src: &'a MetadataKey) -> MetadataKey {
        src.clone()
    }
}

impl From<MetadataKey> for Bytes {
    #[inline]
    fn from(name: MetadataKey) -> Bytes {
        name.inner.into()
    }
}

impl<'a> PartialEq<&'a MetadataKey> for MetadataKey {
    #[inline]
    fn eq(&self, other: &&'a MetadataKey) -> bool {
        *self == **other
    }
}


impl<'a> PartialEq<MetadataKey> for &'a MetadataKey {
    #[inline]
    fn eq(&self, other: &MetadataKey) -> bool {
        *other == *self
    }
}

impl PartialEq<str> for MetadataKey {
    /// Performs a case-insensitive comparison of the string against the header
    /// name
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let content_length = MetadataKey::from_static("content-length");
    ///
    /// assert_eq!(content_length, "content-length");
    /// assert_eq!(content_length, "Content-Length");
    /// assert_ne!(content_length, "content length");
    /// ```
    #[inline]
    fn eq(&self, other: &str) -> bool {
        self.inner.eq(other)
    }
}


impl PartialEq<MetadataKey> for str {
    /// Performs a case-insensitive comparison of the string against the header
    /// name
    ///
    /// # Examples
    ///
    /// ```
    /// # use tower_grpc::metadata::*;
    /// let content_length = MetadataKey::from_static("content-length");
    ///
    /// assert_eq!(content_length, "content-length");
    /// assert_eq!(content_length, "Content-Length");
    /// assert_ne!(content_length, "content length");
    /// ```
    #[inline]
    fn eq(&self, other: &MetadataKey) -> bool {
        (*other).inner == *self
    }
}

impl<'a> PartialEq<&'a str> for MetadataKey {
    /// Performs a case-insensitive comparison of the string against the header
    /// name
    #[inline]
    fn eq(&self, other: &&'a str) -> bool {
        *self == **other
    }
}


impl<'a> PartialEq<MetadataKey> for &'a str {
    /// Performs a case-insensitive comparison of the string against the header
    /// name
    #[inline]
    fn eq(&self, other: &MetadataKey) -> bool {
        *other == *self
    }
}

impl fmt::Display for InvalidMetadataKey {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.description().fmt(f)
    }
}

impl Error for InvalidMetadataKey {
    fn description(&self) -> &str {
        "invalid gRPC metadata key name"
    }
}
