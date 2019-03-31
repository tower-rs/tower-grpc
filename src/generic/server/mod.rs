mod grpc;

pub(crate) mod client_streaming;
pub(crate) mod server_streaming;
pub(crate) mod streaming;
pub(crate) mod unary;

pub(crate) use self::grpc::Grpc;

use {Request, Response};

use futures::{Future, Stream};
use tower_service::Service;

/// A specialization of tower_service::Service.
///
/// Existing tower_service::Service implementations with the correct form will
/// automatically implement `GrpcService`.
pub trait StreamingService<RequestStream> {
    /// Protobuf response message type
    type Response;

    /// Stream of outbound response messages
    type ResponseStream: Stream<Item = Self::Response, Error = ::Status>;

    /// Response future
    type Future: Future<Item = ::Response<Self::ResponseStream>, Error = ::Status>;

    /// Call the service
    fn call(&mut self, request: Request<RequestStream>) -> Self::Future;
}

impl<T, S1, S2> StreamingService<S1> for T
where
    T: Service<Request<S1>, Response = Response<S2>, Error = ::Status>,
    S1: Stream<Error = ::Status>,
    S2: Stream<Error = ::Status>,
{
    type Response = S2::Item;
    type ResponseStream = S2;
    type Future = T::Future;

    fn call(&mut self, request: Request<S1>) -> Self::Future {
        Service::call(self, request)
    }
}

/// A specialization of tower_service::Service.
///
/// Existing tower_service::Service implementations with the correct form will
/// automatically implement `UnaryService`.
pub trait UnaryService<R> {
    /// Protobuf response message type
    type Response;

    /// Response future
    type Future: Future<Item = ::Response<Self::Response>, Error = ::Status>;

    /// Call the service
    fn call(&mut self, request: Request<R>) -> Self::Future;
}

impl<T, M1, M2> UnaryService<M1> for T
where
    T: Service<Request<M1>, Response = Response<M2>, Error = ::Status>,
{
    type Response = M2;
    type Future = T::Future;

    fn call(&mut self, request: Request<M1>) -> Self::Future {
        Service::call(self, request)
    }
}

/// A specialization of tower_service::Service.
///
/// Existing tower_service::Service implementations with the correct form will
/// automatically implement `UnaryService`.
pub trait ClientStreamingService<RequestStream> {
    /// Protobuf response message type
    type Response;

    /// Response future
    type Future: Future<Item = ::Response<Self::Response>, Error = ::Status>;

    /// Call the service
    fn call(&mut self, request: Request<RequestStream>) -> Self::Future;
}

impl<T, M, S> ClientStreamingService<S> for T
where
    T: Service<Request<S>, Response = Response<M>, Error = ::Status>,
    S: Stream<Error = ::Status>,
{
    type Response = M;
    type Future = T::Future;

    fn call(&mut self, request: Request<S>) -> Self::Future {
        Service::call(self, request)
    }
}

/// A specialization of tower_service::Service.
///
/// Existing tower_service::Service implementations with the correct form will
/// automatically implement `UnaryService`.
pub trait ServerStreamingService<R> {
    /// Protobuf response message type
    type Response;

    /// Stream of outbound response messages
    type ResponseStream: Stream<Item = Self::Response, Error = ::Status>;

    /// Response future
    type Future: Future<Item = ::Response<Self::ResponseStream>, Error = ::Status>;

    /// Call the service
    fn call(&mut self, request: Request<R>) -> Self::Future;
}

impl<T, M, S> ServerStreamingService<M> for T
where
    T: Service<Request<M>, Response = Response<S>, Error = ::Status>,
    S: Stream<Error = ::Status>,
{
    type Response = S::Item;
    type ResponseStream = S;
    type Future = T::Future;

    fn call(&mut self, request: Request<M>) -> Self::Future {
        Service::call(self, request)
    }
}
