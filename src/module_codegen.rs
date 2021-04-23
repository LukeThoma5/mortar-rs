extern crate lazysort;
use crate::{
    mortar_type::MortarType,
    parser::{EndpointType, MortarConcreteType, MortarEndpoint, MortarModule, MortarTypeReference},
    string_tools::{ensure_camel_case, ensure_pascal_case},
};
use anyhow::{anyhow, Context};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    rc::Rc,
};

use lazysort::SortedBy;

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
    ) -> anyhow::Result<()> {
        let import_map = self.emit_imports(resolver);

        for (path, imports) in import_map {
            write!(file, "import {{")?;
            for import in imports {
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
            imports: &mut HashMap<String, HashSet<String>>,
        ) {
            for generic in t.generic_arguments.clone() {
                add_concrete_type(&generic, imports);
            }

            let path = format!("~mortar/{}", t.namespace.clone().join("/"));

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
                    let concrete_type = resolver.resolve_to_type(reference).unwrap();
                    add_concrete_type(concrete_type, imports);
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

pub struct SchemaResolver {
    pub schemas: HashMap<MortarTypeReference, MortarConcreteType>,
}

fn map_concrete_type(t: &MortarConcreteType) -> String {
    let mut type_name = t.type_name.clone();

    if t.generic_arguments.len() > 0 {
        type_name.push_str("<");

        for generic_arg in &t.generic_arguments {
            type_name.push_str(&map_concrete_type(generic_arg));
        }

        type_name.push_str(">");
    }

    type_name
}

impl SchemaResolver {
    pub fn new(schemas: HashMap<MortarTypeReference, MortarConcreteType>) -> SchemaResolver {
        SchemaResolver { schemas }
    }

    pub fn resolve_to_type_name(&self, type_ref: &MortarTypeReference) -> Option<String> {
        self.schemas.get(type_ref).map(map_concrete_type)
    }

    pub fn resolve_to_type<'a>(
        &'a self,
        type_ref: &MortarTypeReference,
    ) -> Option<&'a MortarConcreteType> {
        self.schemas.get(type_ref)
    }
}

pub struct NamedTypeDefinition {
    pub name: String,
    pub def: AnonymousTypeDefinition,
}

impl NamedTypeDefinition {
    pub fn is_empty(&self) -> bool {
        self.def.properties.is_empty()
    }

    pub fn write_structure_to_file(
        &self,
        file: &mut String,
        resolver: &SchemaResolver,
    ) -> anyhow::Result<()> {
        write!(file, "export interface {} ", self.name)?;

        self.def.write_structure_to_file(file, resolver)?;

        write!(file, ";\n")?;

        Ok(())
    }

    pub fn contains_property(&self, prop: &str) -> bool {
        self.def.properties.iter().any(|x| x.name == prop)
    }
}

pub struct AnonymousTypeDefinition {
    properties: Vec<TypeDefinitionProperty>,
}

impl AnonymousTypeDefinition {
    pub fn new() -> Self {
        AnonymousTypeDefinition {
            properties: Vec::new(),
        }
    }

    pub fn add_property(&mut self, param_property: TypeDefinitionProperty) {
        self.properties.push(param_property);
    }

    pub fn write_structure_to_file(
        &self,
        file: &mut String,
        resolver: &SchemaResolver,
    ) -> anyhow::Result<()> {
        write!(file, "{{")?;

        for prop in &self.properties {
            prop.write_property_to_file(file, resolver)?;
        }

        write!(file, "}}")?;

        // write!(file, "{{")

        Ok(())
    }
}

pub struct TypeDefinitionProperty {
    pub name: String,
    pub optional: bool,
    pub prop_type: MortarTypeOrAnon,
}

impl TypeDefinitionProperty {
    pub fn write_property_to_file(
        &self,
        file: &mut String,
        resolver: &SchemaResolver,
    ) -> anyhow::Result<()> {
        write!(file, "{}", self.name)?;

        write!(file, "{}", if self.optional { "?: " } else { ": " })?;

        match &self.prop_type {
            MortarTypeOrAnon::BlackBox(s) => write!(file, "{}", s)?,
            MortarTypeOrAnon::Type(s) => write!(file, "{}", s.to_type_string(resolver))?,
            MortarTypeOrAnon::Anon(a) => a.write_structure_to_file(file, resolver)?,
        };

        write!(file, ";\n")?;

        Ok(())
    }
}

pub enum MortarTypeOrAnon {
    Type(MortarType),
    Anon(AnonymousTypeDefinition),
    BlackBox(String),
}

pub struct ModuleCodeGenerator {
    module: MortarModule,
    resolver: Rc<SchemaResolver>,
    imports: ImportTracker,
}

impl ModuleCodeGenerator {
    pub fn new(module: MortarModule, resolver: Rc<SchemaResolver>) -> Self {
        Self {
            module,
            resolver,
            imports: ImportTracker::new(),
        }
    }

    fn make_action_request(
        &mut self,
        endpoint: &MortarEndpoint,
    ) -> anyhow::Result<AnonymousTypeDefinition> {
        let mut object_def = AnonymousTypeDefinition::new();

        if !endpoint.route_params.is_empty() {
            let mut route_params = AnonymousTypeDefinition::new();

            for route_param in &endpoint.route_params {
                let mut key = route_param.name.clone();
                ensure_camel_case(&mut key);
                self.imports.track_type(route_param.schema.clone());
                route_params.add_property(TypeDefinitionProperty {
                    name: key,
                    optional: false,
                    prop_type: MortarTypeOrAnon::Type(route_param.schema.clone()),
                });
            }

            object_def.add_property(TypeDefinitionProperty {
                name: "routeParams".to_owned(),
                optional: false,
                prop_type: MortarTypeOrAnon::Anon(route_params),
            });
        }

        if !endpoint.query_params.is_empty() {
            let mut query_params = AnonymousTypeDefinition::new();

            for query_param in &endpoint.query_params {
                let mut key = query_param.name.clone();
                ensure_camel_case(&mut key);
                self.imports.track_type(query_param.schema.clone());
                query_params.add_property(TypeDefinitionProperty {
                    name: key,
                    optional: false,
                    prop_type: MortarTypeOrAnon::Type(query_param.schema.clone()),
                });
            }

            object_def.add_property(TypeDefinitionProperty {
                name: "queryParams".to_owned(),
                optional: false,
                prop_type: MortarTypeOrAnon::Anon(query_params),
            });
        }

        if let Some(req) = &endpoint.request {
            self.imports.track_type(req.clone());
            object_def.add_property(TypeDefinitionProperty {
                name: "request".to_owned(),
                optional: false,
                prop_type: MortarTypeOrAnon::Type(req.clone()),
            });
        }

        Ok(object_def)
    }

    pub fn create_action_request_name(endpoint: &MortarEndpoint) -> String {
        let mut action_request_name = endpoint.action_name.clone();
        ensure_pascal_case(&mut action_request_name);
        action_request_name.push_str("ActionRequest");

        action_request_name
    }

    pub fn generate(&mut self) -> anyhow::Result<String> {
        let mut file = String::with_capacity(1024 * 1024);

        // todo drain rather than clone
        for endpoint in self
            .module
            .endpoints
            .clone()
            .into_iter()
            .sorted_by(|a, b| a.path.cmp(&b.path))
        {
            let formatted_route = endpoint.path.replace("{", "${routeParams.");

            let mut action_request = self.make_action_request(&endpoint)?;

            let action_type = format!("{}/{}", &self.module.name, endpoint.action_name);

            let return_type = endpoint
                .response
                .as_ref()
                .map(|r| {
                    self.imports.track_type(r.clone());
                    r.to_type_string(&self.resolver)
                })
                .unwrap_or("void".to_owned());

            action_request.add_property(TypeDefinitionProperty {
                name: "options".to_string(),
                optional: true,
                prop_type: MortarTypeOrAnon::BlackBox(format!(
                    "Partial<ApiRequestOptions<{}, \"{}\">>",
                    &return_type, &action_type
                )),
            });

            // TODO start the process of writing this out to disk
            // TODO start the code gen for request/view emittion
            // Reminder use the reco branch `feature/mortar`

            // no more mutating
            let action_request = NamedTypeDefinition {
                def: action_request,
                name: ModuleCodeGenerator::create_action_request_name(&endpoint),
            };

            if !action_request.is_empty() {
                action_request.write_structure_to_file(&mut file, &self.resolver)?;
                writeln!(file, "\n")?;
            }

            writeln!(file, "export const {} = (", endpoint.action_name)?;

            if !action_request.is_empty() {
                write!(file, "{{")?;
                for key in action_request.def.properties.iter().map(|p| &p.name) {
                    write!(file, "{},\n", key)?;
                }

                write!(file, "}}:{}", &action_request.name)?;
            }

            write!(file, ") => ")?;

            let write_optional = |file: &mut String, key: &str| -> anyhow::Result<()> {
                if action_request.contains_property(key) {
                    write!(file, "{},", key)?;
                } else {
                    write!(file, "undefined,")?;
                }

                Ok(())
            };

            match &endpoint.endpoint_type {
                EndpointType::Get => {
                    writeln!(
                        file,
                        "apiGet<{}>(\"{}\" as \"{}\", `{}`,",
                        return_type, &action_type, &action_type, formatted_route
                    )?;
                    write_optional(&mut file, "queryParams")?;
                    write_optional(&mut file, "options")?;
                    writeln!(file, ");")?;
                }
                _ => {
                    writeln!(
                        file,
                        "api{}<{}>(\"{}\" as \"{}\",`{}`,",
                        match &endpoint.endpoint_type {
                            EndpointType::Post => "Post",
                            EndpointType::Put => "Put",
                            EndpointType::Delete => "Delete",
                            _ => Err(anyhow!(
                                "Unknown endpoint type {:?}",
                                endpoint.endpoint_type
                            ))?,
                        },
                        &return_type,
                        &action_type,
                        &action_type,
                        &formatted_route
                    )?;
                    write_optional(&mut file, "request")?;
                    write_optional(&mut file, "options")?;
                    writeln!(file, ");")?;
                }
            };

            writeln!(file, "\n")?;
        }

        let mut import_header = String::with_capacity(10 * 1024);

        // println!("{}", &file);

        self.imports
            .write_imports(&mut import_header, &self.resolver)
            .context("Failed to generate imports")?;

        let default_imports =
            "import {apiGet, apiPost, apiDelete, apiPut, ApiRequestOptions} from 'cinnamon';";

        let file = format!(
            "// Auto Generated file, do not modify\n{}\n{}\n\n{}\n",
            default_imports, import_header, file
        );

        return Ok(file);
    }
}
