use futures::{Future, Poll};
use {h2, http};

use {Code, Status};

#[derive(Debug)]
pub struct ResponseFuture {
    status: Option<Status>,
}

impl ResponseFuture {
    pub(crate) fn new(msg: String) -> Self {
        ResponseFuture {
            status: Some(Status::new(Code::Unimplemented, msg)),
        }
    }
}

impl Future for ResponseFuture {
    type Item = http::Response<()>;
    type Error = h2::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let status = self.status.take().expect("polled after complete");

        let mut resp = http::Response::new(());
        status.add_header(resp.headers_mut())?;
        Ok(resp.into())
    }
}
