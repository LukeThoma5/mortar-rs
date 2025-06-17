use std::rc::Rc;
use anyhow::{anyhow, Context};
use itertools::Itertools;
use crate::module_codegen::anon_object_definition::{AnonymousObjectDefinition, AnonymousPropertyValue};
use crate::module_codegen::anon_type_definition::{AnonymousTypeDefinition, TypeDefinitionProperty};
use crate::module_codegen::import_tracker::ImportTracker;
use crate::module_codegen::MortarTypeOrAnon;
use crate::module_codegen::named_type_definitions::NamedTypeDefinition;
use crate::parser::endpoint::{EndpointType, MortarEndpoint, MortarParam};
use crate::parser::mortar_module::MortarModule;
use crate::parser::mortar_type::MortarType;
use crate::schema_resolver::SchemaResolver;
use crate::string_tools::{ensure_camel_case, ensure_pascal_case};
use std::{
    fmt::Write,
};
use crate::module_codegen::standalone_request_gen::{create_request_object_from_params, get_request_base_name};
use crate::settings::Settings;


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

    let base_name = get_request_base_name(&endpoint);

    let mut add_params = |params: &Vec<MortarParam>, prop_name: &str, suffix: &str, nullable_props: bool| -> anyhow::Result<()> {
        if let Some(named) = create_request_object_from_params(
            &params,
            imports,
            &base_name,
            suffix,
            nullable_props
        )? {
            object_def.add_property(TypeDefinitionProperty {
                name: prop_name.to_owned(),
                optional: false,
                nullable: false,
                prop_type: MortarTypeOrAnon::BlackBox(named.name.clone()),
            });
            extra_types.push(named);
        }

        Ok(())
    };

    add_params(
        &endpoint.route_params,
        "routeParams",
        "RouteParams",
        false,
    )?;
    add_params(
        &endpoint.query_params,
        "queryParams",
        "QueryParams",
        true
    )?;

    if let Some(named) = create_request_object_from_params(
        &endpoint.form_params,
        imports,
        &base_name,
        "ActionFormData",
        false,
    )? {
        object_def.add_property(TypeDefinitionProperty {
            name:  "formParams".to_owned(),
            optional: false,
            nullable: false,
            prop_type: MortarTypeOrAnon::BlackBox(named.name.clone()),
        });

        object_def.add_property(TypeDefinitionProperty {
            name: "formTransform".to_owned(),
            optional: true,
            nullable: false,
            prop_type: MortarTypeOrAnon::BlackBox(format!(
                "(request: {}) => FormData",
                &named.name
            )),
        });

        extra_types.push(named);
    }

    if let Some(req) = &endpoint.request {
        imports.track_type(req.clone());
        object_def.add_property(TypeDefinitionProperty {
            name: "request".to_owned(),
            optional: false,
            nullable: false,
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
    settings: &Settings
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
            Some(Ok(x)) => x,
            Some(Err(x)) => {
                dbg!("Failed to get return type of {}\n{:?}", &action_type, x);
                "unknown".to_owned()
            }
        };

        action_request.add_property(TypeDefinitionProperty {
            name: "options".to_string(),
            optional: true,
            nullable: false,
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

            extra.write_structure_to_file(&mut file, &resolver, settings)?;
            writeln!(file, "\n")?;
        }

        if !action_request.is_empty() {
            action_request.write_structure_to_file(&mut file, &resolver, settings)?;
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



    let cinnamon_library: &str = settings.cinnamon_library.as_ref().map_or_else(|| "@redriver/cinnamon-mui", |f| f);
    let file = format!(
        "// Auto Generated file, do not modify
import {{makeAction, makeFormData}} from \"../lib\";\nimport {{apiGet, apiPost, apiDelete, apiPut, ApiRequestOptions}} from '{}';\n{}\n\n{}\n",
        cinnamon_library, import_header, file
    );

    return Ok(file);
}
