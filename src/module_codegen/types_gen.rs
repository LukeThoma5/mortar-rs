use anyhow::{anyhow, Context};
use std::collections::HashSet;
use itertools::Itertools;
use crate::module_codegen;
use crate::module_codegen::anon_type_definition::{AnonymousTypeDefinition, TypeDefinitionProperty};
use crate::module_codegen::import_tracker::ImportTracker;
use crate::module_codegen::MortarTypeOrAnon;
use crate::module_codegen::named_type_definitions::{NamedTypeDefinitionDefinition, WriteableTypeDefinition};
use crate::parser::mortar_concrete_type::{GenericParameterInfoType, MortarConcreteType, MortarConcreteTypeType};
use crate::parser::mortar_type::MortarType;
use crate::parser::MortarTypeReference;
use crate::schema_resolver::SchemaResolver;
use std::{
    fmt::Write,
};

pub fn create_type_files(
    types: Vec<MortarConcreteType>,
    resolver: &SchemaResolver,
) -> anyhow::Result<Vec<TypeFileCollection>> {
    let mut results = Vec::with_capacity(24);
    // use into group map
    let map = types
        .into_iter()
        .map(|t| (module_codegen::get_concrete_type_path(&t), t))
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