use crate::parser::MortarTypeReference;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum MortarType {
    I32,
    Str,
    F32,
    Bool,
    Uuid,
    DateTime,
    Any,
    Array(Box<MortarType>),
    Reference(MortarTypeReference),
}
