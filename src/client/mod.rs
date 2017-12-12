#![allow(warnings, missing_debug_implementations)]

use Status;
use util::BoxBody;

use futures::{stream, Future, Stream, Poll};
use http::{uri, Request, Response, HeaderMap, Uri};
use prost::Message;
use tower::Service;
use tower_h2::Body;

use std::marker::PhantomData;

#[derive(Debug)]
pub struct Grpc<T> {
    /// The inner HTTP/2.0 service.
    inner: T,

    /// The service's scheme.
    scheme: uri::Scheme,

    /// The service's authority.
    authority: uri::Authority,
}

#[derive(Debug, Default)]
pub struct Builder {
    /// The service's URI
    uri: Option<uri::Uri>,
}

/// An HTTP (2.0) service that backs the gRPC client
pub trait HttpService {
    type RequestBody: Body;
    type ResponseBody: Body;
    type Error;
    type Future: Future<Item = Response<Self::ResponseBody>, Error = Self::Error>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error>;

    fn call(&mut self, request: Request<Self::RequestBody>) -> Self::Future;
}

impl<T, B1, B2> HttpService for T
where T: Service<Request = Request<B1>,
                Response = Response<B2>>,
      B1: Body,
      B2: Body,
{
    type RequestBody = B1;
    type ResponseBody = B2;
    type Error = T::Error;
    type Future = T::Future;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        Service::poll_ready(self)
    }

    fn call(&mut self, request: Request<Self::RequestBody>) -> Self::Future {
        Service::call(self, request)
    }
}

/// Convert a stream of protobuf messages to an HTTP body payload.
pub trait IntoBody<T>
{
    fn into_body(self) -> T;
}

impl<T, U> IntoBody<BoxBody> for T
where T: Stream<Item = U, Error = ::Error> + Send + 'static,
      U: Message + 'static,
{
    fn into_body(self) -> BoxBody {
        use codec::Encoder;
        use generic::Encode;

        let encode = Encode::new(Encoder::new(), self);
        BoxBody::new(Box::new(encode))
    }
}

pub type Once<T> = stream::Once<T, ::Error>;

impl<T> Grpc<T>
where T: HttpService,
{
    pub fn poll_ready(&mut self) -> Poll<(), ::Error<T::Error>> {
        self.inner.poll_ready()
            .map_err(::Error::Inner)
    }

    pub fn unary<M1, M2>(&mut self,
                         request: ::Request<M1>,
                         path: uri::PathAndQuery)
        -> unary::ResponseFuture<M2, T::Future, T::ResponseBody>
    where Once<M1>: IntoBody<T::RequestBody>,
    {
        let request = request.map(|v| stream::once(Ok(v)));
        let response = self.client_streaming(request, path);

        unary::ResponseFuture::new(response)
    }

    pub fn client_streaming<B, M>(&mut self,
                                  request: ::Request<B>,
                                  path: uri::PathAndQuery)
        -> client_streaming::ResponseFuture<M, T::Future, T::ResponseBody>
    where B: IntoBody<T::RequestBody>,
    {
        let response = self.streaming(request, path);
        client_streaming::ResponseFuture::new(response)
    }

    pub fn server_streaming<M1, M2>(&mut self,
                                    request: ::Request<M1>,
                                    path: uri::PathAndQuery)
        -> server_streaming::ResponseFuture<M2, T::Future>
    where Once<M1>: IntoBody<T::RequestBody>,
    {
        let request = request.map(|v| stream::once(Ok(v)));
        let response = self.streaming(request, path);

        server_streaming::ResponseFuture::new(response)
    }

    /// Initiate a full streaming gRPC request
    ///
    /// # Generics
    ///
    /// **B**: The request stream of gRPC message values.
    /// **M**: The response **message** (not stream) type.
    pub fn streaming<B, M>(&mut self,
                           request: ::Request<B>,
                           path: uri::PathAndQuery)
        -> streaming::ResponseFuture<M, T::Future>
    where B: IntoBody<T::RequestBody>,
    {
        use http::header::{self, HeaderValue};

        // TODO: validate the path

        // Get the gRPC's method URI
        let mut parts = uri::Parts::default();
        parts.scheme = Some(self.scheme.clone());
        parts.authority = Some(self.authority.clone());
        parts.path_and_query = Some(path);

        // Get the URI;
        let uri = match Uri::from_parts(parts) {
            Ok(uri) => uri,
            Err(_) => unimplemented!(),
        };

        // Convert the request body
        let request = request.map(|v| v.into_body());

        // Convert to an HTTP request
        let mut request = request.into_http(uri);

        // Add the gRPC related HTTP headers
        request.headers_mut()
            .insert(header::TE, HeaderValue::from_static("trailers"));

        // Set the content type
        // TODO: Don't hard code this here
        let content_type = "application/grpc+proto";
        request.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static(content_type));

        // Call the inner HTTP service
        let response = self.inner.call(request);

        streaming::ResponseFuture::new(response)
    }
}

impl Builder {
    /// Return a new client builder
    pub fn new() -> Self {
        Self::default()
    }

    pub fn uri(&mut self, value: uri::Uri) -> &mut Self {
        // TODO: Validate that the path is empty. Doing this is waiting for
        // hyper/http#149.

        self.uri = Some(value);
        self
    }

    pub fn build<T>(&mut self, inner: T) -> Grpc<T>
    where T: HttpService,
    {
        let uri = self.uri.as_mut().expect("service URI is required");

        let scheme = uri.scheme_part()
            .expect("gRPC service URI must include scheme and authority")
            .clone();

        let authority = uri.authority_part()
            .expect("gRPC service URI must include scheme and authority")
            .clone();

        Grpc {
            inner,
            scheme,
            authority,
        }
    }
}

pub mod unary {
    use super::client_streaming;
    use codec::Streaming;

    use futures::{Future, Stream, Poll};
    use http::{response, Response};
    use prost::Message;
    use tower_h2::{Body, Data};

    use std::marker::PhantomData;

    pub struct ResponseFuture<T, U, B> {
        inner: client_streaming::ResponseFuture<T, U, B>,
    }

    impl<T, U, B> ResponseFuture<T, U, B> {
        /// Create a new client-streaming response future.
        pub(crate) fn new(inner: client_streaming::ResponseFuture<T, U, B>) -> Self {
            ResponseFuture { inner }
        }
    }

    impl<T, U, B> Future for ResponseFuture<T, U, B>
    where T: Message + Default,
          U: Future<Item = Response<B>>,
          B: Body<Data = Data>,
    {
        type Item = ::Response<T>;
        type Error = ::Error<U::Error>;

        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            self.inner.poll()
        }
    }
}

pub mod client_streaming {
    use super::streaming;
    use codec::Streaming;

    use futures::{Future, Stream, Poll};
    use http::{response, Response};
    use prost::Message;
    use tower_h2::{Body, Data};

    use std::marker::PhantomData;

    pub struct ResponseFuture<T, U, B> {
        state: State<T, U, B>,
    }

    enum State<T, U, B> {
        WaitResponse(streaming::ResponseFuture<T, U>),
        WaitMessage {
            head: Option<response::Parts>,
            stream: Streaming<T, B>,
        },
    }

    impl<T, U, B> ResponseFuture<T, U, B> {
        /// Create a new client-streaming response future.
        pub(crate) fn new(inner: streaming::ResponseFuture<T, U>) -> Self {
            let state = State::WaitResponse(inner);
            ResponseFuture { state }
        }
    }

    impl<T, U, B> Future for ResponseFuture<T, U, B>
    where T: Message + Default,
          U: Future<Item = Response<B>>,
          B: Body<Data = Data>,
    {
        type Item = ::Response<T>;
        type Error = ::Error<U::Error>;

        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            use self::State::*;

            loop {
                let response = match self.state {
                    WaitResponse(ref mut inner) => {
                        try_ready!(inner.poll())
                    }
                    WaitMessage { ref mut head, ref mut stream } => {
                        let res = stream.poll()
                            .map_err(|e| {
                                match e {
                                    ::Error::Grpc(s) => ::Error::Grpc(s),
                                    _ => ::Error::Grpc(::Status::INTERNAL),
                                }
                            });

                        let message = match try_ready!(res) {
                            Some(message) => message,
                            // TODO: handle missing message
                            None => unimplemented!(),
                        };

                        let head = head.take().unwrap();
                        let response = Response::from_parts(head, message);

                        return Ok(::Response::from_http(response).into());
                    }
                };

                let (head, body) = response
                    .into_http()
                    .into_parts();

                self.state = WaitMessage {
                    head: Some(head),
                    stream: body,
                };
            }
        }
    }
}

pub mod server_streaming {
    use super::streaming;
    use codec::Streaming;

    use futures::{Future, Stream, Poll};
    use http::{response, Response};
    use prost::Message;
    use tower_h2::{Body, Data};

    use std::marker::PhantomData;

    pub struct ResponseFuture<T, U> {
        inner: streaming::ResponseFuture<T, U>,
    }

    impl<T, U> ResponseFuture<T, U> {
        /// Create a new client-streaming response future.
        pub(crate) fn new(inner: streaming::ResponseFuture<T, U>) -> Self {
            ResponseFuture { inner }
        }
    }

    impl<T, U, B> Future for ResponseFuture<T, U>
    where T: Message + Default,
          U: Future<Item = Response<B>>,
          B: Body<Data = Data>,
    {
        type Item = ::Response<Streaming<T, B>>;
        type Error = ::Error<U::Error>;

        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            self.inner.poll()
        }
    }
}

pub mod streaming {
    use codec::Streaming;

    use futures::{Future, Poll};
    use http::Response;
    use prost::Message;
    use tower_h2::{Body, Data};

    use std::marker::PhantomData;

    pub struct ResponseFuture<T, U> {
        inner: U,
        _m: PhantomData<T>,
    }

    impl<T, U> ResponseFuture<T, U> {
        /// Create a new client-streaming response future.
        pub(crate) fn new(inner: U) -> Self {
            ResponseFuture {
                inner,
                _m: PhantomData,
            }
        }
    }

    impl<T, U, B> Future for ResponseFuture<T, U>
    where T: Message + Default,
          U: Future<Item = Response<B>>,
          B: Body<Data = Data>,
    {
        type Item = ::Response<Streaming<T, B>>;
        type Error = ::Error<U::Error>;

        fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
            use codec::Decoder;
            use generic::Streaming;

            let response = self.inner.poll()
                .map_err(::Error::Inner);

            // Get the response
            let response = try_ready!(response);

            // Destructure into the head / body
            let (head, body) = response.into_parts();

            if let Some(status) = super::check_grpc_status(&head.headers) {
                return Err(::Error::Grpc(status));
            }

            let body = Streaming::new(Decoder::new(), body, true);
            let response = Response::from_parts(head, body);

            Ok(::Response::from_http(response).into())
        }
    }
}

fn check_grpc_status(trailers: &HeaderMap) -> Option<Status> {
    trailers.get("grpc-status").map(|s| {
        Status::from_bytes(s.as_ref())
    })
}
