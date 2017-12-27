use ::pb;

use std::{default, iter};

pub fn client_payload(type_: pb::PayloadType, size: usize) -> pb::Payload {
    pb::Payload { 
        type_: type_ as i32,
        body: iter::repeat(0u8).take(size).collect(),
    }
}