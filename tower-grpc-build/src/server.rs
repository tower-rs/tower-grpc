use codegen;
use comments_to_rustdoc;
use prost_build;
use super::ImportType;

/// Generates service code
pub struct ServiceGenerator;

impl ServiceGenerator {
    /// Generate the gRPC server code
    pub fn generate(&self,
                    service: &prost_build::Service,
                    scope: &mut codegen::Scope) {
        self.define(service, scope);
    }

    fn define(&self,
              service: &prost_build::Service,
              scope: &mut codegen::Scope) {
        // Create scope that contains the generated server code.
        {
            let module = scope.get_or_new_module("server")
                .vis("pub")
                .import("::tower_grpc::codegen::server", "*")
                ;

            // Re-define the try_ready macro
            module.scope()
                .raw("\
// Redefine the try_ready macro so that it doesn't need to be explicitly
// imported by the user of this generated code.
macro_rules! try_ready {
    ($e:expr) => (match $e {
        Ok(futures::Async::Ready(t)) => t,
        Ok(futures::Async::NotReady) => return Ok(futures::Async::NotReady),
        Err(e) => return Err(From::from(e)),
    })
}");

            self.define_service_trait(service, module.scope());
            self.define_server_struct(service, module.scope());

            let support = module.new_module(&::lower_name(&service.name))
                .vis("pub")
                .import("::tower_grpc::codegen::server", "*")
                .import("super", &service.name)
                ;

            self.define_response_future(service, support);
            self.define_response_body(service, support);
            self.define_kind(service, support);

            // Define methods module
            let methods = support.new_module("methods")
                .vis("pub")
                .import("::tower_grpc::codegen::server", "*")
                .import("super::super", &service.name)
                ;

            // Define service modules
            for method in &service.methods {
                methods.import_type(&method.input_type, 2);

                if !method.server_streaming {
                    methods.import_type(&method.output_type, 2);
                }

                self.define_service_method(service, method, methods);
            }
        }
    }

    fn define_service_trait(&self,
                            service: &prost_build::Service,
                            scope: &mut codegen::Scope)
    {
        let mut service_trait = codegen::Trait::new(&service.name);
        service_trait.vis("pub")
            .parent("Clone")
            .doc(&comments_to_rustdoc(&service.comments))
            ;

        for method in &service.methods {
            let name = &method.name;
            let upper_name = ::to_upper_camel(&method.proto_name);
            let future_bound;

            if method.server_streaming {
                let stream_name = format!("{}Stream", &upper_name);
                let stream_bound = format!(
                    "futures::Stream<Item = {}, Error = grpc::Status>",
                    ::unqualified(&method.output_type, &method.output_proto_type, 1));

                future_bound = format!(
                    "futures::Future<Item = grpc::Response<Self::{}>, Error = grpc::Status>",
                    stream_name);

                service_trait.associated_type(&stream_name)
                    .bound(&stream_bound);
            } else {
                future_bound = format!(
                    "futures::Future<Item = grpc::Response<{}>, Error = grpc::Status>",
                    ::unqualified(&method.output_type, &method.output_proto_type, 1));
            }

            let future_name = format!("{}Future", &upper_name);

            service_trait.associated_type(&future_name)
                .bound(&future_bound)
                ;

            for &ty in [&method.input_type, &method.output_type].iter() {
                if ::should_import(ty) {
                    let (path, ty) = ::super_import(ty, 1);

                    scope.import(&path, &ty);
                }
            }

            let input_type = ::unqualified(&method.input_type, &method.input_proto_type, 1);

            let request_type = if method.client_streaming {
                format!("grpc::Request<grpc::Streaming<{}>>", input_type)
            } else {
                format!("grpc::Request<{}>", input_type)
            };

            service_trait.new_fn(&name)
                .arg_mut_self()
                .arg("request", &request_type)
                .ret(&format!("Self::{}Future", &upper_name))
                .doc(&comments_to_rustdoc(&method.comments))
                ;
        }

        scope.push_trait(service_trait);
    }

    fn define_server_struct(&self,
                            service: &prost_build::Service,
                            scope: &mut codegen::Scope)
    {
        let name = format!("{}Server", service.name);
        let lower_name = ::lower_name(&service.name);

        scope.new_struct(&name)
            .vis("pub")
            .derive("Debug")
            .derive("Clone")
            .generic("T")
            .field(&lower_name, "T")
            ;

        scope.new_impl(&name)
            .generic("T")
            .target_generic("T")
            .bound("T", &service.name)
            .new_fn("new")
            .vis("pub")
            .arg(&lower_name, "T")
            .ret("Self")
            .line(format!("Self {{ {} }}", lower_name))
            ;

        let response_type = format!("http::Response<{}::ResponseBody<T>>", lower_name);

        // Implement service trait
        let mut service_impl = codegen::Impl::new(&name);
        service_impl.impl_trait("tower::Service<http::Request<grpc::BoxBody>>")
            .generic("T")
            .target_generic("T")
            .bound("T", &service.name)
            .associate_type("Response", &response_type)
            .associate_type("Error", "grpc::Never")
            .associate_type("Future", &format!("{}::ResponseFuture<T>", lower_name))
            ;

        service_impl.new_fn("poll_ready")
            .arg_mut_self()
            .ret("futures::Poll<(), Self::Error>")
            .line("Ok(().into())")
            ;

        {
            let call = service_impl.new_fn("call")
                .arg_mut_self()
                .arg("request", "http::Request<grpc::BoxBody>")
                .ret("Self::Future")
                .line(&format!("use self::{}::Kind::*;", lower_name))
                .line("")
                ;

            let mut route_block = codegen::Block::new("match request.uri().path()");

            for method in &service.methods {
                let upper_name = ::to_upper_camel(&method.proto_name);

                // The service method path.
                let match_line = format!("{} =>", ::method_path(service, method));

                // Match the service path
                let mut handle = codegen::Block::new(&match_line);


                match (method.client_streaming, method.server_streaming) {
                    (false, false) => {
                        handle.line(&format!(
                                "let service = {}::methods::{}(self.{}.clone());",
                                lower_name, &upper_name, lower_name));

                        handle.line("let response = grpc::unary(service, request);");
                    }
                    (false, true) => {
                        handle.line(&format!(
                                "let service = {}::methods::{}(self.{}.clone());",
                                lower_name, &upper_name, lower_name));

                        handle.line("let response = grpc::server_streaming(service, request);");
                    }
                    (true, false) => {
                        handle.line(&format!(
                                "let mut service = {}::methods::{}(self.{}.clone());",
                                lower_name, &upper_name, lower_name));

                        handle.line("let response = grpc::client_streaming(&mut service, request);");
                    }
                    (true, true) => {
                        handle.line(&format!(
                                "let mut service = {}::methods::{}(self.{}.clone());",
                                lower_name, &upper_name, lower_name));

                        handle.line("let response = grpc::streaming(&mut service, request);");
                    }
                }

                handle.line(&format!(
                        "{}::ResponseFuture {{ kind: {}(response) }}",
                        lower_name, &upper_name));

                route_block.push_block(handle);
            }

            let mut catch_all = codegen::Block::new("_ =>");
            catch_all
                .line(&format!(
                    "{}::ResponseFuture {{ kind: {}(grpc::unimplemented(format!(\"unknown service: {{:?}}\", request.uri().path()))) }}",
                    lower_name,
                    UNIMPLEMENTED_VARIANT,
                ));

            route_block.push_block(catch_all);
            call.push_block(route_block);
        }

        scope.push_impl(service_impl);

        // MakeService impl
        {
            let imp = scope.new_impl(&name)
                .generic("T")
                .target_generic("T")
                .impl_trait("tower::Service<()>")
                .bound("T", &service.name)
                .associate_type("Response", "Self")
                .associate_type("Error", "grpc::Never")
                .associate_type("Future", "futures::FutureResult<Self::Response, Self::Error>")
                ;


            imp.new_fn("poll_ready")
                .arg_mut_self()
                .ret("futures::Poll<(), Self::Error>")
                .line("Ok(futures::Async::Ready(()))")
                ;

            imp.new_fn("call")
                .arg_mut_self()
                .arg("_target", "()")
                .ret("Self::Future")
                .line("futures::ok(self.clone())")
                ;
        }

        #[cfg(feature = "tower-h2")]
        // Service that converts tower_grpc::BoxBody to tower_h2 bodies
        {
            let imp = scope.new_impl(&name)
                .generic("T")
                .target_generic("T")
                .impl_trait("tower::Service<http::Request<tower_h2::RecvBody>>")
                .bound("T", &service.name)
                .associate_type("Response", "<Self as tower::Service<http::Request<grpc::BoxBody>>>::Response")
                .associate_type("Error", "<Self as tower::Service<http::Request<grpc::BoxBody>>>::Error")
                .associate_type("Future", "<Self as tower::Service<http::Request<grpc::BoxBody>>>::Future")
                ;


            imp.new_fn("poll_ready")
                .arg_mut_self()
                .ret("futures::Poll<(), Self::Error>")
                .line("tower::Service::<http::Request<grpc::BoxBody>>::poll_ready(self)")
                ;

            imp.new_fn("call")
                .arg_mut_self()
                .arg("request", "http::Request<tower_h2::RecvBody>")
                .ret("Self::Future")
                .line("let request = request.map(|b| grpc::BoxBody::map_from(b));")
                .line("tower::Service::<http::Request<grpc::BoxBody>>::call(self, request)")
                ;
        }
    }

    fn define_response_future(&self,
                              service: &prost_build::Service,
                              module: &mut codegen::Module)
    {
        module.new_struct("ResponseFuture")
            .generic("T")
            .bound("T", &service.name)
            .vis("pub")
            .field("pub(super) kind", response_fut_kind(service))
            ;

        module.new_impl("ResponseFuture")
            .generic("T")
            .target_generic("T")
            .impl_trait("futures::Future")
            .bound("T", &service.name)
            .associate_type("Item", "http::Response<ResponseBody<T>>")
            .associate_type("Error", "grpc::Never")
            .new_fn("poll")
            .arg_mut_self()
            .ret("futures::Poll<Self::Item, Self::Error>")
            .line("use self::Kind::*;")
            .line("")
            .push_block({
                let mut match_kind = codegen::Block::new("match self.kind");

                for method in &service.methods {
                    let upper_name = ::to_upper_camel(&method.proto_name);

                    let match_line = format!(
                        "{}(ref mut fut) =>", &upper_name
                    );

                    let mut blk = codegen::Block::new(&match_line);
                    blk
                        .line("let response = try_ready!(fut.poll());")
                        .line("let response = response.map(|body| {")
                        .line(&format!("    ResponseBody {{ kind: {}(body) }}", &upper_name))
                        .line("});")
                        .line("Ok(response.into())")
                        ;

                    match_kind.push_block(blk);
                }

                let mut err = codegen::Block::new(&format!(
                    "{}(ref mut fut) =>",
                    UNIMPLEMENTED_VARIANT,
                ));

               err
                    .line("let response = try_ready!(fut.poll());")
                    .line("let response = response.map(|body| {")
                    .line(&format!("    ResponseBody {{ kind: {}(body) }}", UNIMPLEMENTED_VARIANT))
                    .line("});")
                    .line("Ok(response.into())")
                    ;

                match_kind.push_block(err);
                match_kind
            })
            ;
    }

    fn define_response_body(&self,
                            service: &prost_build::Service,
                            module: &mut codegen::Module)
    {
        for method in &service.methods {
            if ::should_import(&method.input_type) {
                let (path, thing) = ::super_import(&method.input_type, 2);
                module.import(&path, &thing);
            }
        }

        module.new_struct("ResponseBody")
            .generic("T")
            .bound("T", &service.name)
            .vis("pub")
            .field("pub(super) kind", response_body_kind(service))
            ;

        // impl grpc::Body
        {
            let imp = module.new_impl("ResponseBody")
                .generic("T")
                .target_generic("T")
                .impl_trait("tower::HttpBody")
                .bound("T", &service.name)
                .associate_type("Item", "<grpc::BoxBody as grpc::Body>::Item")
                .associate_type("Error", "grpc::Status")
                ;

            let mut is_end_stream_block = codegen::Block::new("match self.kind");
            let mut poll_buf_block = codegen::Block::new("match self.kind");
            let mut poll_trailers_block = codegen::Block::new("match self.kind");

            for method in &service.methods {
                let upper_name = ::to_upper_camel(&method.proto_name);

                is_end_stream_block
                    .line(&format!(
                        "{}(ref v) => v.is_end_stream(),",
                        &upper_name
                    ));

                poll_buf_block
                    .line(&format!(
                        "{}(ref mut v) => v.poll_buf(),",
                         &upper_name
                    ));

                poll_trailers_block
                    .line(&format!(
                        "{}(ref mut v) => v.poll_trailers(),",
                         &upper_name
                    ));
            }

            is_end_stream_block
                .line(&format!(
                    "{}(_) => true,",
                    UNIMPLEMENTED_VARIANT
                ));
            poll_buf_block
                .line(&format!(
                    "{}(_) => Ok(None.into()),",
                    UNIMPLEMENTED_VARIANT
                ));
            poll_trailers_block
                .line(&format!(
                    "{}(_) => Ok(None.into()),",
                    UNIMPLEMENTED_VARIANT
                ));

            imp.new_fn("is_end_stream")
                .arg_ref_self()
                .ret("bool")
                .line("use self::Kind::*;")
                .line("")
                .push_block(is_end_stream_block)
                ;

            imp.new_fn("poll_buf")
                .arg_mut_self()
                .ret("futures::Poll<Option<Self::Item>, Self::Error>")
                .line("use self::Kind::*;")
                .line("")
                .push_block(poll_buf_block)
                ;

            imp.new_fn("poll_trailers")
                .arg_mut_self()
                .ret("futures::Poll<Option<http::HeaderMap>, Self::Error>")
                .line("use self::Kind::*;")
                .line("")
                .push_block(poll_trailers_block)
                ;
        }
    }

    fn define_kind(&self,
                   service: &prost_build::Service,
                   module: &mut codegen::Module)
    {
        let kind_enum = module.new_enum("Kind")
            .vis("pub(super)")
            .derive("Debug")
            .derive("Clone")
            .allow("non_camel_case_types")
            ;

        for method in &service.methods {

            let upper_name = ::to_upper_camel(&method.proto_name);
            kind_enum.generic(&upper_name);
            kind_enum.new_variant(&upper_name)
                .tuple(&upper_name)
                ;
        }

        // Unimplemented variant
        kind_enum.generic(UNIMPLEMENTED_VARIANT);
        kind_enum.new_variant(UNIMPLEMENTED_VARIANT)
            .tuple(UNIMPLEMENTED_VARIANT)
            ;
    }

    fn define_service_method(&self,
                             service: &prost_build::Service,
                             method: &prost_build::Method,
                             module: &mut codegen::Module)
    {
        let upper_name = ::to_upper_camel(&method.proto_name);

        module.new_struct(&upper_name)
            .vis("pub")
            .generic("T")
            .tuple_field("pub T")
            ;
        let mut request = codegen::Type::new("grpc::Request");
        let mut response = codegen::Type::new("grpc::Response");
        let request_stream = streaming_input_type(&method, 3);
        let response_stream = format!("T::{}Stream", &upper_name);

        match (method.client_streaming, method.server_streaming) {
            (false, false) => {
                request.generic(::unqualified(&method.input_type, &method.input_proto_type, 3));
                response.generic(::unqualified(&method.output_type, &method.output_proto_type, 3));
            }
            (false, true) => {
                request.generic(::unqualified(&method.input_type, &method.input_proto_type, 3));
                response.generic(&response_stream);
            }
            (true, false) => {
                response.generic(::unqualified(&method.output_type, &method.output_proto_type, 3));
                request.generic(&request_stream);
            }
            (true, true) => {
                request.generic(&request_stream);
                response.generic(&response_stream);
            }
        }

        let mut req_str = String::new();
        request.fmt(&mut codegen::Formatter::new(&mut req_str)).unwrap();

        let imp = module.new_impl(&upper_name)
            .generic("T")
            .target_generic("T")
            .impl_trait(format!("tower::Service<{}>", req_str))
            .bound("T", &service.name)
            .associate_type("Response", response)
            .associate_type("Error", "grpc::Status")
            .associate_type("Future", &format!("T::{}Future", &upper_name))
            ;

        imp.new_fn("poll_ready")
            .arg_mut_self()
            .ret("futures::Poll<(), Self::Error>")
            .line("Ok(futures::Async::Ready(()))")
            ;

        imp.new_fn("call")
            .arg_mut_self()
            .arg("request", &req_str)
            .ret("Self::Future")
            .line(&format!("self.0.{}(request)", method.name))
            ;
    }
}

// ===== Here be the crazy types =====

fn response_fut_kind(service: &prost_build::Service) -> String {
    use std::fmt::Write;

    let mut ret = "Kind<\n".to_string();

    // grpc::unary::ResponseFuture<methods::SayHello<T>, grpc::BoxBody>
    for method in &service.methods {
        // Add a comment describing this generic
        write!(&mut ret, "    // {}\n", method.proto_name).unwrap();

        let upper_name = ::to_upper_camel(&method.proto_name);
        match (method.client_streaming, method.server_streaming) {
            (false, false) => {
                write!(&mut ret, "    grpc::unary::ResponseFuture<methods::{}<T>, grpc::BoxBody, {}>,\n",
                                 &upper_name, ::unqualified(&method.input_type, &method.input_proto_type, 2)).unwrap();
            }
            (false, true) => {
                write!(&mut ret, "    grpc::server_streaming::ResponseFuture<methods::{}<T>, grpc::BoxBody, {}>,\n",
                                 &upper_name, ::unqualified(&method.input_type, &method.input_proto_type, 2)).unwrap();
            }
            (true, false) => {
                write!(&mut ret, "    grpc::client_streaming::ResponseFuture<methods::{}<T>, {}>,\n",
                                 &upper_name, streaming_input_type(&method, 2)).unwrap();
            }
            (true, true) => {
                let mut request = codegen::Type::new("grpc::Streaming");
                request.generic(::unqualified(&method.input_type, &method.input_proto_type, 2));
                write!(&mut ret, "    grpc::streaming::ResponseFuture<methods::{}<T>, {}>,\n",
                                 &upper_name, streaming_input_type(&method, 2)).unwrap();
            }
        }
    }

    // Unimplemented variant
    write!(&mut ret, "    // A generated catch-all for unimplemented service calls\n").unwrap();
    write!(&mut ret, "    grpc::unimplemented::ResponseFuture,\n").unwrap();

    ret.push_str(">");
    ret
}

static UNIMPLEMENTED_VARIANT: &str = "__Generated__Unimplemented";

fn response_body_kind(service: &prost_build::Service) -> String {
    use std::fmt::Write;

    let mut ret = "Kind<\n".to_string();

    // grpc::Encode<grpc::unary::Once<<methods::SayHello<T> as grpc::UnaryService>::Response>>
    for method in &service.methods {
        write!(&mut ret, "    // {}\n", method.proto_name).unwrap();
        let upper_name = ::to_upper_camel(&method.proto_name);

        match (method.client_streaming, method.server_streaming) {
            (false, false) => {
                write!(&mut ret, "    grpc::Encode<grpc::unary::Once<<methods::{}<T> as grpc::UnaryService<{}>>::Response>>,\n",
                                 &upper_name, ::unqualified(&method.input_type, &method.input_proto_type, 2)).unwrap();
            }
            (false, true) => {
                write!(&mut ret, "    grpc::Encode<<methods::{}<T> as grpc::ServerStreamingService<{}>>::ResponseStream>,\n",
                                 &upper_name, ::unqualified(&method.input_type, &method.input_proto_type, 2)).unwrap();
            }
            (true, false) => {
                write!(&mut ret, "    grpc::Encode<grpc::unary::Once<<methods::{}<T> as grpc::ClientStreamingService<{}>>::Response>>,\n",
                                 &upper_name, streaming_input_type(&method, 2)
                            ).unwrap();
            }
            (true, true) => {
                write!(&mut ret, "    grpc::Encode<<methods::{}<T> as grpc::StreamingService<{}>>::ResponseStream>,\n",
                                 &upper_name, streaming_input_type(&method, 2)
                            ).unwrap();
            }
        }
    }

    // Unimplemented variant
    write!(&mut ret, "    // A generated catch-all for unimplemented service calls\n").unwrap();
    write!(&mut ret, "    (),\n").unwrap();

    ret.push_str(">");
    ret
}

fn streaming_input_type(method: &prost_build::Method, level: usize) -> String {
    format!("grpc::Streaming<{}>", ::unqualified(&method.input_type, &method.input_proto_type, level))
}
