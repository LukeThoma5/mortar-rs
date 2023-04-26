use crate::parser::mortar_type::MortarType;

#[derive(Debug, Clone, Copy)]
pub enum EndpointType {
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Debug, Clone)]
pub struct MortarEndpoint {
    pub endpoint_type: EndpointType,
    pub path: String,
    pub route_params: Vec<MortarParam>,
    pub query_params: Vec<MortarParam>,
    pub form_params: Vec<MortarParam>,
    pub request: Option<MortarType>,
    pub response: Option<MortarType>,
    pub action_name: String,
}

#[derive(Debug, Clone)]
pub struct MortarParam {
    pub name: String,
    pub schema: MortarType,
}
