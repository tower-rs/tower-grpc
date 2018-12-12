use h2;
use std;
use std::fmt;

#[derive(Debug)]
pub enum Error<T = ()> {
    Grpc(::Status),
    Inner(T),
}

impl<T> fmt::Display for Error<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Grpc(ref status) => {
                write!(
                    f,
                    "grpc-status: {:?}, grpc-message: {:?}",
                    status.code(),
                    status.error_message()
                )
            },
            Error::Inner(ref _inner) =>
                f.pad("inner error"),
        }
    }
}

impl<T> std::error::Error for Error<T> where T : fmt::Debug {
    fn cause(&self) -> Option<&std::error::Error> {
        match *self {
            Error::Grpc(_) => None,
            Error::Inner(ref _inner) => None,
        }
    }
}

impl<T> From<T> for Error<T> {
    fn from(inner: T) -> Self {
        Error::Inner(inner)
    }
}

impl From<Error<()>> for h2::Error {
    fn from(_err: Error<()>) -> Self {
        // TODO: implement
        h2::Reason::INTERNAL_ERROR.into()
    }
}

impl From<h2::Error> for Error<()> {
    fn from(err: h2::Error) -> Self {
        let status = err.into();
        Error::Grpc(status)
    }
}
