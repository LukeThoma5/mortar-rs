use crate::parser::{EndpointType, MortarConcreteType, MortarModule, MortarTypeReference};
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

    pub fn generate(&self) -> anyhow::Result<String> {
        let mut file = String::with_capacity(1024 * 1024);

        for endpoint in &self.module.endpoints {
            writeln!(file, "export const {} = ", endpoint.action_name)?;

            match &endpoint.endpoint_type {
                EndpointType::Get => {
                    let return_type = endpoint
                        .response
                        .as_ref()
                        .map(|r| r.to_type_string(&self.resolver))
                        .unwrap_or("void".to_owned());

                    let mut request_type = endpoint.action_name.to_owned();
                    request_type.push_str("Request");

                    // TODO replace the path parms with their actual values

                    // TODO how to handle input params. Maybe we should have just the 1 object and its up to the caller to use a helper
                    // to squish listbuilderstate+load params into just load params. (so that it works for modals as well)

                    writeln!(
                        file,
                        "(data: GetRequest<{}>) =>
                    {{
                        const {{ routeParams }} = data;
                        return apiGet<{}>(`{}`);
                    }}
                    ",
                        request_type, return_type, endpoint.path
                    )?;

                    println!("{:?}", endpoint)
                }
                EndpointType::Post => {
                    let request_type = endpoint
                        .request
                        .as_ref()
                        .map(|r| r.to_type_string(&self.resolver))
                        .unwrap_or("void".to_owned());

                    let return_type = endpoint
                        .response
                        .as_ref()
                        .map(|r| r.to_type_string(&self.resolver))
                        .unwrap_or("void".to_owned());

                    writeln!(
                        file,
                        "(data: PostRequest<{}, {}>) =>
                    {{
                        const {{ routeParams, request }} = data;
                        return apiPost<{}>(`{}`, request);
                    }}
                    ",
                        &request_type, &return_type, &return_type, &endpoint.path
                    )?;

                    println!("{:?}", endpoint)
                }
                _ => {
                    writeln!(file, "() => {{}};")?;
                }
            };

            writeln!(file, "\n")?;
        }

        println!("{}", &file);

        return Ok(file);
    }
}
