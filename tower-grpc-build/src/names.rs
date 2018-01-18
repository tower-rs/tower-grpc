use std::collections::HashMap;
use std::ops::Index;
use prost_build;
use codegen;

#[derive(Debug)]
pub struct Names {
    names: HashMap<String, Name>,
    level: usize,
}

#[derive(Debug)]
enum Name {
    NotImportable(String),
    Importable { name: String, path: String },
}

impl Names {
    pub fn new(level: usize) -> Self {
        Names {
            names: HashMap::new(),
            level,
        }
    }

    fn get(&self, name: &str) -> Option<&str> {
        self.names.get(name).map(Name::name)
    }

    pub fn for_method<'a>(&'a self, method: &'a prost_build::Method) -> (&'a str, &'a str) {
        let input_type = self.get(&method.input_type)
            .unwrap_or_else(|| {
                panic!("no entry in names for method.input_type='{}'!",
                    &method.input_type)
            });
        let output_type = self.get(&method.output_type)
            .unwrap_or_else(|| {
                panic!("no entry in names for method.output_type='{}'!",
                    &method.output_type)
            });
        (input_type, output_type)
    }

    pub fn with_message_types(&mut self,
                              service: &prost_build::Service)
                              -> &mut Self
    {

        for method in &service.methods {
            let input_type = Name::super_import(&method.input_type, self.level);
            self.names.insert(method.input_type.to_string(), input_type);

            let output_type = Name::super_import(&method.input_type, self.level);
            self.names.insert(method.output_type.to_string(), output_type);
        }

        self
    }

    pub fn import_into(&self, scope: &mut codegen::Scope) {
        for name in self.names.values() {
            if let Name::Importable { ref path, ref name } = *name {
                scope.import(path, name);
            }
        }
    }


impl<'a> From<&'a prost_build::Service> for Names {
    fn from(svc: &'a prost_build::Service) -> Self {
        let mut names = Names::new(1);
        names.with_message_types(svc);
        names
    }
}

impl Name {
    fn super_import(ty: &str, level: usize) -> Self {
        let mut v: Vec<&str> = ty.split("::").collect();
        for _ in 0..level {
            v.insert(0, "super");
        }

        // index of the first path element in `ty` that concretely names an item
        // (i.e., isn't super). a `use` statement may only end with a concrete name;
        // you can't `use super::super::super;`.
        let first_concrete_name = v.iter()
            .position(|s| s != &"super")
            .expect("got a type name that was just a string of \"::super\"s!");

        if first_concrete_name == v.len() - 1 {
            // the first concrete name in the path is the actual type name.
            // in this case, we can't come up with a reasonable `use` statement
            // for it, since if we import the name directly, it may clash with // names defined in this namespace, but we can't import its
            // containing namespace, because the containing namespace is
            // `super`, and `use` statements have to end in a concrete name.
            Name::NotImportable(v.join("::"))
        } else {

            let last = v[v.len()-2..].join("::");
            let path = v[..v.len()-2].join("::");

            Name::Importable {
                name: last,
                path,
            }
        }
    }

    /// Returns true if this `Name` can be imported with a `use` statement.
    ///
    /// Otherwise, it must be used in full.
    fn can_import(&self) -> bool {
        match *self {
            Name::Importable { .. } => true,
            _ => false,
        }
    }

    fn name(&self) -> &str {
        match *self {
            Name::Importable { ref name, .. } => name,
            Name::NotImportable(ref name ) => name,
        }
    }

    }
}