use crate::pb;

use std::{default, iter};

pub fn client_payload(size: usize) -> pb::Payload {
    pb::Payload {
        r#type: default::Default::default(),
        body: iter::repeat(0u8).take(size).collect(),
    }
}
