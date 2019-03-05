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
extern crate tower_http_service;
extern crate tower_service;
extern crate tower_util;

#[cfg(feature = "tower-h2")]
extern crate tower_h2;
#[cfg(feature = "protobuf")]
extern crate prost;

pub mod client;
pub mod generic;
pub mod metadata;

mod body;
mod request;
mod response;
mod status;

pub use body::{Body, BoxBody};
pub use status::{Code, Status};
pub use request::Request;
pub use response::Response;

#[cfg(feature = "protobuf")]
pub mod server;

/// Type re-exports used by generated code
#[cfg(feature = "protobuf")]
pub mod codegen;

#[cfg(feature = "protobuf")]
mod codec;

#[cfg(feature = "protobuf")]
pub use codec::{Encode, Streaming};

