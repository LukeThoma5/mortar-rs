mod swagger;

use anyhow::Context;
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

mod Parser;
mod settings;

use crate::swagger::SwaggerApi;
use crate::Parser::SwaggerParser;
use settings::Settings;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Settings::new().context("Failed to create settings")?;

    println!("Settings {:?}", settings);

    let swagger_api = SwaggerApi::new();

    let swagger = swagger_api
        .get_swagger_info(&settings.swagger_endpoint)
        .await?;

    let mut parser = SwaggerParser::new();

    parser
        .parse_swagger(swagger)
        .context("Failed to parse swagger")?;

    let modules = parser.into_modules();

    for module in modules {
        println!("{:?}\n\n", module);
    }

    Ok(())
}
