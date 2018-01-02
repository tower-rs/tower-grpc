use http;

#[derive(Debug)]
pub struct Request<T> {
    headers: http::HeaderMap,
    message: T,
}

impl<T> Request<T> {
    /// Create a new gRPC request
    pub fn new(message: T) -> Self {
        Request {
            headers: http::HeaderMap::new(),
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

    /// Get a reference to the request headers.
    pub fn headers(&self) -> &http::HeaderMap {
        &self.headers
    }

    /// Get a mutable reference to the request headers.
    pub fn headers_mut(&mut self) -> &mut http::HeaderMap {
        &mut self.headers
    }

    /// Consumes `self`, returning the message
    pub fn into_inner(self) -> T {
        self.message
    }

    /// Convert an HTTP request to a gRPC request
    pub fn from_http(http: http::Request<T>) -> Self {
        let (head, message) = http.into_parts();
        Request {
            headers: head.headers,
            message,
        }
    }

    pub fn into_http(self, uri: http::Uri) -> http::Request<T> {
        let mut request = http::Request::new(self.message);

        *request.version_mut() = http::Version::HTTP_2;
        *request.method_mut() = http::Method::POST;
        *request.uri_mut() = uri;
        *request.headers_mut() = self.headers;

        request
    }

    pub fn map<F, U>(self, f: F) -> Request<U>
    where F: FnOnce(T) -> U,
    {
        let message = f(self.message);

        Request {
            headers: self.headers,
            message,
        }
    }
}
