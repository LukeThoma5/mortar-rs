use anyhow::Context;
use serde::{Deserialize, Deserializer};
use std::collections::{BTreeMap, HashMap};
use tokio::prelude::*;

#[derive(Deserialize, Debug)]
pub struct Swagger {
    #[serde(rename = "openapi")]
    pub open_api: String,
    pub info: HashMap<String, String>,
    pub paths: HashMap<String, SwaggerPath>,
    pub components: SwaggerComponents,
}

#[derive(Deserialize, Debug)]
pub struct SwaggerComponents {
    schemas: BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct SwaggerSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(flatten)]
    pub fields: BTreeMap<String, serde_json::Value>,
}

#[derive(Deserialize, Debug)]
pub struct SwaggerPath {
    pub post: Option<SwaggerEndpoint>,
    pub put: Option<SwaggerEndpoint>,
    pub get: Option<SwaggerEndpoint>,
    pub delete: Option<SwaggerEndpoint>,
}

#[derive(Deserialize, Debug)]
pub struct SwaggerEndpoint {
    pub tags: Vec<String>,
    pub description: Option<String>,
    #[serde(rename = "x-mtr")]
    pub mortar: Option<MortarEndpointMeta>,
    // #[serde(rename = "a"]
    // pub response: Option<String>,
    #[serde(flatten)]
    pub fields: BTreeMap<String, serde_json::Value>,
}

// fn extract<D>(deserializer: D) -> Result<String, D::Error>
// where
//     D: Deserializer,
// {
//     #[derive(Deserialize)]
//     struct Responses {
//         #[serde(rename = "200")]
//         pub ok: Response,
//     }

//     #[derive(Deserialize)]
//     struct Response {
//         #[serde(rename = "200")]
//         pub content: Content,
//     }

//     #[derive(Deserialize)]
//     struct Content {
//         #[serde(rename = "application/json")]
//         pub json: Types,
//     }
//     #[derive(Deserialize)]
//     struct Types {
//         pub schema: Schema,
//     }

//     #[derive(Deserialize)]
//     struct Schema {
//         pub jsonRef: SwagRef,
//     }

//     #[derive(Deserialize)]
//     struct SwagRef {
//         #[serde(rename = "$ref")]
//         pub swag_ref: String,
//     }

//     Responses::deserialize(deserializer).map(|a| a.ok.content.schema.json.c)
// }

#[derive(Deserialize, Debug)]
pub struct MortarEndpointMeta {
    #[serde(rename = "an")]
    pub action_name: String,
    #[serde(rename = "ag")]
    pub action_group: String,
}

pub struct SwaggerApi {
    client: reqwest::Client,
}

impl SwaggerApi {
    pub fn new() -> Self {
        SwaggerApi {
            client: reqwest::Client::new(),
        }
    }

    pub async fn get_swagger_info(&self, endpoint: &str) -> anyhow::Result<Swagger> {
        let response = self
            .client
            .get(endpoint)
            .send()
            .await
            .context("Api call to gallery failed")?;

        let result = response.json::<Swagger>().await?;

        Ok(result)
    }
}
