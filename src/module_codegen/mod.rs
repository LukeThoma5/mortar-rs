use crate::{
    parser::{mortar_module, MortarTypeReference},
    string_tools::{ensure_camel_case, ensure_pascal_case},
};
use anyhow::{anyhow, Context, Error};
use std::{
    any::type_name,
    collections::{HashMap, HashSet},
    fmt::Write,
    rc::Rc,
};

use crate::parser::endpoint::{EndpointType, MortarEndpoint, MortarParam};
use crate::parser::mortar_concrete_type::{
    EnumElement, GenericParameterInfoType, MortarConcreteType, MortarConcreteTypeType,
};
use crate::parser::mortar_module::MortarModule;
use crate::parser::mortar_type::MortarType;
use crate::schema_resolver::SchemaResolver;
use anon_object_definition::{AnonymousObjectDefinition, AnonymousPropertyValue};
use anon_type_definition::{AnonymousTypeDefinition, TypeDefinitionProperty};
use import_tracker::ImportTracker;
use itertools::Itertools;
use named_type_definitions::{
    NamedTypeDefinition, NamedTypeDefinitionDefinition, WriteableTypeDefinition,
};

pub mod action_gen;
mod anon_object_definition;
mod anon_type_definition;
mod import_tracker;
mod named_type_definitions;
pub mod standalone_request_gen;
pub mod types_gen;

fn get_concrete_type_path(t: &MortarConcreteType) -> String {
    let path = format!("mortar/{}", t.namespace.clone().join("/"));

    path
}

pub enum MortarTypeOrAnon {
    Type(MortarType),
    Anon(AnonymousTypeDefinition),
    BlackBox(String),
}
