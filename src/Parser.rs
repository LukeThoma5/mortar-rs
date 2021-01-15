use crate::swagger::{Swagger, SwaggerEndpoint};
use anyhow::Result;
use anyhow::{anyhow, Context};
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
pub struct MortarTypeReference(pub String);

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

        // for field in fields.iter() {
        //     println!("{:?}\n\n", field);
        // }

        let response_ref = fields
            .get("responses")
            .and_then(|v| v.get("200"))
            .and_then(|v| v.get("content"))
            .and_then(|v| v.get("application/json"))
            .and_then(|v| v.get("schema"))
            .and_then(|v| v.get("$ref"))
            .and_then(|v| v.as_str())
            .map(|v| MortarTypeReference(v.to_owned()));

        let mut mortar_endpoint = MortarEndpoint {
            path: endpoint_path.to_owned(),
            endpoint_type,
            response: response_ref,
            request: None,
            query_params: vec![],
            route_params: vec![],
            action_name: mortar.action_name,
        };

        if let Some(parameters) = fields.get("parameters").and_then(|v| v.as_array())
        // .map(|v| v.iter().map(|v| {}));
        {
            for param in parameters {
                // TODO investigate the doesn't have schema
                let schema = param
                    .get("schema")
                    .and_then(|v| v.get("$ref"))
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow!("param doesn't have schema"))?
                    .to_owned();

                let name = param
                    .get("name")
                    .and_then(|v| v.as_str())
                    .ok_or(anyhow!("param doesn't have name"))?
                    .to_owned();

                let mortar_param = MortarParam {
                    name,
                    schema: MortarTypeReference(schema),
                };

                match param.get("in").and_then(|v| v.as_str()) {
                    Some("query") => {
                        mortar_endpoint.query_params.push(mortar_param);
                    }
                    Some("path") => {
                        mortar_endpoint.route_params.push(mortar_param);
                    }
                    a => Err(anyhow!("unknown param location"))?,
                };
            }
        }

        // if let Some(resp) = fields.remove("responses") {

        // }

        // panic!("End");

        // TODO parse the interfaces we will need
        // TODO figure out how to get a good name for the action creator (e.g. the endpoint name?)
        module.endpoints.push(mortar_endpoint);

        Ok(())
    }
}
