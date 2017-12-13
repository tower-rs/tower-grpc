use codegen;
use prost_build;

use std::fmt;

/// Generates service code
pub struct ServiceGenerator;

impl ServiceGenerator {
    /// Generate the gRPC server code
    pub fn generate(&self, service: &prost_build::Service, buf: &mut String) -> fmt::Result {
        let scope = self.define(service);
        let mut fmt = codegen::Formatter::new(buf);

        scope.fmt(&mut fmt)
    }

    fn define(&self, service: &prost_build::Service) -> codegen::Scope {
        // Create scope that contains the generated server code.
        let mut scope = codegen::Scope::new();

        {
            let module = scope.new_module("server")
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
                .import("super", &service.proto_name)
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
                let (input_path, input_type) = ::super_import(&method.input_type, 2);
                let (output_path, output_type) = ::super_import(&method.output_type, 2);

                methods.import(&input_path, input_type);
                methods.import(&output_path, output_type);

                self.define_service_method(service, method, methods);
            }
        }

        scope
    }

    fn define_service_trait(&self, service: &prost_build::Service, scope: &mut codegen::Scope) {
        let mut service_trait = codegen::Trait::new(&service.name);
        service_trait.vis("pub")
            .parent("Clone")
            ;

        for method in &service.methods {
            let name = ::lower_name(&method.proto_name);

            let future_bound;

            if method.server_streaming {
                let stream_name = format!("{}Stream", &method.proto_name);
                let stream_bound = format!(
                    "futures::Stream<Item = {}, Error = grpc::Error>",
                    ::unqualified(&method.output_type));

                future_bound = format!(
                    "futures::Future<Item = grpc::Response<Self::{}>, Error = grpc::Error>",
                    stream_name);

                service_trait.associated_type(&stream_name)
                    .bound(&stream_bound);
            } else {
                future_bound = format!(
                    "futures::Future<Item = grpc::Response<{}>, Error = grpc::Error>",
                    ::unqualified(&method.output_type));
            }

            let future_name = format!("{}Future", &method.proto_name);

            service_trait.associated_type(&future_name)
                .bound(&future_bound)
                ;

            let (input_path, input_type) = ::super_import(&method.input_type, 1);
            let (output_path, output_type) = ::super_import(&method.output_type, 1);

            scope.import(&input_path, input_type);
            scope.import(&output_path, output_type);

            let response_type = if method.client_streaming {
                format!("grpc::Request<grpc::Streaming<{}>>", input_type)
            } else {
                format!("grpc::Request<{}>", input_type)
            };

            service_trait.new_fn(&name)
                .arg_mut_self()
                .arg("request", &response_type)
                .ret(&format!("Self::{}Future", method.proto_name))
                ;
        }

        scope.push_trait(service_trait);
    }

    fn define_server_struct(&self, service: &prost_build::Service, scope: &mut codegen::Scope) {
        let name = format!("{}Server", service.name);
        let lower_name = ::lower_name(&service.name);

        scope.new_struct(&name)
            .vis("pub")
            .derive("Debug")
            .derive("Clone")
            .generic("T")
            .field(&lower_name, "T")
            ;

        {
            let imp = scope.new_impl(&name)
                .generic("T")
                .target_generic("T")
                .bound("T", &service.name)
                ;

            imp.new_fn("new")
                .vis("pub")
                .arg(&lower_name, "T")
                .ret("Self")
                .line(format!("Self {{ {} }}", lower_name))
                ;
        }

        let response_type = format!("http::Response<{}::ResponseBody<T>>", lower_name);

        // Implement service trait
        let mut service_impl = codegen::Impl::new(&name);
        service_impl.impl_trait("tower::Service")
            .generic("T")
            .target_generic("T")
            .bound("T", &service.proto_name)
            .associate_type("Request", "http::Request<tower_h2::RecvBody>")
            .associate_type("Response", &response_type)
            .associate_type("Error", "h2::Error")
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
                .arg("request", "Self::Request")
                .ret("Self::Future")
                .line(&format!("use self::{}::Kind::*;", lower_name))
                .line("")
                ;

            let mut route_block = codegen::Block::new("match request.uri().path()");

            for method in &service.methods {
                // The service method path.
                let match_line = format!("{} =>", ::method_path(service, method));

                // Match the service path
                let mut handle = codegen::Block::new(&match_line);


                match (method.client_streaming, method.server_streaming) {
                    (false, false) => {
                        handle.line(&format!(
                                "let service = {}::methods::{}(self.{}.clone());",
                                lower_name, method.proto_name, lower_name));

                        handle.line("let response = grpc::Grpc::unary(service, request);");
                    }
                    (false, true) => {
                        handle.line(&format!(
                                "let service = {}::methods::{}(self.{}.clone());",
                                lower_name, method.proto_name, lower_name));

                        handle.line("let response = grpc::Grpc::server_streaming(service, request);");
                    }
                    (true, false) => {
                        handle.line(&format!(
                                "let mut service = {}::methods::{}(self.{}.clone());",
                                lower_name, method.proto_name, lower_name));

                        handle.line("let response = grpc::Grpc::client_streaming(&mut service, request);");
                    }
                    (true, true) => {
                        handle.line(&format!(
                                "let mut service = {}::methods::{}(self.{}.clone());",
                                lower_name, method.proto_name, lower_name));

                        handle.line("let response = grpc::Grpc::streaming(&mut service, request);");
                    }
                }

                handle.line(&format!(
                        "{}::ResponseFuture {{ kind: Ok({}(response)) }}",
                        lower_name, method.proto_name));

                route_block.push_block(handle);
            }

            let mut catch_all = codegen::Block::new("_ =>");
            catch_all
                .line(&format!("{}::ResponseFuture {{ kind: Err(grpc::Status::UNIMPLEMENTED) }}",
                               lower_name));

            route_block.push_block(catch_all);
            call.push_block(route_block);
        }

        scope.push_impl(service_impl);

        scope.new_impl(&name)
            .generic("T")
            .target_generic("T")
            .impl_trait("tower::NewService")
            .bound("T", &service.proto_name)
            .associate_type("Request", "http::Request<tower_h2::RecvBody>")
            .associate_type("Response", &response_type)
            .associate_type("Error", "h2::Error")
            .associate_type("Service", "Self")
            .associate_type("InitError", "h2::Error")
            .associate_type("Future", "futures::FutureResult<Self::Service, Self::Error>")
            .new_fn("new_service")
            .arg_ref_self()
            .ret("Self::Future")
            .line("futures::ok(self.clone())")
            ;
    }

    fn define_response_future(&self,
                              service: &prost_build::Service,
                              module: &mut codegen::Module)
    {
        let mut ty = codegen::Type::new("Result");
        ty.generic(response_fut_kind(service));
        ty.generic("grpc::Status");

        module.new_struct("ResponseFuture")
            .generic("T")
            .bound("T", &service.proto_name)
            .vis("pub")
            .field("pub(super) kind", ty)
            ;

        module.new_impl("ResponseFuture")
            .generic("T")
            .target_generic("T")
            .impl_trait("futures::Future")
            .bound("T", &service.proto_name)
            .associate_type("Item", "http::Response<ResponseBody<T>>")
            .associate_type("Error", "h2::Error")
            .new_fn("poll")
            .arg_mut_self()
            .ret("futures::Poll<Self::Item, Self::Error>")
            .line("use self::Kind::*;")
            .line("")
            .push_block({
                let mut match_kind = codegen::Block::new("match self.kind");

                for method in &service.methods {
                    let match_line = format!("Ok({}(ref mut fut)) =>", method.proto_name);

                    let mut blk = codegen::Block::new(&match_line);
                    blk
                        .line("let response = try_ready!(fut.poll());")
                        .line("let (head, body) = response.into_parts();")
                        .line(&format!("let body = ResponseBody {{ kind: Ok({}(body)) }};", method.proto_name))
                        .line("let response = http::Response::from_parts(head, body);")
                        .line("Ok(response.into())")
                        ;

                    match_kind.push_block(blk);
                }

                let mut err = codegen::Block::new("Err(ref status) =>");

                err
                    .line("let body = ResponseBody { kind: Err(status.clone()) };")
                    .line("Ok(grpc::Response::new(body).into_http().into())")
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
        let mut ty = codegen::Type::new("Result");
        ty.generic(response_body_kind(service));
        ty.generic("grpc::Status");

        module.new_struct("ResponseBody")
            .generic("T")
            .bound("T", &service.proto_name)
            .vis("pub")
            .field("pub(super) kind", ty)
            ;

        let imp = module.new_impl("ResponseBody")
            .generic("T")
            .target_generic("T")
            .impl_trait("tower_h2::Body")
            .bound("T", &service.proto_name)
            .associate_type("Data", "bytes::Bytes")
            ;

        let mut is_end_stream_block = codegen::Block::new("match self.kind");
        let mut poll_data_block = codegen::Block::new("match self.kind");
        let mut poll_trailers_block = codegen::Block::new("match self.kind");

        for method in &service.methods {
            is_end_stream_block
                .line(&format!("Ok({}(ref v)) => v.is_end_stream(),", method.proto_name));

            poll_data_block
                .line(&format!("Ok({}(ref mut v)) => v.poll_data(),", method.proto_name));

            poll_trailers_block
                .line(&format!("Ok({}(ref mut v)) => v.poll_trailers(),", method.proto_name));
        }

        is_end_stream_block.line("Err(_) => true,");
        poll_data_block.line("Err(_) => Ok(None.into()),");

        let mut poll_trailers_catch_all = codegen::Block::new("Err(ref status) =>");
        poll_trailers_catch_all
            .line("let mut map = http::HeaderMap::new();")
            .line("map.insert(\"grpc-status\", status.to_header_value());")
            .line("Ok(Some(map).into())")
            ;

        poll_trailers_block.push_block(poll_trailers_catch_all);

        {
            imp.new_fn("is_end_stream")
                .arg_ref_self()
                .ret("bool")
                .line("use self::Kind::*;")
                .line("")
                .push_block(is_end_stream_block)
                ;

            imp.new_fn("poll_data")
                .arg_mut_self()
                .ret("futures::Poll<Option<Self::Data>, h2::Error>")
                .line("use self::Kind::*;")
                .line("")
                .push_block(poll_data_block)
                ;

            imp.new_fn("poll_trailers")
                .arg_mut_self()
                .ret("futures::Poll<Option<http::HeaderMap>, h2::Error>")
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
            ;

        for method in &service.methods {
            kind_enum.generic(&method.proto_name);
            kind_enum.new_variant(&method.proto_name)
                .tuple(&method.proto_name)
                ;
        }
    }

    fn define_service_method(&self,
                             service: &prost_build::Service,
                             method: &prost_build::Method,
                             module: &mut codegen::Module)
    {
        module.new_struct(&method.proto_name)
            .vis("pub")
            .generic("T")
            .tuple_field("pub T")
            ;

        let mut request = codegen::Type::new("grpc::Request");
        let mut response = codegen::Type::new("grpc::Response");
        let request_stream = format!("grpc::Streaming<{}>", ::unqualified(&method.input_type));
        let response_stream = format!("T::{}Stream", method.proto_name);

        match (method.client_streaming, method.server_streaming) {
            (false, false) => {
                request.generic(::unqualified(&method.input_type));
                response.generic(::unqualified(&method.output_type));
            }
            (false, true) => {
                request.generic(::unqualified(&method.input_type));
                response.generic(&response_stream);
            }
            (true, false) => {
                response.generic(::unqualified(&method.output_type));
                request.generic(&request_stream);
            }
            (true, true) => {
                request.generic(&request_stream);
                response.generic(&response_stream);
            }
        }

        module.new_impl(&method.proto_name)
            .generic("T")
            .target_generic("T")
            .impl_trait("tower::ReadyService")
            .bound("T", &service.proto_name)
            .associate_type("Request", request)
            .associate_type("Response", response)
            .associate_type("Error", "grpc::Error")
            .associate_type("Future", &format!("T::{}Future", method.proto_name))
            .new_fn("call")
            .arg_mut_self()
            .arg("request", "Self::Request")
            .ret("Self::Future")
            .line(&format!("self.0.{}(request)", method.name))
            ;
    }
}

// ===== Here be the crazy types =====

fn response_fut_kind(service: &prost_build::Service) -> String {
    use std::fmt::Write;

    // Handle theempty case...
    if service.methods.is_empty() {
        return "Kind".to_string();
    }

    let mut ret = "Kind<\n".to_string();

    // grpc::unary::ResponseFuture<methods::SayHello<T>, tower_h2::RecvBody>
    for method in &service.methods {
        match (method.client_streaming, method.server_streaming) {
            (false, false) => {
                write!(&mut ret, "    grpc::unary::ResponseFuture<methods::{}<T>, tower_h2::RecvBody>,\n",
                                 method.proto_name).unwrap();
            }
            (false, true) => {
                write!(&mut ret, "    grpc::server_streaming::ResponseFuture<methods::{}<T>, tower_h2::RecvBody>,\n",
                                 method.proto_name).unwrap();
            }
            (true, false) => {
                write!(&mut ret, "    grpc::client_streaming::ResponseFuture<methods::{}<T>>,\n",
                                 method.proto_name).unwrap();
            }
            (true, true) => {
                write!(&mut ret, "    grpc::streaming::ResponseFuture<methods::{}<T>>,\n",
                                 method.proto_name).unwrap();
            }
        }
    }

    ret.push_str(">");
    ret
}

fn response_body_kind(service: &prost_build::Service) -> String {
    use std::fmt::Write;

    // Handle theempty case...
    if service.methods.is_empty() {
        return "Kind".to_string();
    }

    let mut ret = "Kind<\n".to_string();

    // grpc::Encode<grpc::unary::Once<<methods::SayHello<T> as grpc::UnaryService>::Response>>
    for method in &service.methods {
        match (method.client_streaming, method.server_streaming) {
            (false, false) => {
                write!(&mut ret, "    grpc::Encode<grpc::unary::Once<<methods::{}<T> as grpc::UnaryService>::Response>>,\n",
                                 method.proto_name).unwrap();
            }
            (false, true) => {
                write!(&mut ret, "    grpc::Encode<<methods::{}<T> as grpc::ServerStreamingService>::ResponseStream>,\n",
                                 method.proto_name).unwrap();
            }
            (true, false) => {
                write!(&mut ret, "    grpc::Encode<grpc::unary::Once<<methods::{}<T> as grpc::ClientStreamingService>::Response>>,\n",
                                 method.proto_name).unwrap();
            }
            (true, true) => {
                write!(&mut ret, "    grpc::Encode<<methods::{}<T> as grpc::StreamingService>::ResponseStream>,\n",
                                 method.proto_name).unwrap();
            }
        }
    }

    ret.push_str(">");
    ret
}
