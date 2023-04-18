use crate::parser::MortarTypeReference;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum MortarType {
    I32,
    Str,
    FileLike,
    F32,
    Bool,
    Uuid,
    DateTime,
    Any,
    Array(Box<MortarType>),
    Reference(MortarTypeReference),
}
