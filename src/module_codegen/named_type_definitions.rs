use crate::module_codegen::anon_type_definition::AnonymousTypeDefinition;
use crate::parser::mortar_concrete_type::EnumElement;
use crate::schema_resolver::SchemaResolver;
use std::fmt::Write;
use crate::settings::Settings;

pub struct NamedTypeDefinition {
    pub name: String,
    pub def: AnonymousTypeDefinition,
}

pub struct WriteableTypeDefinition {
    pub name: String,
    pub def: NamedTypeDefinitionDefinition,
}

impl WriteableTypeDefinition {
    pub fn write_structure_to_file(
        &self,
        file: &mut String,
        resolver: &SchemaResolver,
        settings: &Settings,
    ) -> anyhow::Result<()> {
        match &self.def {
            NamedTypeDefinitionDefinition::Anon(def) => {
                write!(file, "export interface {} ", self.name)?;

                def.write_structure_to_file(file, resolver, settings)?;

                write!(file, ";\n")?;
            }
            NamedTypeDefinitionDefinition::Enum(variants) => {
                write!(file, "export const {} = {{\n", self.name)?;

                let mut any_variants_raw = false;

                for v in variants {
                    if let Some(ref raw) = v.raw_value {
                        any_variants_raw = true;
                        write!(file, "\"{}\": {},", &v.key, raw)?;
                    } else {
                        write!(file, "\"{}\": \"{}\",", &v.key, &v.key)?;
                    }
                }
                write!(file, "}} as const;\n")?;

                if any_variants_raw {
                    write!(
                        file,
                        "\nexport type {} = typeof {}[keyof typeof {}];\n",
                        self.name, self.name, self.name
                    )?;
                } else {
                    write!(
                        file,
                        "\nexport type {} = keyof typeof {};\n",
                        self.name, self.name
                    )?;
                }
            }
        }

        Ok(())
    }
}

impl NamedTypeDefinition {
    pub fn is_empty(&self) -> bool {
        self.def.properties.is_empty()
    }

    pub fn write_structure_to_file(
        &self,
        file: &mut String,
        resolver: &SchemaResolver,
        settings: &Settings,
    ) -> anyhow::Result<()> {
        write!(file, "export interface {} ", self.name)?;

        self.def.write_structure_to_file(file, resolver, settings)?;

        write!(file, ";\n")?;

        Ok(())
    }

    pub fn contains_property(&self, prop: &str) -> bool {
        self.def.properties.iter().any(|x| x.name == prop)
    }
}

pub enum NamedTypeDefinitionDefinition {
    Anon(AnonymousTypeDefinition),
    Enum(Vec<EnumElement>),
}
