use crate::module_codegen::anon_object_definition::{
    AnonymousObjectDefinition, AnonymousPropertyValue,
};
use crate::module_codegen::anon_type_definition::{
    AnonymousTypeDefinition, TypeDefinitionProperty,
};
use crate::module_codegen::import_tracker::ImportTracker;
use crate::module_codegen::named_type_definitions::NamedTypeDefinition;
use crate::module_codegen::MortarTypeOrAnon;
use crate::parser::endpoint::{EndpointType, MortarEndpoint, MortarParam};
use crate::parser::mortar_module::MortarModule;
use crate::parser::mortar_type::MortarType;
use crate::schema_resolver::SchemaResolver;
use crate::string_tools::{ensure_camel_case, ensure_pascal_case};
use anyhow::{anyhow, Context};
use itertools::Itertools;
use std::fmt::Write;
use std::rc::Rc;

pub fn create_request_object_from_params(
    params: &Vec<MortarParam>,
    imports: &mut ImportTracker,
    name_base: &str,
    suffix: &str,
) -> anyhow::Result<Option<NamedTypeDefinition>> {
    if params.is_empty() {
        return Ok(None);
    }

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

    Ok(Some(NamedTypeDefinition {
        name: format!("{}{}", name_base, suffix),
        def: route_params,
    }))
}

pub fn get_request_base_name(endpoint: &MortarEndpoint) -> String {
    let mut action_request_name = endpoint.action_name.clone();
    ensure_pascal_case(&mut action_request_name);
    action_request_name
}

fn get_request_types(
    module: MortarModule,
    imports: &mut ImportTracker,
) -> anyhow::Result<(Vec<NamedTypeDefinition>, AnonymousObjectDefinition)> {
    let mut action_types = vec![];

    let mut paths = AnonymousObjectDefinition::new();

    for endpoint in module
        .endpoints
        .clone()
        .into_iter()
        .sorted_by(|a, b| a.path.cmp(&b.path))
    {
        let base_name = get_request_base_name(&endpoint);

        if let Some(named) = create_request_object_from_params(
            &endpoint.route_params,
            imports,
            &base_name,
            "RouteParams",
        )? {
            let formatted_route = endpoint
                .path
                [1..]
                .replace("{", "${routeParams.");
            paths.add_property(AnonymousPropertyValue {
                name: base_name.clone(),
                value: format!("(routeParams: {}) => `${{base_path}}{}`", &named.name, &formatted_route)
            });
            action_types.push(named);
        } else {
            paths.add_property(AnonymousPropertyValue {
                name: base_name.clone(),
                value: format!("`${{base_path}}{}`", &endpoint.path[1..])
            })
        }

        if let Some(named) = create_request_object_from_params(
            &endpoint.query_params,
            imports,
            &base_name,
            "QueryParams",
        )? {
            action_types.push(named);
        }
    }

    Ok((action_types, paths))
}

pub fn generate_requests_file(
    module: MortarModule,
    resolver: Rc<SchemaResolver>,
) -> anyhow::Result<String> {
    let mut imports = ImportTracker::new();
    let mut file = String::with_capacity(1024 * 1024);

    let (mut types, paths) = get_request_types(module, &mut imports)?;

    types.sort_by_cached_key(|t| t.name.clone());

    write!(file, "// Auto Generated file, do not modify\n")?;
    imports
        .write_imports(&mut file, &resolver, None)
        .context("Failed to generate imports")?;

    write!(file, "\nexport const PathFactory = (base_path: string) => (")?;
    paths.write_structure_to_file(&mut file)?;
    write!(file, ");\n")?;

    for t in types {
        t.write_structure_to_file(&mut file, &resolver)?;
        writeln!(file, "\n")?;
    }

    return Ok(file);
}
