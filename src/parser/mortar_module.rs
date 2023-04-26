use crate::parser::mortar_type::MortarType;
use crate::parser::MortarEndpoint;

#[derive(Debug)]
pub struct MortarModule {
    pub name: String,
    pub endpoints: Vec<MortarEndpoint>,
    pub requests: Vec<MortarType>,
    pub responses: Vec<MortarType>,
}
