#![deny(warnings, missing_debug_implementations)]
//#![deny(missing_docs)]

extern crate bytes;
#[macro_use]
extern crate futures;
extern crate http;
extern crate h2;
#[macro_use]
extern crate log;
extern crate tower_http;
extern crate tower_service;

#[cfg(feature = "tower-h2")]
extern crate tower_h2;
#[cfg(feature = "protobuf")]
extern crate prost;

pub mod client;
pub mod generic;

mod body;
mod error;
mod metadata_key;
mod metadata_value;
mod metadata_map;
mod request;
mod response;
mod status;

pub use body::{Body, BoxBody};
pub use error::{Error, ProtocolError};
pub use status::{Code, Status};
pub use request::Request;
pub use response::Response;

pub mod metadata {
    pub use metadata_key::MetadataKey;
    pub use metadata_key::InvalidMetadataKey;
    pub use metadata_value::MetadataValue;
    pub use metadata_value::InvalidMetadataValue;
    pub use metadata_value::InvalidMetadataValueBytes;
    pub use metadata_value::ToStrError;
    pub use metadata_map::MetadataMap;
    pub use metadata_map::Iter;
    pub use metadata_map::ValueDrain;
    pub use metadata_map::Drain;
    pub use metadata_map::Keys;
    pub use metadata_map::Values;
    pub use metadata_map::ValueIter;
    pub use metadata_map::GetAll;
    pub use metadata_map::Entry;
    pub use metadata_map::VacantEntry;
    pub use metadata_map::OccupiedEntry;
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

