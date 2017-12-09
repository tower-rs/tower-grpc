use bytes::Bytes;
use tower_h2::Body;

use std::fmt;

pub struct BoxBody {
    inner: Box<Body<Data = Bytes> + Send + 'static>,
}

pub struct UnsyncBoxBody {
    inner: Box<Body<Data = Bytes> + 'static>,
}

impl fmt::Debug for BoxBody {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("BoxBody")
            .finish()
    }
}

impl fmt::Debug for UnsyncBoxBody {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("UnsyncBoxBody")
            .finish()
    }
}
