use crate::{
    parser::{
        mortar_module,
        MortarTypeReference,
    },
    string_tools::{ensure_camel_case, ensure_pascal_case},
};
use anyhow::{anyhow, Context, Error};
use std::{
    any::type_name,
    collections::{HashMap, HashSet},
    fmt::Write,
    rc::Rc,
};

use itertools::Itertools;
use anon_object_definition::{AnonymousObjectDefinition, AnonymousPropertyValue};
use anon_type_definition::{AnonymousTypeDefinition, TypeDefinitionProperty};
use import_tracker::ImportTracker;
use crate::parser::endpoint::{EndpointType, MortarEndpoint, MortarParam};
use crate::parser::mortar_concrete_type::{EnumElement, GenericParameterInfoType, MortarConcreteType, MortarConcreteTypeType};
use crate::parser::mortar_module::MortarModule;
use crate::parser::mortar_type::MortarType;
use crate::schema_resolver::SchemaResolver;

mod import_tracker;
mod anon_type_definition;
mod anon_object_definition;

// use lazysort::SortedBy;

fn get_concrete_type_path(t: &MortarConcreteType) -> String {
    let path = format!("mortar/{}", t.namespace.clone().join("/"));

    path
}

pub enum NamedTypeDefinitionDefinition {
    Anon(AnonymousTypeDefinition),
    Enum(Vec<EnumElement>),
}

pub struct NamedTypeDefinition {
    pub name: String,
    pub def: AnonymousTypeDefinition,
}

pub struct WriteableTypeDefinition {
    pub name: String,
    pub def: NamedTypeDefinitionDefinition,
}

impl WriteableTypeDefinition {
    pub fn write_structure_to_file(
        &self,
        file: &mut String,
        resolver: &SchemaResolver,
    ) -> anyhow::Result<()> {
        match &self.def {
            NamedTypeDefinitionDefinition::Anon(def) => {
                write!(file, "export interface {} ", self.name)?;

                def.write_structure_to_file(file, resolver)?;

                write!(file, ";\n")?;
            }
            NamedTypeDefinitionDefinition::Enum(variants) => {
                write!(file, "export const {} = {{\n", self.name)?;

                let mut any_variants_raw = false;

                for v in variants {
                    if let Some(ref raw) = v.raw_value {
                        any_variants_raw = true;
                        write!(file, "\"{}\": {},", &v.key, raw)?;
                    } else {
                        write!(file, "\"{}\": \"{}\",", &v.key, &v.key)?;
                    }
                }
                write!(file, "}} as const;\n")?;

                if any_variants_raw {
                    write!(
                        file,
                        "\nexport type {} = typeof {}[keyof typeof {}];\n",
                        self.name, self.name, self.name
                    )?;
                } else {
                    write!(
                        file,
                        "\nexport type {} = keyof typeof {};\n",
                        self.name, self.name
                    )?;
                }
            }
        }

        Ok(())
    }
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

pub enum MortarTypeOrAnon {
    Type(MortarType),
    Anon(AnonymousTypeDefinition),
    BlackBox(String),
}

fn add_params(
    params: &Vec<MortarParam>,
    object_def: &mut AnonymousTypeDefinition,
    imports: &mut ImportTracker,
    prop_name: &str,
) -> anyhow::Result<()> {
    if !params.is_empty() {
        let mut route_params = AnonymousTypeDefinition::new();

        for route_param in params {
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
            name: prop_name.to_owned(),
            optional: false,
            prop_type: MortarTypeOrAnon::Anon(route_params),
        });
    }

    Ok(())
}

fn get_mapping_command(param: &MortarParam, resolver: &SchemaResolver) -> String {
    match &param.schema {
        MortarType::Reference(type_ref) => {
            if resolver
                .is_type_enum(type_ref)
                .expect("Form Data is for unexpected type")
            {
                // If its a simple enum, it should be appended to prevent inserting "'EnumVariant'"
                "'Append'"
            } else {
                "'JSON'"
            }
        }
        MortarType::Array(inner_type) => match inner_type.as_ref() {
            MortarType::FileLike => "'ArrayAppend'",
            _ => "'JSON'",
        },
        _ => "'Append'",
    }
    .to_owned()
}

fn make_mapping_commands(
    endpoint: &MortarEndpoint,
    resolver: &SchemaResolver,
) -> anyhow::Result<AnonymousObjectDefinition> {
    let mut form_commands = AnonymousObjectDefinition::new();

    for route_param in &endpoint.form_params {
        let mut key = route_param.name.clone();
        ensure_camel_case(&mut key);

        form_commands.add_property(AnonymousPropertyValue {
            name: key,
            value: get_mapping_command(route_param, resolver),
        });
    }

    Ok(form_commands)
}

fn make_action_request(
    imports: &mut ImportTracker,
    endpoint: &MortarEndpoint,
) -> anyhow::Result<(AnonymousTypeDefinition, Vec<NamedTypeDefinition>)> {
    let mut object_def = AnonymousTypeDefinition::new();

    let mut extra_types = vec![];

    add_params(
        &endpoint.route_params,
        &mut object_def,
        imports,
        "routeParams",
    )?;
    add_params(
        &endpoint.query_params,
        &mut object_def,
        imports,
        "queryParams",
    )?;

    if !endpoint.form_params.is_empty() {
        let mut form_params = AnonymousTypeDefinition::new();

        let form_request_name = create_action_request_name(&endpoint, "ActionFormData");

        for route_param in &endpoint.form_params {
            let mut key = route_param.name.clone();
            ensure_camel_case(&mut key);
            imports.track_type(route_param.schema.clone());
            form_params.add_property(TypeDefinitionProperty {
                name: key,
                optional: false,
                prop_type: MortarTypeOrAnon::Type(route_param.schema.clone()),
            });
        }

        object_def.add_property(TypeDefinitionProperty {
            name: "formParams".to_owned(),
            optional: false,
            prop_type: MortarTypeOrAnon::BlackBox(form_request_name.clone()),
        });

        object_def.add_property(TypeDefinitionProperty {
            name: "formTransform".to_owned(),
            optional: true,
            prop_type: MortarTypeOrAnon::BlackBox(format!(
                "(request: {}) => FormData",
                &form_request_name
            )),
        });

        extra_types.push(NamedTypeDefinition {
            name: form_request_name,
            def: form_params,
        })
    }

    if let Some(req) = &endpoint.request {
        imports.track_type(req.clone());
        object_def.add_property(TypeDefinitionProperty {
            name: "request".to_owned(),
            optional: false,
            prop_type: MortarTypeOrAnon::Type(req.clone()),
        });
    }

    Ok((object_def, extra_types))
}

fn create_action_request_name(endpoint: &MortarEndpoint, suffix: &str) -> String {
    let mut action_request_name = endpoint.action_name.clone();
    ensure_pascal_case(&mut action_request_name);
    action_request_name.push_str(suffix);

    action_request_name
}

pub fn generate_actions_file(
    module: MortarModule,
    resolver: Rc<SchemaResolver>,
) -> anyhow::Result<String> {
    let mut imports = ImportTracker::new();
    let mut file = String::with_capacity(1024 * 1024);

    // TODO create an ActionTypes object that tracks all the action_types for easier use in redux.
    // https://rrsoftware.slack.com/archives/DMZSQ9WMD/p1657096803475849

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

        let (mut action_request, extra_types) = make_action_request(&mut imports, &endpoint)?;

        let action_type = format!("{}/{}", &module.name, endpoint.action_name);

        let return_type = match endpoint.response.as_ref().map(|r| {
            imports.track_type(r.clone());
            r.to_type_string(&resolver)
        }) {
            None => "void".to_owned(),
            Some(x) => {
                x.with_context(|| format!("Failed to get return type of {}", &action_type))?
            }
        };

        action_request.add_property(TypeDefinitionProperty {
            name: "options".to_string(),
            optional: true,
            prop_type: MortarTypeOrAnon::BlackBox(format!(
                "Partial<ApiRequestOptions<{}, \"{}\">>",
                &return_type, &action_type
            )),
        });

        // no more mutating
        let action_request = NamedTypeDefinition {
            def: action_request,
            name: create_action_request_name(&endpoint, "ActionRequest"),
        };

        for extra in extra_types {
            if extra.is_empty() {
                continue;
            }

            extra.write_structure_to_file(&mut file, &resolver)?;
            writeln!(file, "\n")?;
        }

        if !action_request.is_empty() {
            action_request.write_structure_to_file(&mut file, &resolver)?;
            writeln!(file, "\n")?;
        }

        writeln!(file, "export const {} = makeAction((", endpoint.action_name)?;

        if !action_request.is_empty() {
            write!(file, "{{")?;
            for key in action_request.def.properties.iter().map(|p| &p.name) {
                write!(file, "{},\n", key)?;
            }

            write!(file, "}}:{}", &action_request.name)?;
        }

        if action_request.def.properties.iter().all(|t| t.optional) {
            // For get requests where there is no body, make sure you don't have to specify anything.
            write!(file, " = {{ }}")?;
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

                if action_request.contains_property("request") {
                    write!(file, "request,")?;
                } else if action_request.contains_property("formParams") {
                    write!(file, "(formTransform || makeFormData)(formParams,\n")?;
                    let commands = make_mapping_commands(&endpoint, &resolver)?;
                    commands.write_structure_to_file(&mut file)?;
                    write!(file, "),")?;
                } else {
                    write!(file, "undefined,")?;
                }

                if action_request.contains_property("queryParams")
                    && action_request.contains_property("options")
                {
                    // Where a delete endpoint etc make sure that query params that should have been route params are being used.
                    write!(file, "{{params: queryParams, ...options}},")?;
                } else if action_request.contains_property("formParams") {
                    if action_request.contains_property("options") {
                        write!(file, "{{contentType: null, ...options}},")?;
                    } else {
                        write!(file, "undefined,")?;
                    }
                } else {
                    write_optional(&mut file, "options")?;
                }
            }
        };

        writeln!(file, "), \"{}\");\n", &action_type)?;
    }

    let mut import_header = String::with_capacity(10 * 1024);

    imports
        .write_imports(&mut import_header, &resolver, None)
        .context("Failed to generate imports")?;

    let default_imports =
        "import {makeAction, makeFormData} from \"../lib\";\nimport {apiGet, apiPost, apiDelete, apiPut, ApiRequestOptions} from '@redriver/cinnamon-mui';";

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

        let mut handled_generic_types = HashSet::new();

        for concrete in types
            .into_iter()
            .sorted_by(|a, b| a.type_name.cmp(&b.type_name))
        {
            if concrete.generics.is_some() {
                if handled_generic_types.contains(&concrete.type_name) {
                    // Already handled this generic, don't do it again
                    continue;
                }

                handled_generic_types.insert(concrete.type_name.to_owned());
            }

            let named_definition =
                concrete_type_to_named_definition(concrete, &mut imports, resolver)?;

            named_definition.write_structure_to_file(&mut file, resolver)?;
            write!(file, "\n\n")?;
        }

        let mut import_header = String::with_capacity(10 * 1024);

        imports
            .write_imports(&mut import_header, &resolver, Some(&path))
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
    resolver: &SchemaResolver,
) -> anyhow::Result<WriteableTypeDefinition> {
    let MortarConcreteType {
        mut type_name,
        data,
        generics,
        ..
    } = concrete;

    let def = match data {
        MortarConcreteTypeType::Enum(variants) => NamedTypeDefinitionDefinition::Enum(variants),
        MortarConcreteTypeType::Obj { properties } => {
            let mut def = AnonymousTypeDefinition::new();
            for (prop, mortar_type) in properties {
                let mortar_type_for_track = mortar_type.clone();
                let mut prop_type = MortarTypeOrAnon::Type(mortar_type);

                if let Some(generics) = &generics {
                    if let Some(generic_position) = generics.generic_properties.get(&prop) {
                        let mut buffer = String::new();
                        write_nested_generic_name(
                            generic_position,
                            &mut buffer,
                            &mortar_type_for_track,
                            resolver,
                            imports,
                        )?;
                        prop_type = MortarTypeOrAnon::BlackBox(buffer)
                    } else {
                        // only track if not a generic prop
                        imports.track_type(mortar_type_for_track);
                    }
                } else {
                    // only track if not a generic prop
                    imports.track_type(mortar_type_for_track);
                }

                def.add_property(TypeDefinitionProperty {
                    name: prop,
                    // Todo how to handle optional types
                    optional: false,
                    prop_type,
                });
            }

            NamedTypeDefinitionDefinition::Anon(def)
        }
    };

    if let Some(generics) = generics {
        let len = generics.generic_arguments.len();

        type_name.push('<');
        for (generic_position, _) in generics.generic_arguments.iter().enumerate() {
            type_name.push_str(&format!("T{}", generic_position));
            if generic_position + 1 < len {
                type_name.push_str(", ");
            }
        }
        type_name.push('>');
    }

    Ok(WriteableTypeDefinition {
        name: type_name,
        def,
    })
}

pub fn write_nested_generic_name(
    info: &GenericParameterInfoType,
    file: &mut String,
    mortar_type: &MortarType,
    resolver: &SchemaResolver,
    imports: &mut ImportTracker,
) -> anyhow::Result<()> {
    let mut write_for_reference =
        |r: &MortarTypeReference, items: &Vec<GenericParameterInfoType>| -> anyhow::Result<()> {
            let t = resolver
                .resolve_to_type(r)
                .with_context(|| format!("Failed to resolve reference to a generic {:?}", r))?;
            write!(file, "{}", &t.type_name)?;
            let len = items.len();

            if let Some(generics) = t.generics.as_ref() {
                write!(file, "<")?;
                // let op = t.generics.as_ref();

                for (generic_position, (gen_arg, gen_arg_type)) in items
                    .iter()
                    .zip(generics.generic_arguments.iter())
                    .enumerate()
                {
                    write_nested_generic_name(gen_arg, file, gen_arg_type, resolver, imports)?;
                    if generic_position + 1 < len {
                        write!(file, ", ")?;
                    }
                }
                write!(file, ">")?;
            }

            Ok(())
        };

    match info {
        GenericParameterInfoType::GenericParamPosition(pos) => {
            write!(file, "T{}", pos)?;
        }
        GenericParameterInfoType::TerminalType(terminal_type) => {
            let type_name = terminal_type.to_type_string(resolver)?;
            write!(file, "{}", type_name)?;
            imports.track_type(terminal_type.clone());
        }
        GenericParameterInfoType::Generic(items) => match mortar_type {
            MortarType::Reference(r) => {
                write_for_reference(r, items)?;
            }
            MortarType::Array(_array_type) => match items.get(0) {
                Some(GenericParameterInfoType::GenericParamPosition(pos)) => {
                    write!(file, "T{}[]", pos)?;
                }
                Some(GenericParameterInfoType::TerminalType(terminal_type)) => {
                    let type_name = terminal_type.to_type_string(resolver)?;
                    write!(file, "{}[]", type_name)?;
                    imports.track_type(terminal_type.clone());
                }
                _ => Err(anyhow!("Generic provided for non generic array"))?,
            },
            _ => Err(anyhow!("Generics provided for a non reference type"))?,
        },
    }

    Ok(())
}

pub struct TypeFileCollection {
    pub source: String,
    pub path: String,
}
