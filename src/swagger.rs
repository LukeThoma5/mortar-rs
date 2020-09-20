use anyhow::Context;

use serde::Deserialize;
use tokio::prelude::*;
use std::collections::{HashMap, BTreeMap};

#[derive(Deserialize, Debug)]
pub struct Swagger {
    #[serde(rename = "openapi")]
    pub open_api: String,
    pub info: HashMap<String, String>,
    pub paths: HashMap<String, SwaggerPath>,
    pub components: SwaggerComponents
}

#[derive(Deserialize, Debug)]
pub struct SwaggerComponents {
    schemas: BTreeMap<String, serde_json::Value>
}

#[derive(Deserialize, Debug)]
pub struct SwaggerSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    #[serde(flatten)]
    pub fields: BTreeMap<String, serde_json::Value>
}

#[derive(Deserialize, Debug)]
pub struct SwaggerPath {
    pub post: Option<SwaggerEndpoint>,
    pub put: Option<SwaggerEndpoint>,
    pub get: Option<SwaggerEndpoint>,
    pub delete: Option<SwaggerEndpoint>
}

#[derive(Deserialize, Debug)]
pub struct SwaggerEndpoint {
    pub tags: Vec<String>,
    pub description: Option<String>,
    #[serde(flatten)]
    pub fields: BTreeMap<String, serde_json::Value>
}

pub struct SwaggerApi {
    client: reqwest::Client
}

impl SwaggerApi {
    pub fn new() -> Self {
        SwaggerApi {
            client: reqwest::Client::new()
        }
    }

    pub async fn get_swagger_info(&self, endpoint: &str) -> anyhow::Result<Swagger> {

        let response = self.client
            .get(endpoint)
            .send()
            .await
            .context("Api call to gallery failed")?;


        let result = response.json::<Swagger>().await?;

        Ok(result)
    }
}