use std::collections::HashMap;
use anyhow::{anyhow, Context};
// use crate::module_codegen;
use crate::parser::mortar_concrete_type::{MortarConcreteType, MortarConcreteTypeType};
use crate::parser::MortarTypeReference;

pub struct SchemaResolver {
    pub schemas: HashMap<MortarTypeReference, MortarConcreteType>,
}

impl SchemaResolver {
    pub fn new(schemas: HashMap<MortarTypeReference, MortarConcreteType>) -> SchemaResolver {
        SchemaResolver { schemas }
    }

    pub fn resolve_to_type_name(
        &self,
        type_ref: &MortarTypeReference,
    ) -> anyhow::Result<Option<String>> {
        self.schemas
            .get(type_ref)
            .map(|s| map_concrete_type(s, self))
            .transpose()
    }

    pub fn resolve_to_type<'a>(
        &'a self,
        type_ref: &MortarTypeReference,
    ) -> anyhow::Result<&'a MortarConcreteType> {
        self.schemas.get(type_ref)
        .with_context(|| anyhow!("Unable to find schema {:?}. Is this a nested generic type? Try adding [GenerateSchema(typeof(NestedType<InnerType>))] to the class", &type_ref))
        .with_context(|| format!("Failed to resolve type reference {:?}. Is the type a c# built-in or generic? Maybe an issue with MortarType::from_generic", &type_ref))
    }

    pub fn is_type_enum(&self, type_ref: &MortarTypeReference) -> anyhow::Result<bool> {
        let concrete = self.resolve_to_type(type_ref)?;
        let is_enum = match &concrete.data {
            MortarConcreteTypeType::Enum(_) => true,
            _ => false,
        };

        Ok(is_enum)
    }
}

fn map_concrete_type(t: &MortarConcreteType, resolver: &SchemaResolver) -> anyhow::Result<String> {
    let mut type_name = t.type_name.clone();

    if let Some(generics) = &t.generics {
        type_name.push_str("<");

        let len = generics.generic_arguments.len();

        for (index, generic_arg) in generics.generic_arguments.iter().enumerate() {
            let generic_type_name = generic_arg.to_type_string(resolver)?;
            type_name.push_str(&generic_type_name);
            if index + 1 < len {
                type_name.push_str(", ");
            }
        }

        type_name.push_str(">");
    }

    // if its an enum.
    // if let MortarConcreteTypeType::Enum(_) = t.data {
    //     type_name = format!("(keyof typeof {})", type_name);
    // }

    Ok(type_name)
}
