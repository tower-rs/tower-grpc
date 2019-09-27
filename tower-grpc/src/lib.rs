#![doc(html_root_url = "https://docs.rs/tower-grpc/0.1.1")]
#![deny(missing_debug_implementations, rust_2018_idioms)]
// TODO: enable when there actually are docs
// #![deny(missing_docs)]
#![cfg_attr(test, deny(warnings))]

//! gRPC client and server implementation based on Tower.

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
