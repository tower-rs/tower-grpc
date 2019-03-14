extern crate tower_grpc;

pub mod uses_empty {
    include!(concat!(env!("OUT_DIR"), "/uses_empty.rs"));
}

#[cfg(test)]
mod tests {
    #[test]
    fn can_call() {
        use ::uses_empty::client::UsesEmpty;
        use ::tower_grpc::codegen::client::*;
        use ::tower_grpc::codegen::client::futures::Future;

        #[allow(dead_code)]
        fn zomg<T, R>(client: &mut UsesEmpty<T>)
        where T: tower::HttpService<R>,
              T::ResponseBody: grpc::Body,
              grpc::unary::Once<()>: grpc::Encodable<R>
        {
            let _ = client.do_call(tower_grpc::Request::new(())).map(|resp| {
                let inner: () = resp.into_inner();
                inner
            });
        }
    }
}
