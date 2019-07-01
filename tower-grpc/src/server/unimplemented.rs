use crate::{Code, Status};

use http::header;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

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
    type Output = Result<http::Response<()>, crate::error::Never>;

    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Self::Output> {
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
        Ok(response).into()
    }
}
