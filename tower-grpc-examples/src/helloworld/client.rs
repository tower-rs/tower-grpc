#![feature(async_await)]
// #![deny(warnings, rust_2018_idioms)]

use tokio::net::TcpStream;
use tower_grpc::Request;
use tower_h2::Connection;

#[allow(unused_variables)]
pub mod hello_world {
    include!(concat!(env!("OUT_DIR"), "/helloworld.rs"));
}

use hello_world::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let addr = "[::1]:50051";
    let io = TcpStream::connect(&addr.parse()?).await?;

    let svc = {
        let conn = Connection::handshake(io).await?;
        add_origin::layer(conn, &format!("http://{}", addr))?
    };

    let req = Request::new(HelloRequest {
        name: "What is in a name?".to_string(),
    });

    let mut svc = client::Greeter::new(svc).ready().await?;
    let res = svc.say_hello(req).await?;

    println!("RESPONSE={:?}", res);

    Ok(())
}

mod add_origin {
    use http::{uri, HttpTryFrom, Request, Uri};
    use std::marker::PhantomData;
    use std::task::{Context, Poll};
    use tower_service::Service;

    pub fn layer<T, B, O>(inner: T, origin: O) -> Result<AddOrigin<T, B>, <Uri as HttpTryFrom<O>>::Error>
    where
        Uri: HttpTryFrom<O>,
    {
        let origin = Uri::try_from(origin)?;

        Ok(AddOrigin {
            inner,
            origin,
            _pd: PhantomData,
        })
    }

    pub struct AddOrigin<T, B> {
        inner: T,
        origin: Uri,
        _pd: PhantomData<B>,
    }

    impl<T, B> Service<Request<B>> for AddOrigin<T, B>
    where
        T: Service<Request<B>> + Send + Unpin,
        B: Send + Unpin,
    {
        type Response = T::Response;
        type Error = T::Error;
        type Future = T::Future;

        fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
            self.inner.poll_ready(cx)
        }

        fn call(&mut self, request: Request<B>) -> Self::Future {
            let parts = uri::Parts::from(self.origin.clone());

            let scheme = parts.scheme.unwrap();
            let authority = parts.authority.unwrap();

            let _ = match parts.path_and_query {
                None => Ok(()),
                Some(ref path) if path == "/" => Ok(()),
                _ => Err("something went wrong!".to_string()),
            }
            .unwrap();

            // Split the request into the head and the body.
            let (mut head, body) = request.into_parts();

            // Split the request URI into parts.
            let mut uri: http::uri::Parts = head.uri.into();

            // Update the URI parts, setting the scheme and authority
            uri.authority = Some(authority.clone());
            uri.scheme = Some(scheme.clone());

            // Update the the request URI
            head.uri = http::Uri::from_parts(uri).expect("valid uri");

            self.inner.call(Request::from_parts(head, body))
        }
    }

    impl<T: Clone, B> Clone for AddOrigin<T, B> {
        fn clone(&self) -> Self {
            Self {
                inner: self.inner.clone(),
                origin: self.origin.clone(),
                _pd: PhantomData
            }
        }
    }
}
