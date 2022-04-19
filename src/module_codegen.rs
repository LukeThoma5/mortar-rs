use crate::{
    mortar_type::MortarType,
    parser::{
        EndpointType, MortarConcreteType, MortarConcreteTypeType, MortarEndpoint, MortarModule,
        MortarTypeReference,
    },
    string_tools::{ensure_camel_case, ensure_pascal_case},
};
use anyhow::{anyhow, Context, Error};
use std::{
    collections::{HashMap, HashSet},
    fmt::Write,
    rc::Rc,
};

use itertools::Itertools;

// use lazysort::SortedBy;

#[derive(Debug)]
pub struct ImportTracker {
    imports: HashSet<MortarType>,
}

fn get_concrete_type_path(t: &MortarConcreteType) -> String {
    let path = format!("mortar/{}", t.namespace.clone().join("/"));

    path
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

            let path = get_concrete_type_path(t);

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
        write!(file, "{{\n")?;

        for prop in &self.properties {
            prop.write_property_to_file(file, resolver)?;
        }

        write!(file, "\n}}")?;

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

fn make_action_request(
    imports: &mut ImportTracker,
    endpoint: &MortarEndpoint,
) -> anyhow::Result<AnonymousTypeDefinition> {
    let mut object_def = AnonymousTypeDefinition::new();

    if !endpoint.route_params.is_empty() {
        let mut route_params = AnonymousTypeDefinition::new();

        for route_param in &endpoint.route_params {
            let mut key = route_param.name.clone();
            ensure_camel_case(&mut key);
            imports.track_type(route_param.schema.clone());
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
            imports.track_type(query_param.schema.clone());
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
        imports.track_type(req.clone());
        object_def.add_property(TypeDefinitionProperty {
            name: "request".to_owned(),
            optional: false,
            prop_type: MortarTypeOrAnon::Type(req.clone()),
        });
    }

    Ok(object_def)
}

fn create_action_request_name(endpoint: &MortarEndpoint) -> String {
    let mut action_request_name = endpoint.action_name.clone();
    ensure_pascal_case(&mut action_request_name);
    action_request_name.push_str("ActionRequest");

    action_request_name
}

pub fn generate_actions_file(
    module: MortarModule,
    resolver: Rc<SchemaResolver>,
) -> anyhow::Result<String> {
    let mut imports = ImportTracker::new();
    let mut file = String::with_capacity(1024 * 1024);

    // todo drain rather than clone
    for endpoint in module
        .endpoints
        .clone()
        .into_iter()
        .sorted_by(|a, b| a.path.cmp(&b.path))
    {
        let formatted_route = endpoint
            .path
            // Remove the initial slash
            .as_str()[1..]
            .replace("{", "${routeParams.");

        let mut action_request = make_action_request(&mut imports, &endpoint)?;

        let action_type = format!("{}/{}", &module.name, endpoint.action_name);

        let return_type = endpoint
            .response
            .as_ref()
            .map(|r| {
                imports.track_type(r.clone());
                r.to_type_string(&resolver)
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
            name: create_action_request_name(&endpoint),
        };

        if !action_request.is_empty() {
            action_request.write_structure_to_file(&mut file, &resolver)?;
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
                    "apiGet<{}, \"{}\">(\"{}\", `{}`,",
                    return_type, &action_type, &action_type, formatted_route
                )?;
                write_optional(&mut file, "queryParams")?;
                write_optional(&mut file, "options")?;
                writeln!(file, ");")?;
            }
            _ => {
                writeln!(
                    file,
                    "api{}<{}, \"{}\">(\"{}\",`{}`,",
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

    imports
        .write_imports(&mut import_header, &resolver)
        .context("Failed to generate imports")?;

    let default_imports =
        "import {apiGet, apiPost, apiDelete, apiPut, ApiRequestOptions} from '@redriver/cinnamon';";

    let file = format!(
        "// Auto Generated file, do not modify\n{}\n{}\n\n{}\n",
        default_imports, import_header, file
    );

    return Ok(file);
}

pub fn create_type_files(
    types: Vec<MortarConcreteType>,
    resolver: &SchemaResolver,
) -> anyhow::Result<Vec<TypeFileCollection>> {
    let mut results = Vec::with_capacity(24);
    // use into group map
    let map = types
        .into_iter()
        .map(|t| (get_concrete_type_path(&t), t))
        .into_group_map();

    for (path, types) in map {
        let mut imports = ImportTracker::new();

        let mut file = String::with_capacity(1024 * 1024);

        let mut handled_paged_view = false;

        for concrete in types {
            if concrete.generic_arguments.is_empty() {
                let named_definition = concrete_type_to_named_definition(concrete, &mut imports);

                named_definition.write_structure_to_file(&mut file, resolver)?;
                write!(file, "\n\n")?;
            } else if concrete.type_name == "PagedView" {
                if !handled_paged_view {
                    write!(
                        file,
                        "\nexport interface PagedView<T> {{
                        results: T[];
                        totalResults: number;
                    }}\n"
                    )?;
                    handled_paged_view = true;
                }
            } else {
                // TODO handle generics properly. Will require extra information from saffron to know what fields use the generic
                // as opposed to just happening to be the same as the generic.
                return Err(anyhow!(
                    "Interface includes generic type '{}'. Only PagedView is supported at present",
                    &concrete.type_name
                ));
            }
        }

        let mut import_header = String::with_capacity(10 * 1024);

        imports
            .write_imports(&mut import_header, &resolver)
            .context("Failed to generate imports")?;

        let file = format!(
            "// Auto Generated file, do not modify\n{}\n\n{}\n",
            import_header, file
        );

        results.push(TypeFileCollection { path, source: file })
    }

    Ok(results)
}

fn concrete_type_to_named_definition(
    concrete: MortarConcreteType,
    imports: &mut ImportTracker,
) -> NamedTypeDefinition {
    let MortarConcreteType {
        type_name,
        data,
        generic_arguments,
        ..
    } = concrete;
    let mut def = NamedTypeDefinition {
        name: type_name,
        def: AnonymousTypeDefinition::new(),
    };

    match data {
        MortarConcreteTypeType::Enum(_) => {
            // todo enums
        }
        MortarConcreteTypeType::Obj { properties } => {
            for (prop, mortar_type) in properties {
                imports.track_type(mortar_type.clone());
                def.def.add_property(TypeDefinitionProperty {
                    name: prop,
                    // Todo how to handle optional types
                    optional: false,
                    prop_type: MortarTypeOrAnon::Type(mortar_type),
                });
            }
        }
    };

    def
}

pub struct TypeFileCollection {
    pub source: String,
    pub path: String,
}
