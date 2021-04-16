use crate::{
    parser::{EndpointType, MortarConcreteType, MortarEndpoint, MortarModule, MortarTypeReference},
    string_tools::ensure_camel_case,
};
use anyhow::anyhow;
use serde_json::json;
use std::{collections::HashMap, fmt::Write, rc::Rc};

pub struct SchemaResolver {
    pub schemas: HashMap<MortarTypeReference, MortarConcreteType>,
}

fn map_concrete_type(t: &MortarConcreteType) -> String {
    let mut type_name = t.type_name.clone();

    if t.generic_arguments.len() > 0 {
        type_name.push_str("<");

        for generic_arg in &t.generic_arguments {
            type_name.push_str(&map_concrete_type(generic_arg));
        }

        type_name.push_str(">");
    }

    type_name
}

impl SchemaResolver {
    pub fn new(schemas: HashMap<MortarTypeReference, MortarConcreteType>) -> SchemaResolver {
        SchemaResolver { schemas }
    }

    pub fn resolve_to_type_name(&self, type_ref: &MortarTypeReference) -> Option<String> {
        self.schemas.get(type_ref).map(map_concrete_type)
    }
}

pub struct ModuleCodeGenerator {
    module: MortarModule,
    resolver: Rc<SchemaResolver>,
}

impl ModuleCodeGenerator {
    pub fn new(module: MortarModule, resolver: Rc<SchemaResolver>) -> Self {
        Self { module, resolver }
    }

    fn make_action_request(
        &self,
        endpoint: &MortarEndpoint,
    ) -> anyhow::Result<serde_json::Map<String, serde_json::Value>> {
        let mut object_def = serde_json::Map::new();

        if !endpoint.route_params.is_empty() {
            let mut route_params = serde_json::Map::new();

            for route_param in &endpoint.route_params {
                let mut key = route_param.name.clone();
                ensure_camel_case(&mut key);
                let type_str = route_param.schema.to_type_string(&self.resolver);
                route_params.insert(key, serde_json::Value::String(type_str));
            }

            object_def.insert(
                "routeParams".to_owned(),
                serde_json::Value::Object(route_params),
            );
        }

        if !endpoint.query_params.is_empty() {
            let mut query_params = serde_json::Map::new();

            for query_param in &endpoint.query_params {
                let mut key = query_param.name.clone();
                ensure_camel_case(&mut key);
                let type_str = query_param.schema.to_type_string(&self.resolver);
                query_params.insert(key, serde_json::Value::String(type_str));
            }

            object_def.insert(
                "queryParams".to_owned(),
                serde_json::Value::Object(query_params),
            );
        }

        if let Some(req) = &endpoint.request {
            object_def.insert(
                "request".to_owned(),
                serde_json::Value::String(req.to_type_string(&self.resolver)),
            );
        }

        Ok(object_def)
    }

    fn write_structure_to_file(
        &self,
        file: &mut String,
        def: &serde_json::Value,
    ) -> anyhow::Result<()> {
        match def {
            serde_json::Value::String(s) => write!(file, "{}", s)?,
            // serde_json::Value::Bool(b) => write!(file, "\"{}\"", if *b { "true" } else { "false" })?,
            serde_json::Value::Object(o) => {
                write!(file, "{{")?;
                for (key, val) in o {
                    write!(file, "{}:", key)?;
                    self.write_structure_to_file(file, val)?;
                    write!(file, ";\n\n")?;
                }

                write!(file, "}}")?;
            }
            _ => Err(anyhow!("unhandled json type"))?,
        };

        // write!(file, "{{")

        Ok(())
    }

    pub fn generate(&self) -> anyhow::Result<String> {
        let mut file = String::with_capacity(1024 * 1024);

        for endpoint in &self.module.endpoints {
            // TODO make a request object for each request. One that has the specified route/query-params/request_body etc etc etc
            let formatted_route = endpoint.path.replace("{", "${routeParams.");

            let mut action_request = self.make_action_request(endpoint)?;
            let action_request_name = format!("{}ActionRequest", endpoint.action_name);
            let action_type = format!("{}/{}", &self.module.name, endpoint.action_name);

            let return_type = endpoint
                .response
                .as_ref()
                .map(|r| r.to_type_string(&self.resolver))
                .unwrap_or("void".to_owned());

            // TODO going to need to make action_request its own type so it can express optionability e.g. this should be options?: Partial<_>
            // TODO start tracking the imports required
            // TODO start the process of writing this out to disk
            // TODO start the code gen for request/view emittion
            // Reminder use the reco branch `feature/mortar`
            action_request.insert(
                "options".to_string(),
                serde_json::Value::String(format!(
                    "Partial<ApiRequestOptions<{}, \"{}\">>",
                    &return_type, &action_type
                )),
            );

            // no more mutating
            let action_request = action_request;

            if !action_request.is_empty() {
                write!(file, "export interface {}", &action_request_name)?;
                self.write_structure_to_file(
                    &mut file,
                    &serde_json::Value::Object(action_request.clone()),
                )?;

                writeln!(file, "\n")?;
            }

            writeln!(file, "export const {} = (", endpoint.action_name)?;

            if !action_request.is_empty() {
                write!(file, "{{")?;
                for key in action_request.keys() {
                    write!(file, "{},\n", key)?;
                }

                write!(file, "}}:{}", &action_request_name)?;
            }

            write!(file, ") => ")?;

            let write_optional = |file: &mut String, key: &str| -> anyhow::Result<()> {
                if action_request.contains_key(key) {
                    write!(file, "{},", key)?;
                } else {
                    write!(file, "undefined,")?;
                }

                Ok(())
            };

            match &endpoint.endpoint_type {
                EndpointType::Get => {
                    writeln!(
                        file,
                        "apiGet<{}>(\"{}\" as \"{}\", `{}`,",
                        return_type, &action_type, &action_type, formatted_route
                    )?;
                    write_optional(&mut file, "queryParams")?;
                    write_optional(&mut file, "options")?;
                    writeln!(file, ");")?;
                }
                _ => {
                    writeln!(
                        file,
                        "api{}<{}>(\"{}\" as \"{}\",`{}`,",
                        match &endpoint.endpoint_type {
                            EndpointType::Post => "Post",
                            EndpointType::Put => "Put",
                            EndpointType::Delete => "apiDelete",
                            _ => Err(anyhow!(
                                "Unknown endpoint type {:?}",
                                endpoint.endpoint_type
                            ))?,
                        },
                        &return_type,
                        &action_type,
                        &action_type,
                        &formatted_route
                    )?;
                    write_optional(&mut file, "request")?;
                    write_optional(&mut file, "options")?;
                    writeln!(file, ");")?;
                }
            };

            writeln!(file, "\n")?;
        }

        // println!("{}", &file);

        return Ok(file);
    }
}
