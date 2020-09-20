mod swagger;

use anyhow::{Context};
use std::path::PathBuf;
use std::str::FromStr;
use std::collections::HashMap;

mod settings;
use settings::Settings;
use crate::swagger::SwaggerApi;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Settings::new().context("Failed to create settings")?;

    println!("Settings {:?}", settings);

    let swagger_api = SwaggerApi::new();

    let swagger = swagger_api.get_swagger_info(&settings.swagger_endpoint).await?;

    println!("{:?}", swagger);

    Ok(())
}
