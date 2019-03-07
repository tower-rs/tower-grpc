use futures::{Future, Poll};
use http::{Request, Response};
use tower_http_service::HttpService;

use body::{Body, HttpBody};

type Error = Box<dyn std::error::Error + Send + Sync>;

/// A specialization of tower_service::Service.
///
/// Existing tower_service::Service implementations with the correct form will
/// automatically implement `GrpcService`.
pub trait GrpcService<ReqBody> {

    type ResponseBody: Body;

    /// Response future
    type Future: Future<Item = Response<Self::ResponseBody>>;

    type Error: Into<Error>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error>;

    /// Call the service
    fn call(&mut self, request: Request<ReqBody>) -> Self::Future;
}

impl<T, ReqBody, ResBody> GrpcService<ReqBody> for T
where
    T: HttpService<ReqBody, ResponseBody = ResBody>,
    T::Error: Into<Error>,
    ResBody: Body + HttpBody,
{
    type ResponseBody = ResBody;
    type Future = T::Future;
    type Error = T::Error;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        HttpService::poll_ready(self)
    }

    fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
        HttpService::call(self, request)
    }
}
