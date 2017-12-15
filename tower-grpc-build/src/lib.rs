extern crate codegen;
extern crate prost_build;

mod client;
mod server;

use std::io;
use std::cell::RefCell;
use std::fmt::Write;
use std::path::Path;
use std::rc::Rc;
use std::ascii::AsciiExt;

/// Code generation configuration
pub struct Config {
    prost: prost_build::Config,
    inner: Rc<RefCell<Inner>>,
}

struct Inner {
    build_client: bool,
    build_server: bool,
}

struct ServiceGenerator {
    client: client::ServiceGenerator,
    server: server::ServiceGenerator,
    inner: Rc<RefCell<Inner>>,
}

impl Config {
    /// Returns a new `Config` with pre-configured prost.
    ///
    /// You can tweak the configuration how the proto buffers are generated and use this config.
    pub fn from_prost(mut prost: prost_build::Config) -> Self {
        let inner = Rc::new(RefCell::new(Inner {
            // Enable client code gen by default
            build_client: true,

            // Disable server code gen by default
            build_server: false,
        }));

        // Set the service generator
        prost.service_generator(Box::new(ServiceGenerator {
            client: client::ServiceGenerator,
            server: server::ServiceGenerator,
            inner: inner.clone(),
        }));

        Config {
            prost,
            inner,
        }
    }

    /// Returns a new `Config` with default values.
    pub fn new() -> Self {
        Self::from_prost(prost_build::Config::new())
    }

    /// Enable gRPC client code generation
    pub fn enable_client(&mut self, enable: bool) -> &mut Self {
        self.inner.borrow_mut().build_client = enable;
        self
    }

    /// Enable gRPC server code generation
    pub fn enable_server(&mut self, enable: bool) -> &mut Self {
        self.inner.borrow_mut().build_server = enable;
        self
    }

    /// Generate code
    pub fn build<P>(&self, protos: &[P], includes: &[P]) -> io::Result<()>
    where P: AsRef<Path>,
    {
        self.prost.compile_protos(protos, includes)
    }
}

impl prost_build::ServiceGenerator for ServiceGenerator {
    fn generate(&self, service: prost_build::Service, buf: &mut String) {
        let inner = self.inner.borrow();

        if inner.build_client {
            // Add an extra new line to separate messages
            write!(buf, "\n").unwrap();

            self.client.generate(&service, buf).unwrap();
        }

        if inner.build_server {
            write!(buf, "\n").unwrap();
            self.server.generate(&service, buf).unwrap();
        }
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

fn super_import(ty: &str, level: usize) -> (String, &str) {
    let mut v: Vec<&str> = ty.split("::").collect();

    for _ in 0..level {
        v.insert(0, "super");
    }

    let last = v.pop().unwrap_or(ty);

    (v.join("::"), last)
}

fn unqualified(ty: &str) -> &str {
    ty.rsplit("::").next().unwrap_or(ty)
}
