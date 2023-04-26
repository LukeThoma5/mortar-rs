use std::{
    fmt::Write,
};

pub struct AnonymousPropertyValue {
    pub name: String,
    pub value: String,
}

pub struct AnonymousObjectDefinition {
    properties: Vec<AnonymousPropertyValue>,
}

impl AnonymousObjectDefinition {
    pub fn new() -> Self {
        AnonymousObjectDefinition {
            properties: Vec::new(),
        }
    }

    pub fn add_property(&mut self, param_property: AnonymousPropertyValue) {
        self.properties.push(param_property);
    }

    pub fn write_structure_to_file(&self, file: &mut String) -> anyhow::Result<()> {
        write!(file, "{{\n")?;

        for prop in &self.properties {
            write!(file, "{}: {},\n", prop.name, prop.value)?;
        }

        write!(file, "\n}}")?;

        Ok(())
    }
}
