use anyhow::anyhow;
use crate::schema_resolver::SchemaResolver;
use crate::parser::MortarTypeReference;

impl MortarType {
    pub fn to_type_string(&self, resolver: &SchemaResolver) -> anyhow::Result<String> {
        // TODO make cow?
        let type_string = match self {
            MortarType::I32 | MortarType::F32 => "number".to_owned(),
            MortarType::Any => "any".to_owned(),
            MortarType::FileLike => "File".to_owned(),
            MortarType::Bool => "boolean".to_owned(),
            MortarType::Uuid | MortarType::DateTime | MortarType::Str => "string".to_owned(),
            MortarType::Array(mt) => format!("{}[]", mt.to_type_string(resolver)?),
            MortarType::Reference(r) => {
                let resolved = resolver.resolve_to_type_name(r);

                let resolved: Option<String> = resolved?;

                if let Some(resolved) = resolved {
                    resolved
                } else {
                    dbg!("Unable to find schema {:?}. Is this a nested generic type? Try adding [GenerateSchema(typeof(NestedType<InnerType>))] to the class", &r);
                    "any".to_owned()
                }
            }
        };

        Ok(type_string)
    }
}

impl MortarType {
    pub fn new(reference: String) -> Self {
        Self::Reference(MortarTypeReference(reference))
    }

    pub fn from_json(value: &serde_json::Value) -> Self {
        if let Some(v) = value.get("$ref") {
            Self::new(v.as_str().unwrap().to_owned())
        } else {
            if value.get("anyOf").is_some() {
                // TODO - allow for string | integer
                return Self::Any;
            }
            match (
                value.get("type").and_then(|x| x.as_str()),
                value.get("format").and_then(|x| x.as_str()),
            ) {
                (Some("date-time"), _) => Self::DateTime,
                (_, Some("int32") | Some("int64")) | (Some("integer"), _) => Self::I32,
                (Some("boolean"), _) => Self::Bool,
                (Some("float"), _) => Self::F32,
                // TODO properly handle float vs double vs decimal
                (Some("number"), _) => Self::F32,
                (_, Some("uuid")) => Self::Uuid,
                // binary file
                (Some("string"), Some("binary")) => Self::FileLike,
                (Some("string"), _) => Self::Str,
                // where we don't have any info e.g. its only typed as object in BE then give any type
                (Some("object"), _) if value.get("additionalProperties").is_none() => Self::Any,
                (Some("array"), _) => {
                    let items = value.get("items").expect("Array doesn't specify items");

                    let items = Self::from_json(items);

                    Self::Array(Box::new(items))
                }
                _ => {
                    if let Some(x) = value.get("additionalProperties") {
                        if x.is_object() {
                            return MortarType::from_json(x);
                        }
                    }

                    return match (value
                        .get("x-mtr")
                        .and_then(|x| x.as_object())
                        .and_then(|x| x.get("ne"))
                        .and_then(|x| x.as_str()))
                    {
                        Some("Object") => MortarType::Any,
                        Some("Dictionary") => MortarType::Any,
                        Some("JToken") => MortarType::Any,
                        x => {
                            dbg!("Unexpected schema type {:?}\n{:?}", value, x);
                            MortarType::Any
                        }
                    };
                }
            }
        }
    }

    pub fn from_generic(mut value: String) -> Self {
        // This is parsing it from the `SwaggerSchemaGenerator.MakeSchemaIdForType` e.g. a different format than for usual properties
        if let Some(mini) = value.strip_suffix("[]") {
            MortarType::Array(Box::new(MortarType::from_generic(mini.to_owned())))
        } else {
            // Remove nullable-ness. Todo encode null-ability in MortarType? Or a type that wraps it?
            value = value.replace("Nullable__", "");
            match value
                .as_str()
                .strip_prefix("#/components/schemas/")
                .expect("Generic type in invalid format")
            {
                "String" => Self::Str,
                "Boolean" => Self::Bool,
                "Object" => Self::Any,
                "DateTime" => Self::DateTime,
                "Guid" => Self::Uuid,
                "Decimal" | "Single" | "Double" => Self::F32,
                _ if value.as_str().contains("Int") => Self::I32,
                _ => MortarType::Reference(MortarTypeReference(value)),
            }
        }
    }
}

#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum MortarType {
    I32,
    Str,
    FileLike,
    F32,
    Bool,
    Uuid,
    DateTime,
    Any,
    Array(Box<MortarType>),
    Reference(MortarTypeReference),
}
