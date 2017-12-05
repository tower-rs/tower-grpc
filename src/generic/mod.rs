//! gRPC generic over encoder / decoder.

pub mod server;

mod codec;

pub use self::codec::{
    Codec,
    Encoder,
    Decoder,
    Streaming,
    Encode,
    EncodeBuf,
    DecodeBuf,
};
