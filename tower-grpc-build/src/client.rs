use super::ImportType;
use codegen;
use comments_to_rustdoc;
use prost_build;

/// Generates service code
pub struct ServiceGenerator;

// ===== impl ServiceGenerator =====

impl ServiceGenerator {
    pub fn generate(&self, service: &prost_build::Service, scope: &mut codegen::Scope) {
        self.define(service, scope);
    }

    fn define(&self, service: &prost_build::Service, scope: &mut codegen::Scope) {
        // Create scope that contains the generated client code.
        let scope = scope
            .get_or_new_module("client")
            .vis("pub")
            .import("::tower_grpc::codegen::client", "*")
            .scope();

        self.import_message_types(service, scope);
        self.define_client_struct(service, scope);
        self.define_client_impl(service, scope);
    }

    fn import_message_types(&self, service: &prost_build::Service, scope: &mut codegen::Scope) {
        for method in &service.methods {
            scope.import_type(&method.input_type, 1);
            scope.import_type(&method.output_type, 1);
        }
    }

    fn define_client_struct(&self, service: &prost_build::Service, scope: &mut codegen::Scope) {
        scope
            .new_struct(&service.name)
            .vis("pub")
            .generic("T")
            .derive("Debug")
            .derive("Clone")
            .doc(&comments_to_rustdoc(&service.comments))
            .field("inner", "grpc::Grpc<T>");
    }

    fn define_client_impl(&self, service: &prost_build::Service, scope: &mut codegen::Scope) {
        let imp = scope
            .new_impl(&service.name)
            .generic("T")
            .target_generic("T");

        // TODO: figure out what to do about potential conflicts with these
        // "inherent" methods and those from the gRPC service methods.
        //
        // For instance, if the service had a method named "new".

        imp.new_fn("new")
            .vis("pub")
            .arg("inner", "T")
            .ret("Self")
            .line("let inner = grpc::Grpc::new(inner);")
            .line("Self { inner }");

        imp.new_fn("poll_ready")
            .doc("Poll whether this client is ready to send another request.")
            .generic("R")
            .bound("T", "grpc::GrpcService<R>")
            .vis("pub")
            .arg_mut_self()
            .ret("futures::Poll<(), grpc::Status>")
            .line("self.inner.poll_ready()");

        imp.new_fn("ready")
            .doc("Get a `Future` of when this client is ready to send another request.")
            .generic("R")
            .bound("T", "grpc::GrpcService<R>")
            .vis("pub")
            .arg_self()
            .ret("impl futures::Future<Item = Self, Error = grpc::Status>")
            .line("futures::Future::map(self.inner.ready(), |inner| Self { inner })");

        for method in &service.methods {
            let name = &method.name;
            let path = ::method_path(service, method);
            let input_type = ::unqualified(&method.input_type, &method.input_proto_type, 1);
            let output_type = ::unqualified(&method.output_type, &method.output_proto_type, 1);

            let func = imp
                .new_fn(&name)
                .vis("pub")
                .generic("R")
                .bound("T", "grpc::GrpcService<R>")
                .arg_mut_self()
                .line(format!(
                    "let path = http::PathAndQuery::from_static({});",
                    path
                ))
                .doc(&comments_to_rustdoc(&service.comments));

            let mut request = codegen::Type::new("grpc::Request");

            let req_body = match (method.client_streaming, method.server_streaming) {
                (false, false) => {
                    let ret = format!(
                        "grpc::unary::ResponseFuture<{}, T::Future, T::ResponseBody>",
                        output_type
                    );

                    request.generic(&input_type);

                    func.ret(ret).line("self.inner.unary(request, path)");

                    format!("grpc::unary::Once<{}>", input_type)
                }
                (false, true) => {
                    let ret = format!(
                        "grpc::server_streaming::ResponseFuture<{}, T::Future>",
                        output_type
                    );

                    request.generic(&input_type);

                    func.ret(ret)
                        .line("self.inner.server_streaming(request, path)");

                    format!("grpc::unary::Once<{}>", input_type)
                }
                (true, false) => {
                    let ret = format!(
                        "grpc::client_streaming::ResponseFuture<{}, T::Future, T::ResponseBody>",
                        output_type
                    );

                    request.generic("B");

                    func.generic("B")
                        .bound("B", &format!("futures::Stream<Item = {}>", input_type,))
                        .ret(ret)
                        .line("self.inner.client_streaming(request, path)");

                    "B".to_string()
                }
                (true, true) => {
                    let ret = format!(
                        "grpc::streaming::ResponseFuture<{}, T::Future>",
                        output_type
                    );

                    request.generic("B");

                    func.generic("B")
                        .bound("B", &format!("futures::Stream<Item = {}>", input_type,))
                        .ret(ret)
                        .line("self.inner.streaming(request, path)");

                    "B".to_string()
                }
            };

            func.arg("request", request)
                .bound(&req_body, "grpc::Encodable<R>");
        }
    }
}
