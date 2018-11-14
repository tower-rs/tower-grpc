use std::fmt;

use h2;
use http::header::{HeaderMap, HeaderValue};
use http::status::StatusCode;

#[derive(Debug, Clone)]
pub struct Status {
    code: Code,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Code(Code_);

impl Status {
    #[inline]
    pub fn code(&self) -> Code {
        self.code
    }

    pub const OK: Status = Status {
        code: Code(Code_::Ok),
    };

    #[deprecated(note = "use Status::CANCELLED")]
    pub const CANCELED: Status = Status::CANCELLED;

    pub const CANCELLED: Status = Status {
        code: Code(Code_::Cancelled),
    };

    pub const UNKNOWN: Status = Status {
        code: Code(Code_::Unknown),
    };

    pub const INVALID_ARGUMENT: Status = Status {
        code: Code(Code_::InvalidArgument),
    };

    pub const DEADLINE_EXCEEDED: Status = Status {
        code: Code(Code_::DeadlineExceeded),
    };

    pub const NOT_FOUND: Status = Status {
        code: Code(Code_::NotFound),
    };

    pub const ALREADY_EXISTS: Status = Status {
        code: Code(Code_::AlreadyExists),
    };

    pub const PERMISSION_DENIED: Status = Status {
        code: Code(Code_::PermissionDenied),
    };

    pub const RESOURCE_EXHAUSTED: Status = Status {
        code: Code(Code_::ResourceExhausted),
    };

    pub const FAILED_PRECONDITION: Status = Status {
        code: Code(Code_::FailedPrecondition),
    };

    pub const ABORTED: Status = Status {
        code: Code(Code_::Aborted),
    };

    pub const OUT_OF_RANGE: Status = Status {
        code: Code(Code_::OutOfRange),
    };

    pub const UNIMPLEMENTED: Status = Status {
        code: Code(Code_::Unimplemented),
    };

    pub const INTERNAL: Status = Status {
        code: Code(Code_::Internal),
    };

    pub const UNAVAILABLE: Status = Status {
        code: Code(Code_::Unavailable),
    };

    pub const DATA_LOSS: Status = Status {
        code: Code(Code_::DataLoss),
    };

    pub const UNAUTHENTICATED: Status = Status {
        code: Code(Code_::Unauthenticated),
    };

    pub(crate) fn from_bytes(bytes: &[u8]) -> Status {
        let code = match bytes.len() {
            1 => {
                match bytes[0] {
                    b'0' => Code_::Ok,
                    b'1' => Code_::Cancelled,
                    b'2' => Code_::Unknown,
                    b'3' => Code_::InvalidArgument,
                    b'4' => Code_::DeadlineExceeded,
                    b'5' => Code_::NotFound,
                    b'6' => Code_::AlreadyExists,
                    b'7' => Code_::PermissionDenied,
                    b'8' => Code_::ResourceExhausted,
                    b'9' => Code_::FailedPrecondition,
                    _ => return Status::parse_err(),
                }
            },
            2 => {
                match (bytes[0], bytes[1]) {
                    (b'1', b'0') => Code_::Aborted,
                    (b'1', b'1') => Code_::OutOfRange,
                    (b'1', b'2') => Code_::Unimplemented,
                    (b'1', b'3') => Code_::Internal,
                    (b'1', b'4') => Code_::Unavailable,
                    (b'1', b'5') => Code_::DataLoss,
                    (b'1', b'6') => Code_::Unauthenticated,
                    _ => return Status::parse_err(),
                }
            },
            _ => return Status::parse_err(),
        };

        Status::new(Code(code))
    }

    // TODO: It would be nice for this not to be public
    pub fn to_header_value(&self) -> HeaderValue {
        match self.code.0 {
            Code_::Ok => HeaderValue::from_static("0"),
            Code_::Cancelled => HeaderValue::from_static("1"),
            Code_::Unknown => HeaderValue::from_static("2"),
            Code_::InvalidArgument => HeaderValue::from_static("3"),
            Code_::DeadlineExceeded => HeaderValue::from_static("4"),
            Code_::NotFound => HeaderValue::from_static("5"),
            Code_::AlreadyExists => HeaderValue::from_static("6"),
            Code_::PermissionDenied => HeaderValue::from_static("7"),
            Code_::ResourceExhausted => HeaderValue::from_static("8"),
            Code_::FailedPrecondition => HeaderValue::from_static("9"),
            Code_::Aborted => HeaderValue::from_static("10"),
            Code_::OutOfRange => HeaderValue::from_static("11"),
            Code_::Unimplemented => HeaderValue::from_static("12"),
            Code_::Internal => HeaderValue::from_static("13"),
            Code_::Unavailable => HeaderValue::from_static("14"),
            Code_::DataLoss => HeaderValue::from_static("15"),
            Code_::Unauthenticated => HeaderValue::from_static("16"),
        }
    }

    fn new(code: Code) -> Status {
        Status {
            code,
        }
    }

    fn parse_err() -> Status {
        trace!("error parsing grpc-status");
        Status::UNKNOWN
    }
}

impl From<h2::Error> for Status {
    fn from(err: h2::Error) -> Self {
        // See https://github.com/grpc/grpc/blob/3977c30/doc/PROTOCOL-HTTP2.md#errors
        match err.reason() {
            Some(h2::Reason::NO_ERROR) |
            Some(h2::Reason::PROTOCOL_ERROR) |
            Some(h2::Reason::INTERNAL_ERROR) |
            Some(h2::Reason::FLOW_CONTROL_ERROR) |
            Some(h2::Reason::SETTINGS_TIMEOUT) |
            Some(h2::Reason::COMPRESSION_ERROR) |
            Some(h2::Reason::CONNECT_ERROR) => Status::INTERNAL,
            Some(h2::Reason::REFUSED_STREAM) => Status::UNAVAILABLE,
            Some(h2::Reason::CANCEL) => Status::CANCELLED,
            Some(h2::Reason::ENHANCE_YOUR_CALM) => Status::RESOURCE_EXHAUSTED,
            Some(h2::Reason::INADEQUATE_SECURITY) => Status::PERMISSION_DENIED,

            _ => Status::UNKNOWN,
        }
    }
}

impl From<Status> for h2::Error {
    fn from(_status: Status) -> Self {
        // TODO: implement
        h2::Reason::INTERNAL_ERROR.into()
    }
}

impl Code {
    pub const OK: Code = Code(Code_::Ok);
    //TODO: the rest...
}

impl fmt::Debug for Code {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Code_ {
    Ok = 0,
    Cancelled = 1,
    Unknown = 2,
    InvalidArgument = 3,
    DeadlineExceeded = 4,
    NotFound = 5,
    AlreadyExists = 6,
    PermissionDenied = 7,
    ResourceExhausted = 8,
    FailedPrecondition = 9,
    Aborted = 10,
    OutOfRange = 11,
    Unimplemented = 12,
    Internal = 13,
    Unavailable = 14,
    DataLoss = 15,
    Unauthenticated = 16,
}

/// Take the `Status` value from `trailers` if it is available. if it is not, and `status_code` is
/// provided, infer the `Status` from the `status_code`. Otherwise, return `None`.
pub fn infer_grpc_status(trailers: &HeaderMap, status_code: Option<StatusCode>) -> Option<Status> {
    trailers.get("grpc-status").map(|s| {
        Status::from_bytes(s.as_ref())
    }).or_else(|| {
        status_code.map(|status_code| {
            match status_code {
                // Borrowed from https://github.com/grpc/grpc/blob/master/doc/http-grpc-status-mapping.md
                StatusCode::BAD_REQUEST => Status::INTERNAL,
                StatusCode::UNAUTHORIZED => Status::UNAUTHENTICATED,
                StatusCode::FORBIDDEN => Status::PERMISSION_DENIED,
                StatusCode::NOT_FOUND => Status::UNIMPLEMENTED,
                StatusCode::TOO_MANY_REQUESTS |
                    StatusCode::BAD_GATEWAY |
                    StatusCode::SERVICE_UNAVAILABLE |
                    StatusCode::GATEWAY_TIMEOUT => Status::UNAVAILABLE,
                _ => Status::UNKNOWN,
            }
        })
    })
}
