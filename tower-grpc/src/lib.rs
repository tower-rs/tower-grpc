#![deny(warnings, missing_debug_implementations, rust_2018_idioms)]

pub mod client;
pub mod generic;
pub mod metadata;

mod body;
mod error;
mod request;
mod response;
mod status;

pub use crate::body::{Body, BoxBody};
pub use crate::request::Request;
pub use crate::response::Response;
pub use crate::status::{Code, Status};

#[cfg(feature = "protobuf")]
pub mod server;

/// Type re-exports used by generated code
#[cfg(feature = "protobuf")]
pub mod codegen;

#[cfg(feature = "protobuf")]
mod codec;

#[cfg(feature = "protobuf")]
pub use crate::codec::{Encode, Streaming};
