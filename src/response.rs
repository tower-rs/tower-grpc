use http;

#[derive(Debug)]
pub struct Response<T> {
    http: http::Response<T>,
}

impl<T> Response<T> {
    pub fn new(message: T) -> Self {
        let mut res = http::Response::new(message);
        *res.version_mut() = http::Version::HTTP_2;

        Response {
            http: res,
        }
    }

    /// Get a reference to the message
    pub fn get_ref(&self) -> &T {
        self.http.body()
    }

    /// Get a mutable reference to the message
    pub fn get_mut(&mut self) -> &mut T {
        self.http.body_mut()
    }

    /// Consumes `self`, returning the message
    pub fn into_inner(self) -> T {
        let (_, body) = self.http.into_parts();
        body
    }

    pub(crate) fn from_http(res: http::Response<T>) -> Self {
        Response {
            http: res,
        }
    }

    pub fn into_http(self) -> http::Response<T> {
        self.http
    }

    pub fn map<F, U>(self, f: F) -> Response<U>
    where F: FnOnce(T) -> U,
    {
        let (head, body) = self.http.into_parts();
        let body = f(body);
        let http = http::Response::from_parts(head, body);
        Response::from_http(http)
    }

    // pub fn metadata()
    // pub fn metadata_bin()
}
