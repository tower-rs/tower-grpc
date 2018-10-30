use http;

#[derive(Debug)]
pub struct Request<T> {
    metadata: http::HeaderMap,
    message: T,
}

impl<T> Request<T> {
    /// Create a new gRPC request
    pub fn new(message: T) -> Self {
        Request {
            metadata: http::HeaderMap::new(),
            message,
        }
    }

    /// Get a reference to the message
    pub fn get_ref(&self) -> &T {
        &self.message
    }

    /// Get a mutable reference to the message
    pub fn get_mut(&mut self) -> &mut T {
        &mut self.message
    }

    /// Get a reference to the custom request metadata.
    pub fn metadata(&self) -> &http::HeaderMap {
        &self.metadata
    }

    /// Get a mutable reference to the request metadata.
    pub fn metadata_mut(&mut self) -> &mut http::HeaderMap {
        &mut self.metadata
    }

    /// Consumes `self`, returning the message
    pub fn into_inner(self) -> T {
        self.message
    }

    /// Convert an HTTP request to a gRPC request
    pub fn from_http(http: http::Request<T>) -> Self {
        let (head, message) = http.into_parts();
        Request {
            metadata: head.headers,
            message,
        }
    }

    pub fn into_http(self, uri: http::Uri) -> http::Request<T> {
        let mut request = http::Request::new(self.message);

        *request.version_mut() = http::Version::HTTP_2;
        *request.method_mut() = http::Method::POST;
        *request.uri_mut() = uri;
        *request.headers_mut() = self.metadata;

        request
    }

    pub fn map<F, U>(self, f: F) -> Request<U>
    where F: FnOnce(T) -> U,
    {
        let message = f(self.message);

        Request {
            metadata: self.metadata,
            message,
        }
    }
}
