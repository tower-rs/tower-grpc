//! gRPC generic over encoder / decoder.

pub mod client;
pub mod server;

mod codec;

pub(crate) use self::codec::{
    Direction,
};

pub use self::codec::{
    Codec,
    Encoder,
    Decoder,
    Streaming,
    Encode,
    EncodeBuf,
    DecodeBuf,
};
