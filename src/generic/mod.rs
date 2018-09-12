//! gRPC generic over encoder / decoder.

pub mod server;

mod codec;

pub use self::codec::{
    Codec,
    Encoder,
    Decoder,
    Direction,
    Streaming,
    Encode,
    EncodeBuf,
    DecodeBuf,
};
