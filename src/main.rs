mod swagger;

use anyhow::{Context};
use std::path::PathBuf;
use std::str::FromStr;
use std::collections::HashMap;

mod settings;
mod Parser;

use settings::Settings;
use crate::swagger::SwaggerApi;
use crate::Parser::SwaggerParser;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Settings::new().context("Failed to create settings")?;

    println!("Settings {:?}", settings);

    let swagger_api = SwaggerApi::new();

    let swagger = swagger_api.get_swagger_info(&settings.swagger_endpoint).await?;

    let mut parser = SwaggerParser::new();

    parser.parse_swagger(swagger);

    let modules = parser.into_modules();

    println!("{:?}", modules);

    Ok(())
}
