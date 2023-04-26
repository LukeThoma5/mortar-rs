use mortar_type::MortarType;
use crate::swagger::{Swagger, SwaggerEndpoint};
use crate::{
    swagger::{SwaggerComponents, SwaggerPath},
};
use anyhow::Result;
use anyhow::{anyhow, Context};
use serde::de::value;
use std::collections::{BTreeMap, HashMap};
use endpoint::{EndpointType, MortarEndpoint, MortarParam};
use mortar_concrete_type::{EnumElement, GenericParameterInfoType, MortarConcreteType, MortarConcreteTypeType, MortarGenericInfo};
use crate::schema_resolver::SchemaResolver;
use crate::parser::mortar_module::MortarModule;

pub(crate) mod mortar_module;
pub(crate) mod mortar_type;
pub(crate) mod endpoint;
pub(crate) mod mortar_concrete_type;

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub struct MortarTypeReference(pub String);

pub struct SwaggerParser {
    pub modules: BTreeMap<String, MortarModule>,
    pub schemas: HashMap<MortarTypeReference, MortarConcreteType>,
    pub paths: Option<HashMap<String, SwaggerPath>>,
    pub components: SwaggerComponents,
}

impl SwaggerParser {
    pub fn new(swagger: Swagger) -> Self {
        let Swagger {
            paths, components, ..
        } = swagger;
        Self {
            modules: BTreeMap::new(),
            schemas: HashMap::new(),
            paths: Some(paths),
            components,
        }
    }

    // pub fn into_modules(self) -> Vec<MortarModule> {
    //     self.modules.into_iter().map(|(_, module)| module).collect()
    // }

    pub fn parse_swagger(&mut self) -> Result<()> {
        let paths = self.paths.take().context("Paths already taken")?;
        // todo make this drain
        for (endpoint_path, path) in paths {
            self.parse_endpoint(&endpoint_path, path.get, EndpointType::Get)?;
            self.parse_endpoint(&endpoint_path, path.post, EndpointType::Post)?;
            self.parse_endpoint(&endpoint_path, path.put, EndpointType::Put)?;
            self.parse_endpoint(&endpoint_path, path.delete, EndpointType::Delete)?;
        }

        let keys = self
            .components
            .schemas
            .keys()
            .map(|k| k.clone())
            .collect::<Vec<String>>();
        for schema_fragment in keys {
            let reference: String = "#/components/schemas/".to_owned() + &schema_fragment;
            let type_ref = MortarTypeReference(reference);
            self.parse_schema(type_ref)
                .with_context(|| format!("Failed to parse schema {}", &schema_fragment))?;
        }

        Ok(())
    }

    fn parse_schema(&mut self, type_ref: MortarTypeReference) -> Result<MortarConcreteType> {
        let mini_type_ref = type_ref
            .0
            .strip_prefix("#/components/schemas/")
            .with_context(|| format!("Malformed mortar reference {}", &type_ref.0))?;

        let subject = self
            .components
            .schemas
            .get(mini_type_ref)
            .with_context(|| format!("Failed to get schema {}", &type_ref.0))?;

        let root = subject.get("x-mtr");

        let namespace = root
            .and_then(|v| v.get("ns"))
            .and_then(|v| v.as_array())
            .and_then(|v| {
                v.iter()
                    .map(|v| v.as_str().map(|s| s.to_owned()))
                    .collect::<Option<Vec<_>>>()
            })
            .ok_or(anyhow!("Type didn't include namespace"))?;

        let type_name = root
            .and_then(|v| v.get("ne"))
            .and_then(|v| v.as_str().map(|s| s.to_owned()))
            .ok_or(anyhow!("Type doesn't include name"))?;

        let data = match subject.get("type").and_then(|v| v.as_str()) {
            Some("object") => {
                let props = subject.get("properties");

                let mut properties = BTreeMap::new();

                if let Some(props) = props {
                    for (prop_name, opts) in props
                        .as_object()
                        .with_context(|| format!("properties is not a map - {}", &type_ref.0))?
                    {
                        // todo should map out the nullable pop
                        let type_name = MortarType::from_json(opts);
                        properties.insert(prop_name.clone(), type_name);
                    }
                }

                MortarConcreteTypeType::Obj { properties }
            }
            Some("string") => {
                let results = subject.get("enum").and_then(|v| v.as_array()).map(|o| {
                    o.iter()
                        .map(|value| value.as_str().unwrap().to_owned())
                        .map(|key| EnumElement {
                            key,
                            raw_value: None,
                        })
                        .collect::<Vec<EnumElement>>()
                });

                if let Some(values) = results {
                    MortarConcreteTypeType::Enum(values)
                } else {
                    Err(anyhow!("type is not an enum {:?}", subject))?
                }
            }
            Some("integer") => {
                let results = subject.get("enum").and_then(|v| v.as_array()).map(|o| {
                    o.iter()
                        .map(|value| value.as_f64().unwrap())
                        .map(|value| EnumElement {
                            key: value.to_string(),
                            raw_value: Some(value.to_string()),
                        })
                        .collect::<Vec<EnumElement>>()
                });

                if let Some(values) = results {
                    MortarConcreteTypeType::Enum(values)
                } else {
                    Err(anyhow!("type is not an int enum {:?}", subject))?
                }
            }
            a => Err(anyhow!("unknown type {:?}", a))?,
        };

        let mut generic_arguments = None;
        let mut generic_properties = None;

        if let Some(generic_args) = root
            .and_then(|v| v.get("ga"))
            .and_then(|v| v.as_array())
            .and_then(|v| {
                v.iter()
                    .map(|v| v.as_str().map(|s| s.to_owned()))
                    .collect::<Option<Vec<String>>>()
            })
        {
            generic_arguments = Some(
                generic_args
                    .into_iter()
                    .map(MortarType::from_generic)
                    .collect::<Vec<MortarType>>(),
            );
        }

        if let Some(generic_args) = root.and_then(|v| v.get("gm")).and_then(|v| v.as_object()) {
            generic_properties = Some(
                generic_args
                    .iter()
                    .map(|(prop, val)| (prop.to_owned(), mortar_concrete_type::parse_param_info(val)))
                    .collect::<BTreeMap<String, GenericParameterInfoType>>(),
            );
        }

        let generics = match (generic_arguments, generic_properties) {
            (Some(generic_arguments), Some(generic_properties)) if generic_arguments.len() > 0 => {
                Some(MortarGenericInfo {
                    generic_arguments,
                    generic_properties,
                })
            }
            _ => None,
        };

        let mut concrete = MortarConcreteType {
            namespace,
            type_name,
            type_ref,
            data,
            generics,
        };

        self.schemas
            .insert(concrete.type_ref.clone(), concrete.clone());

        Ok(concrete)
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
            .map(|v| MortarType::from_json(v));

        let request = fields
            .get("requestBody")
            .and_then(|v| v.get("content"))
            .and_then(|v| v.get("application/json"))
            .and_then(|v| v.get("schema"))
            .map(|v| MortarType::from_json(v));

        let mut mortar_endpoint = MortarEndpoint {
            path: endpoint_path.to_owned(),
            endpoint_type,
            response,
            request,
            query_params: vec![],
            route_params: vec![],
            form_params: vec![],
            action_name: mortar.action_name,
        };

        if let Some(parameters) = fields.get("parameters").and_then(|v| v.as_array()) {
            for param in parameters {
                let schema = param
                    .get("schema")
                    .map(|v| MortarType::from_json(v))
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
                    Some("header") => {
                        // skip content in headers (assurity)
                    }
                    a => Err(anyhow!("unknown param location {:?}", a))?,
                };
            }
        }

        if let Some(props) = fields
            .get("requestBody")
            .and_then(|v| v.get("content"))
            .and_then(|v| v.get("multipart/form-data"))
            .and_then(|v| v.get("schema"))
            .and_then(|v| v.get("properties"))
            .and_then(|v| v.as_object())
        {
            for (name, schema) in props {
                let schema = MortarType::from_json(schema);

                mortar_endpoint.form_params.push(MortarParam {
                    name: name.to_owned(),
                    schema,
                });
            }
        }

        module.endpoints.push(mortar_endpoint);

        Ok(())
    }
}
