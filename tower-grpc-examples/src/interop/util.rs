use ::pb;

use std::{default, iter};

pub fn client_payload(size: usize) -> pb::Payload {
    pb::Payload { 
        type_: pb::PayloadType::Compressable as i32,
        body: iter::repeat(0u8).take(size).collect(),
    }
}