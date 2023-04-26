use std::collections::BTreeMap;
use crate::parser::mortar_type::MortarType;
use crate::parser::MortarTypeReference;

#[derive(Debug, Clone)]
pub struct EnumElement {
    pub key: String,
    pub raw_value: Option<String>,
}

#[derive(Debug, Clone)]
pub enum MortarConcreteTypeType {
    Enum(Vec<EnumElement>),
    Obj {
        properties: BTreeMap<String, MortarType>,
    },
}

#[derive(Debug, Clone)]
pub struct MortarConcreteType {
    pub type_ref: MortarTypeReference,
    pub namespace: Vec<String>,
    pub type_name: String,
    pub data: MortarConcreteTypeType,
    pub generics: Option<MortarGenericInfo>,
}

#[derive(Debug, Clone)]
pub enum GenericParameterInfoType {
    // Directly one of the top level's generic arguments
    GenericParamPosition(usize),
    // A type (generic or otherwise) that does not depend on the top level's generic arguments
    TerminalType(MortarType),
    // A generic type that has dependencies on the top level's generic arguments.
    // E.g. can be ManyTypes<string, int, T0, CustomType>
    Generic(Vec<GenericParameterInfoType>),
}

#[derive(Debug, Clone)]
pub struct MortarGenericInfo {
    pub generic_arguments: Vec<MortarType>,
    pub generic_properties: BTreeMap<String, GenericParameterInfoType>,
}

pub fn parse_param_info(val: &serde_json::Value) -> GenericParameterInfoType {
    if let Some(v) = val.as_u64() {
        return GenericParameterInfoType::GenericParamPosition(v as usize);
    }

    if let Some(v) = val.as_str() {
        return GenericParameterInfoType::TerminalType(MortarType::from_generic(v.to_owned()));
    }

    if let Some(v) = val.as_array() {
        let items = v.iter().map(parse_param_info).collect::<Vec<_>>();

        return GenericParameterInfoType::Generic(items);
    }

    GenericParameterInfoType::GenericParamPosition(99)
}
