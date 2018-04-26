mod grpc;

pub mod client_streaming;
pub mod server_streaming;
pub mod streaming;
pub mod unary;

pub use self::grpc::Grpc;

use {Request, Response};

use futures::{Future, Stream};
use tower_service::Service;

/// A specialization of tower_service::Service.
///
/// Existing tower_service::Service implementations with the correct form will
/// automatically implement `GrpcService`.
pub trait StreamingService {
    /// Protobuf request message type
    type Request;

    /// Stream of inbound request messages
    type RequestStream: Stream<Item = Self::Request, Error = ::Error>;

    /// Protobuf response message type
    type Response;

    /// Stream of outbound response messages
    type ResponseStream: Stream<Item = Self::Response, Error = ::Error>;

    /// Response future
    type Future: Future<Item = ::Response<Self::ResponseStream>, Error = ::Error>;

    /// Call the service
    fn call(&mut self, request: Request<Self::RequestStream>) -> Self::Future;
}

impl<T, S1, S2> StreamingService for T
where T: Service<Request = Request<S1>,
                Response = Response<S2>,
                   Error = ::Error>,
      S1: Stream<Error = ::Error>,
      S2: Stream<Error = ::Error>,
{
    type Request = S1::Item;
    type RequestStream = S1;
    type Response = S2::Item;
    type ResponseStream = S2;
    type Future = T::Future;

    fn call(&mut self, request: T::Request) -> Self::Future {
        Service::call(self, request)
    }
}

/// A specialization of tower_service::Service.
///
/// Existing tower_service::Service implementations with the correct form will
/// automatically implement `UnaryService`.
pub trait UnaryService {
    /// Protobuf request message type
    type Request;

    /// Protobuf response message type
    type Response;

    /// Response future
    type Future: Future<Item = ::Response<Self::Response>, Error = ::Error>;

    /// Call the service
    fn call(&mut self, request: Request<Self::Request>) -> Self::Future;
}

impl<T, M1, M2> UnaryService for T
where T: Service<Request = Request<M1>,
                Response = Response<M2>,
                   Error = ::Error>,
{
    type Request = M1;
    type Response = M2;
    type Future = T::Future;

    fn call(&mut self, request: T::Request) -> Self::Future {
        Service::call(self, request)
    }
}

/// A specialization of tower_service::Service.
///
/// Existing tower_service::Service implementations with the correct form will
/// automatically implement `UnaryService`.
pub trait ClientStreamingService {
    /// Protobuf request message type
    type Request;

    /// Stream of inbound request messages
    type RequestStream: Stream<Item = Self::Request, Error = ::Error>;

    /// Protobuf response message type
    type Response;

    /// Response future
    type Future: Future<Item = ::Response<Self::Response>, Error = ::Error>;

    /// Call the service
    fn call(&mut self, request: Request<Self::RequestStream>) -> Self::Future;
}

impl<T, M, S> ClientStreamingService for T
where T: Service<Request = Request<S>,
                Response = Response<M>,
                   Error = ::Error>,
      S: Stream<Error = ::Error>,
{
    type Request = S::Item;
    type RequestStream = S;
    type Response = M;
    type Future = T::Future;

    fn call(&mut self, request: T::Request) -> Self::Future {
        Service::call(self, request)
    }
}

/// A specialization of tower_service::Service.
///
/// Existing tower_service::Service implementations with the correct form will
/// automatically implement `UnaryService`.
pub trait ServerStreamingService {
    /// Protobuf request message type
    type Request;

    /// Protobuf response message type
    type Response;

    /// Stream of outbound response messages
    type ResponseStream: Stream<Item = Self::Response, Error = ::Error>;

    /// Response future
    type Future: Future<Item = ::Response<Self::ResponseStream>, Error = ::Error>;

    /// Call the service
    fn call(&mut self, request: Request<Self::Request>) -> Self::Future;
}

impl<T, M, S> ServerStreamingService for T
where T: Service<Request = Request<M>,
                Response = Response<S>,
                   Error = ::Error>,
      S: Stream<Error = ::Error>,
{
    type Request = M;
    type Response = S::Item;
    type ResponseStream = S;
    type Future = T::Future;

    fn call(&mut self, request: T::Request) -> Self::Future {
        Service::call(self, request)
    }
}
