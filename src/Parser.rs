use crate::swagger::{Swagger, SwaggerEndpoint};
use anyhow::Result;
use anyhow::{anyhow, Context};
use serde::de::value;
use std::collections::BTreeMap;

#[derive(Debug)]
pub struct MortarModule {
    pub name: String,
    pub endpoints: Vec<MortarEndpoint>,
    pub requests: Vec<MortarType>,
    pub responses: Vec<MortarType>,
}

#[derive(Debug)]
pub enum EndpointType {
    Get,
    Post,
    Put,
    Delete,
}

#[derive(Debug, Clone)]
pub enum MortarTypeReference {
    I32,
    Str,
    F32,
    Bool,
    Uuid,
    DateTime,
    Array(Box<MortarTypeReference>),
    Reference(String),
}

impl MortarTypeReference {
    pub fn new(reference: String) -> Self {
        Self::Reference(reference)
    }

    pub fn from_json(value: &serde_json::Value) -> Self {
        if let Some(v) = value.get("$ref") {
            Self::new(v.as_str().unwrap().to_owned())
        } else {
            match (
                value.get("type").and_then(|x| x.as_str()),
                value.get("format").and_then(|x| x.as_str()),
            ) {
                (Some("date-time"), _) => Self::DateTime,
                (_, Some("int32")) => Self::I32,
                (Some("boolean"), _) => Self::Bool,
                (Some("float"), _) => Self::F32,
                (_, Some("uuid")) => Self::Uuid,
                (Some("string"), _) => Self::Str,
                (Some("array"), _) => {
                    let items = value.get("items").expect("Array doesn't specify items");

                    let items = Self::from_json(items);

                    Self::Array(Box::new(items))
                }
                t => panic!("Unexpected schema type {:?}", t),
            }
        }
    }
}

#[derive(Debug)]
pub struct MortarEndpoint {
    pub endpoint_type: EndpointType,
    pub path: String,
    pub route_params: Vec<MortarParam>,
    pub query_params: Vec<MortarParam>,
    pub request: Option<MortarTypeReference>,
    pub response: Option<MortarTypeReference>,
    pub action_name: String,
}

#[derive(Debug)]
pub struct MortarParam {
    pub name: String,
    pub schema: MortarTypeReference,
}

#[derive(Debug)]
pub struct MortarType {}

pub struct SwaggerParser {
    modules: BTreeMap<String, MortarModule>,
}

impl SwaggerParser {
    pub fn new() -> Self {
        Self {
            modules: BTreeMap::new(),
        }
    }

    pub fn into_modules(self) -> Vec<MortarModule> {
        self.modules.into_iter().map(|(_, module)| module).collect()
    }

    pub fn parse_swagger(&mut self, swagger: Swagger) -> Result<()> {
        let Swagger { paths, .. } = swagger;

        for (endpoint_path, path) in paths {
            self.parse_endpoint(&endpoint_path, path.get, EndpointType::Get)?;
            self.parse_endpoint(&endpoint_path, path.post, EndpointType::Post)?;
            self.parse_endpoint(&endpoint_path, path.put, EndpointType::Put)?;
            self.parse_endpoint(&endpoint_path, path.delete, EndpointType::Delete)?;
        }

        Ok(())
    }

    fn parse_endpoint(
        &mut self,
        endpoint_path: &str,
        endpoint: Option<SwaggerEndpoint>,
        endpoint_type: EndpointType,
    ) -> Result<()> {
        let endpoint = match endpoint {
            Some(i) => i,
            None => return Ok(()),
        };

        let SwaggerEndpoint {
            mortar,
            description,
            mut fields,
            tags,
        } = endpoint;

        let mortar = mortar.ok_or(anyhow!("Endpoint doesn't have mortar extensions"))?;

        let module = match self.modules.get_mut(&mortar.action_group) {
            Some(t) => t,
            None => {
                let module = MortarModule {
                    name: mortar.action_group.clone(),
                    endpoints: Vec::new(),
                    requests: Vec::new(),
                    responses: Vec::new(),
                };

                self.modules.insert(mortar.action_group.clone(), module);
                self.modules
                    .get_mut(&mortar.action_group)
                    .expect("Failed to lookup just added module")
            }
        };

        let response = fields
            .get("responses")
            .and_then(|v| v.get("200"))
            .and_then(|v| v.get("content"))
            .and_then(|v| v.get("application/json"))
            .and_then(|v| v.get("schema"))
            .map(|v| MortarTypeReference::from_json(v));

        let request = fields
            .get("requestBody")
            .and_then(|v| v.get("content"))
            .and_then(|v| v.get("application/json"))
            .and_then(|v| v.get("schema"))
            .map(|v| MortarTypeReference::from_json(v));

        let mut mortar_endpoint = MortarEndpoint {
            path: endpoint_path.to_owned(),
            endpoint_type,
            response,
            request,
            query_params: vec![],
            route_params: vec![],
            action_name: mortar.action_name,
        };

        if let Some(parameters) = fields.get("parameters").and_then(|v| v.as_array()) {
            for param in parameters {
                let schema = param
                    .get("schema")
                    .map(|v| MortarTypeReference::from_json(v))
                    .ok_or(anyhow!("param doesn't have schema"))?
                    .to_owned();

                let name = param
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow!("param doesn't have name"))?
                    .to_owned();

                let mortar_param = MortarParam { name, schema };

                match param.get("in").and_then(|v| v.as_str()) {
                    Some("query") => {
                        mortar_endpoint.query_params.push(mortar_param);
                    }
                    Some("path") => {
                        mortar_endpoint.route_params.push(mortar_param);
                    }
                    a => Err(anyhow!("unknown param location {:?}", a))?,
                };
            }
        }

        // TODO parse the interfaces we will need
        // TODO figure out how to get a good name for the action creator (e.g. the endpoint name?)
        module.endpoints.push(mortar_endpoint);

        Ok(())
    }
}
