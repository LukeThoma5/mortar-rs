use crate::parser::MortarTypeReference;

#[derive(Debug, Clone)]
pub enum MortarType {
    I32,
    Str,
    F32,
    Bool,
    Uuid,
    DateTime,
    Array(Box<MortarType>),
    Reference(MortarTypeReference),
}
