use std::collections::BTreeMap;
use anyhow::Result;
use crate::swagger::{Swagger, SwaggerEndpoint};
use std::ops::Deref;

#[derive(Debug)]
pub struct MortarModule
{
    pub name: String,
    pub endpoints: Vec<MortarEndpoint>,
    pub requests: Vec<MortarType>,
    pub responses: Vec<MortarType>
}

#[derive(Debug)]
pub enum EndpointType
{
    Get,
    Post,
    Put,
    Delete
}

#[derive(Debug, Clone)]
pub struct MortarTypeReference(pub String);

#[derive(Debug)]
pub struct MortarEndpoint
{
    pub endpoint_type: EndpointType,
    pub name: String,
    pub query_params: Option<MortarTypeReference>,
    pub request: Option<MortarTypeReference>,
    pub response: Option<MortarTypeReference>
}

#[derive(Debug)]
pub struct MortarType
{

}

pub struct SwaggerParser
{
    modules: BTreeMap<String, MortarModule>
}


impl SwaggerParser {
    pub fn new() -> Self {
        Self {
            modules: BTreeMap::new()
        }
    }

    pub fn into_modules(self) -> Vec<MortarModule> {
        self.modules.into_iter().map(|(_, module)| module).collect()
    }

    pub fn parse_swagger(&mut self, swagger: Swagger) -> Result<()> {
        for (endpoint_path, path) in swagger.paths.iter() {
            self.parse_endpoint(endpoint_path, &path.get, EndpointType::Get)?;
            self.parse_endpoint(endpoint_path, &path.post, EndpointType::Post)?;
            self.parse_endpoint(endpoint_path, &path.put, EndpointType::Put)?;
            self.parse_endpoint(endpoint_path, &path.delete, EndpointType::Delete)?;
        }

        Ok(())
    }

    fn parse_endpoint(&mut self, endpoint_path: &str, endpoint: &Option<SwaggerEndpoint>, endpoint_type: EndpointType) -> Result<()>
    {
        let endpoint = match endpoint {
            Some(i) => i,
            None => return Ok(())
        };

        let tag = match endpoint.tags.get(0) {
            Some(t) => t.to_owned(),
            _ => "Unknown".to_owned()
        };

        let module = match self.modules.get_mut(&tag) {
            Some(t) => t,
            None => {
                let module = MortarModule {
                    name: tag.clone(),
                    endpoints: Vec::new(),
                    requests: Vec::new(),
                    responses: Vec::new()
                };

                self.modules.insert(tag.clone(), module);
                self.modules.get_mut(&tag).expect("Failed to lookup just added module")
            }
        };

        let mortar_endpoint = MortarEndpoint {
            name: endpoint_path.to_owned(),
            endpoint_type,
            response: None,
            request: None,
            query_params: None
        };

        // TODO parse the interfaces we will need
        // TODO figure out how to get a good name for the action creator (e.g. the endpoint name?)
        module.endpoints.push(mortar_endpoint);

        Ok(())
    }
}