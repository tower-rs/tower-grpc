use crate::body::{Body, HttpBody};
use crate::error::Error;

use futures::{Future, Poll};
use http::{Request, Response};
use tower_service::Service;

/// A specialization of tower_service::Service.
///
/// Existing tower_service::Service implementations with the correct form will
/// automatically implement `GrpcService`.
pub trait GrpcService<ReqBody> {
    /// Response body type
    type ResponseBody: Body + HttpBody;

    /// Response future
    type Future: Future<Item = Response<Self::ResponseBody>, Error = Self::Error>;

    /// Error type
    type Error: Into<Error>;

    /// Poll that this service is ready.
    fn poll_ready(&mut self) -> Poll<(), Self::Error>;

    /// Call the service.
    fn call(&mut self, request: Request<ReqBody>) -> Self::Future;

    /// Helper when needing to pass this type to bounds needing `Service`.
    fn into_service(self) -> IntoService<Self>
    where
        Self: Sized,
    {
        IntoService(self)
    }

    /// Helper when needing to pass this type to bounds needing `Service`.
    fn as_service(&mut self) -> AsService<'_, Self>
    where
        Self: Sized,
    {
        AsService(self)
    }
}

impl<T, ReqBody, ResBody> GrpcService<ReqBody> for T
where
    T: Service<Request<ReqBody>, Response = Response<ResBody>>,
    T::Error: Into<Error>,
    ResBody: Body + HttpBody,
{
    type ResponseBody = ResBody;
    type Future = T::Future;
    type Error = T::Error;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Service::poll_ready(self)
    }

    fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
        Service::call(self, request)
    }
}

/// Helper when needing to pass a `GrpcService` to bounds needing `Service`.
#[derive(Debug)]
pub struct AsService<'a, T>(&'a mut T);

impl<'a, T, ReqBody> Service<Request<ReqBody>> for AsService<'a, T>
where
    T: GrpcService<ReqBody> + 'a,
{
    type Response = Response<T::ResponseBody>;
    type Future = T::Future;
    type Error = T::Error;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        GrpcService::poll_ready(self.0)
    }

    fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
        GrpcService::call(self.0, request)
    }
}

/// Helper when needing to pass a `GrpcService` to bounds needing `Service`.
#[derive(Debug)]
pub struct IntoService<T>(pub(crate) T);

impl<T, ReqBody> Service<Request<ReqBody>> for IntoService<T>
where
    T: GrpcService<ReqBody>,
{
    type Response = Response<T::ResponseBody>;
    type Future = T::Future;
    type Error = T::Error;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        GrpcService::poll_ready(&mut self.0)
    }

    fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
        GrpcService::call(&mut self.0, request)
    }
}
