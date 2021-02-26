mod formatter;
mod swagger;
use anyhow::Context;
use dprint_plugin_typescript::configuration::{NextControlFlowPosition, QuoteStyle};
use std::collections::HashMap;
use std::path::PathBuf;
use std::str::FromStr;

// mod Parser;
mod settings;

// use crate::swagger::SwaggerApi;
// use crate::Parser::SwaggerParser;
use settings::Settings;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Settings::new().context("Failed to create settings")?;

    println!("Settings {:?}", settings);

    let formatter = formatter::Formatter::new();

    let result = formatter.format("export const someActionName = (formData,          submitParams) => apiGet<string>(\"SOMELITERAL\")")?;

    println!("{}", result);

    // let swagger_api = SwaggerApi::new();

    // let swagger = swagger_api
    //     .get_swagger_info(&settings.swagger_endpoint)
    //     .await?;

    // let mut parser = SwaggerParser::new();

    // parser
    //     .parse_swagger(swagger)
    //     .context("Failed to parse swagger")?;

    // for module in parser.schemas {
    //     println!("{:?}\n\n", module);
    // }

    // for module in parser.modules {
    //     println!("{:?}\n\n", module);
    // }

    Ok(())
}
