#![deny(warnings, missing_debug_implementations)]
//#![deny(missing_docs)]

extern crate base64;
extern crate bytes;
#[macro_use]
extern crate futures;
extern crate http;
extern crate h2;
#[macro_use]
extern crate log;
extern crate percent_encoding;
extern crate tower_http;
extern crate tower_service;
extern crate tower_util;

#[cfg(feature = "tower-h2")]
extern crate tower_h2;
#[cfg(feature = "protobuf")]
extern crate prost;

pub mod client;
pub mod generic;

mod body;
mod metadata_encoding;
mod metadata_key;
mod metadata_value;
mod metadata_map;
mod request;
mod response;
mod status;

pub use body::{Body, BoxBody};
pub use status::{Code, Status};
pub use request::Request;
pub use response::Response;

/// The metadata module contains data structures and utilities for handling
/// gRPC custom metadata.
pub mod metadata {
    pub use metadata_encoding::Ascii;
    pub use metadata_encoding::Binary;
    pub use metadata_key::AsciiMetadataKey;
    pub use metadata_key::BinaryMetadataKey;
    pub use metadata_key::MetadataKey;
    pub use metadata_value::AsciiMetadataValue;
    pub use metadata_value::BinaryMetadataValue;
    pub use metadata_value::MetadataValue;
    pub use metadata_map::MetadataMap;
    pub use metadata_map::Iter;
    pub use metadata_map::ValueDrain;
    pub use metadata_map::Keys;
    pub use metadata_map::KeyRef;
    pub use metadata_map::KeyAndValueRef;
    pub use metadata_map::KeyAndMutValueRef;
    pub use metadata_map::Values;
    pub use metadata_map::ValueRef;
    pub use metadata_map::ValueRefMut;
    pub use metadata_map::ValueIter;
    pub use metadata_map::GetAll;
    pub use metadata_map::Entry;
    pub use metadata_map::VacantEntry;
    pub use metadata_map::OccupiedEntry;

    /// The metadata::errors module contains types for errors that can occur
    /// while handling gRPC custom metadata.
    pub mod errors {
        pub use metadata_key::InvalidMetadataKey;
        pub use metadata_encoding::InvalidMetadataValue;
        pub use metadata_encoding::InvalidMetadataValueBytes;
        pub use metadata_value::ToStrError;
    }
}

#[cfg(feature = "protobuf")]
pub mod server;

/// Type re-exports used by generated code
#[cfg(feature = "protobuf")]
pub mod codegen;

#[cfg(feature = "protobuf")]
mod codec;

#[cfg(feature = "protobuf")]
pub use codec::{Encode, Streaming};

