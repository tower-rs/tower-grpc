use bytes::Bytes;
use std::hash::Hash;

// TODO(pgron): Make sealed
pub trait ValueEncoding: Clone + Eq + PartialEq + Hash {
    #[doc(hidden)]
    fn is_valid_key(key: &str) -> bool;

    #[doc(hidden)]
    fn is_empty(value: &[u8]) -> bool;

    #[doc(hidden)]
    fn encode(value: Bytes) -> Bytes;
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Ascii {}
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Binary {}

impl ValueEncoding for Ascii {
    fn is_valid_key(key: &str) -> bool {
        !Binary::is_valid_key(key)
    }

    fn is_empty(value: &[u8]) -> bool {
        value.is_empty()
    }

    fn encode(value: Bytes) -> Bytes {
       value
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

    fn encode(value: Bytes) -> Bytes {
        // TODO(pgron): Do this properly for base64
        value
    }
}
