extern crate codegen;
extern crate prost_build;
extern crate heck;

mod client;
mod server;

use std::io;
use std::path::Path;

use heck::CamelCase;

/// Code generation configuration
pub struct Config {
    prost: prost_build::Config,
    build_client: bool,
    build_server: bool,
}

struct ServiceGenerator {
    client: Option<client::ServiceGenerator>,
    server: Option<server::ServiceGenerator>,
    root_scope: codegen::Scope,
}

impl Config {
    /// Returns a new `Config` with pre-configured prost.
    ///
    /// You can tweak the configuration how the proto buffers are generated and use this config.
    pub fn from_prost(prost: prost_build::Config) -> Self {
        Config {
            prost,
            // Enable client code gen by default
            build_client: true,

            // Disable server code gen by default
            build_server: false,
        }
    }

    /// Returns a new `Config` with default values.
    pub fn new() -> Self {
        Self::from_prost(prost_build::Config::new())
    }

    /// Enable gRPC client code generation
    pub fn enable_client(&mut self, enable: bool) -> &mut Self {
        self.build_client = enable;
        self
    }

    /// Enable gRPC server code generation
    pub fn enable_server(&mut self, enable: bool) -> &mut Self {
        self.build_server = enable;
        self
    }

    /// Generate code
    pub fn build<P>(&mut self, protos: &[P], includes: &[P]) -> io::Result<()>
    where P: AsRef<Path>,
    {
        let client = if self.build_client {
            Some(client::ServiceGenerator)
        } else {
            None
        };
        let server = if self.build_server {
            Some(server::ServiceGenerator)
        } else {
            None
        };

        // Set or reset the service generator.
        self.prost.service_generator(Box::new(ServiceGenerator {
            client,
            server,
            root_scope: codegen::Scope::new(),
        }));

        self.prost.compile_protos(protos, includes)
    }
}

impl prost_build::ServiceGenerator for ServiceGenerator {

    fn generate(&mut self, service: prost_build::Service, _buf: &mut String) {
        // Note that neither this implementation of `generate` nor the
        // implementations for `client::ServiceGenerator` and
        // `server::ServiceGenerator` will actually output any code to the
        // buffer; all code is written out in the implementation of the
        // `ServiceGenerator::finalize` function on this type.
        if let Some(ref mut client_generator) = self.client {
            client_generator.generate(&service, &mut self.root_scope);
        }
        if let Some(ref mut server_generator) = self.server {
            server_generator.generate(&service, &mut self.root_scope);
        }
    }

    fn finalize(&mut self, buf: &mut String) {
        // Rather than outputting each service to the buffer as it's generated,
        // we generate the code in our root `codegen::Scope`, which is shared
        // between the generation of each service in the proto file. Unlike a
        // string, codegen provides us with something not unlike a simplified
        // Rust AST, making it easier for us to add new items to modules
        // defined by previous service generator invocations. As we want to
        // output the client and server implementations for each service in the
        // proto file in one `client` or `server` module in the generated code,
        // we wait until all the services have been generated before actually
        // outputting to the buffer.
        let mut fmt = codegen::Formatter::new(buf);
        self.root_scope
            .fmt(&mut fmt)
            .expect("formatting root scope failed!");
        // Reset the root scope so that the service generator is ready to
        // generate another file. this prevents the code generated for *this*
        // file being present in the next file.
        self.root_scope = codegen::Scope::new();
    }
}

// ===== utility fns =====

fn method_path(service: &prost_build::Service, method: &prost_build::Method) -> String {
    format!("\"/{}.{}/{}\"",
            service.package,
            service.proto_name,
            method.proto_name)
}

fn lower_name(name: &str) -> String {
    let mut ret = String::new();

    for (i, ch) in name.chars().enumerate() {
        if ch.is_uppercase() {
            if i != 0 {
                ret.push('_');
            }

            ret.push(ch.to_ascii_lowercase());
        } else {
            ret.push(ch);
        }
    }

    ret
}

fn is_imported_type(ty: &str) -> bool {
    ty.split("::")
        .map(|t| t == "super")
        .next().unwrap()
}

fn super_import(ty: &str, level: usize) -> (String, String) {
    let mut v: Vec<&str> = ty.split("::").collect();

    assert!(!is_imported_type(ty));

    for _ in 0..level {
        v.insert(0, "super");
    }

    let ty = v.pop().unwrap_or(ty);

    (v.join("::"), ty.to_string())
}

fn unqualified(ty: &str, level: usize) -> String {
    if !is_imported_type(ty) {
        return ty.to_string();
    }

    let mut v: Vec<&str> = ty.split("::").collect();

    for _ in 0..level {
        v.insert(0, "super");
    }

    v.join("::")
}


/// Converts a `snake_case` identifier to an `UpperCamel` case Rust type
/// identifier.
///
/// This is identical to the same [function] in `prost-build`, however, we
/// reproduce it here as `prost` does not publically export it.
///
/// We need this as `prost-build` will only give us the snake-case transformed
/// names for gRPC methods, but we need to use method names in types as well.
///
/// [function]: https://github.com/danburkert/prost/blob/d3b971ccd90df35d16069753d52289c0c85014e4/prost-build/src/ident.rs#L28-L38
fn to_upper_camel(s: &str) -> String {
    let mut ident = s.to_camel_case();

    // Add a trailing underscore if the identifier matches a Rust keyword
    // (https://doc.rust-lang.org/grammar.html#keywords).
    if ident == "Self" {
        ident.push('_');
    }
    ident
}
