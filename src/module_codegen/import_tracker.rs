use itertools::Itertools;
use anyhow::Context;
use crate::module_codegen;
use crate::parser::mortar_concrete_type::MortarConcreteType;
use crate::parser::mortar_type::MortarType;
use crate::parser::MortarTypeReference;
use crate::schema_resolver::SchemaResolver;
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
};

#[derive(Debug)]
pub struct ImportTracker {
    imports: HashSet<MortarType>,
}

impl ImportTracker {
    pub fn new() -> Self {
        Self {
            imports: HashSet::new(),
        }
    }

    pub fn track_ref(&mut self, reference: MortarTypeReference) {
        self.imports.insert(MortarType::Reference(reference));
    }

    pub fn track_type(&mut self, reference: MortarType) {
        self.imports.insert(reference);
    }

    pub fn write_imports(
        &mut self,
        file: &mut String,
        resolver: &SchemaResolver,
        file_path: Option<&str>,
    ) -> anyhow::Result<()> {
        let import_map = self.emit_imports(resolver);

        for (path, imports) in import_map.into_iter().sorted_by(|a, b| a.0.cmp(&b.0)) {
            match file_path {
                // Don't import from yourself
                Some(p) if p == path => continue,
                _ => {}
            };

            write!(file, "import {{")?;
            for import in imports.into_iter().sorted_by(|a, b| a.cmp(&b)) {
                write!(file, "{},", import)?;
            }

            write!(file, "}} from \"{}\";\n", path)?;
        }

        Ok(())
    }

    pub fn emit_imports(&mut self, resolver: &SchemaResolver) -> HashMap<String, HashSet<String>> {
        let mut import_collection: HashMap<String, HashSet<String>> = HashMap::new();

        fn add_concrete_type(
            t: &MortarConcreteType,
            resolver: &SchemaResolver,
            imports: &mut HashMap<String, HashSet<String>>,
        ) {
            if let Some(generics) = &t.generics {
                for generic in &generics.generic_arguments {
                    add_type(generic, resolver, imports);
                }
            }

            let path = module_codegen::get_concrete_type_path(t);

            let map = imports.entry(path).or_default();

            map.insert(t.type_name.clone());
        }

        fn add_type(
            t: &MortarType,
            resolver: &SchemaResolver,
            imports: &mut HashMap<String, HashSet<String>>,
        ) {
            match t {
                MortarType::Array(arr_type) => add_type(&arr_type, resolver, imports),
                MortarType::Reference(ref reference) => {
                    let concrete_type = resolver
                        .resolve_to_type(reference)
                        .with_context(|| format!("Failed to resolve type reference {:?}. Is the type a c# built-in or generic? Maybe an issue with MortarType::from_generic", &reference))
                        .unwrap();
                    add_concrete_type(concrete_type, resolver, imports);
                }
                _ => {}
            }
        }

        for imported_type in &self.imports {
            add_type(imported_type, resolver, &mut import_collection);
        }

        import_collection
    }
}
