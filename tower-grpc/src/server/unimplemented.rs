use crate::{Code, Status};

use futures::{Future, Poll};
use http::header;

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
    type Error = crate::error::Never;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let status = self.status.take().expect("polled after complete");

        // Construct http response
        let mut response = http::Response::new(());

        // Set the content type
        // As the rpc is unimplemented we don't care about
        // specifying the encoding (+proto, +json, +...)
        // so we can just return a dummy "application/grpc"
        response.headers_mut().insert(
            header::CONTENT_TYPE,
            header::HeaderValue::from_static("application/grpc"),
        );

        status
            .add_header(response.headers_mut())
            .expect("generated unimplemented message should be valid");
        Ok(response.into())
    }
}
