use crate::module_codegen::MortarTypeOrAnon;
use crate::schema_resolver::SchemaResolver;

use std::{
    fmt::Write,
};
use crate::settings::Settings;

pub struct AnonymousTypeDefinition {
    pub properties: Vec<TypeDefinitionProperty>,
}

impl AnonymousTypeDefinition {
    pub fn new() -> Self {
        AnonymousTypeDefinition {
            properties: Vec::new(),
        }
    }

    pub fn add_property(&mut self, param_property: TypeDefinitionProperty) {
        self.properties.push(param_property);
    }

    pub fn write_structure_to_file(
        &self,
        file: &mut String,
        resolver: &SchemaResolver,
        settings: &Settings,
    ) -> anyhow::Result<()> {
        write!(file, "{{\n")?;

        for prop in &self.properties {
            prop.write_property_to_file(file, resolver, settings)?;
        }

        write!(file, "\n}}")?;

        // write!(file, "{{")

        Ok(())
    }
}

pub struct TypeDefinitionProperty {
    pub name: String,
    pub optional: bool,
    pub nullable: bool,
    pub prop_type: MortarTypeOrAnon,
}

impl TypeDefinitionProperty {
    pub fn write_property_to_file(
        &self,
        file: &mut String,
        resolver: &SchemaResolver,
        settings: &Settings,
    ) -> anyhow::Result<()> {
        write!(file, "{}", self.name)?;

        write!(file, "{}", if self.optional { "?: " } else { ": " })?;

        match &self.prop_type {
            MortarTypeOrAnon::BlackBox(s) => write!(file, "{}", s)?,
            MortarTypeOrAnon::Type(s) => write!(file, "{}", s.to_type_string(resolver)?)?,
            MortarTypeOrAnon::Anon(a) => a.write_structure_to_file(file, resolver, settings)?,
        };

        if self.nullable && settings.strict_or_null {
            write!(file, " | null")?;
        }

        write!(file, ";\n")?;

        Ok(())
    }
}
