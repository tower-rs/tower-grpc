use std::hash::Hash;

// TODO(pgron): Make sealed
pub trait ValueEncoding: Clone + Eq + PartialEq + Hash {
    #[doc(hidden)]
    fn is_valid_key(key: &str) -> bool;
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Ascii {}
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct Binary {}

impl ValueEncoding for Ascii {
    fn is_valid_key(key: &str) -> bool {
        !Binary::is_valid_key(key)
    }
}

impl ValueEncoding for Binary {
    fn is_valid_key(key: &str) -> bool {
        key.ends_with("-bin")
    }
}
