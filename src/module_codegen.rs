use crate::parser::{EndpointType, MortarConcreteType, MortarModule, MortarTypeReference};
use std::{collections::HashMap, fmt::Write, rc::Rc};

pub struct SchemaResolver {
    pub schemas: HashMap<MortarTypeReference, MortarConcreteType>,
}

impl SchemaResolver {
    pub fn new(schemas: HashMap<MortarTypeReference, MortarConcreteType>) -> SchemaResolver {
        SchemaResolver { schemas }
    }

    pub fn resolve_to_type_name(&self, type_ref: &MortarTypeReference) -> Option<String> {
        self.schemas.get(type_ref).map(|t| t.type_name.clone())
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

                    // TODO replace the path parms with their actual values

                    writeln!(file, "() => apiGet<{}>(`{}`);", return_type, endpoint.path)?;
                }
                _ => {
                    writeln!(file, "() => {{}};")?;
                }
            };

            writeln!(file, "\n")?;
        }

        return Ok(file);
    }
}
