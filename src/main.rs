mod formatter;
mod module_codegen;
mod mortar_type;
mod string_tools;
mod swagger;
use anyhow::Context;
use std::path::PathBuf;
use std::str::FromStr;
use std::{collections::HashMap, rc::Rc};

mod parser;
mod settings;

use crate::parser::SwaggerParser;
use crate::swagger::SwaggerApi;
use settings::Settings;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Settings::new().context("Failed to create settings")?;

    println!("Settings {:?}", settings);

    let swagger_api = SwaggerApi::new();

    let swagger = swagger_api
        .get_swagger_info(&settings.swagger_endpoint)
        .await?;

    let mut parser = SwaggerParser::new(swagger);

    parser.parse_swagger().context("Failed to parse swagger")?;

    // for module in parser.schemas {
    //     println!("{:?}\n\n", module);
    // }

    let formatter = formatter::Formatter::new();

    let SwaggerParser {
        modules, schemas, ..
    } = parser;

    let resolver = Rc::new(module_codegen::SchemaResolver::new(schemas));

    for (path, module) in modules.into_iter().take(1) {
        // println!("{:?}\n\n", module);
        let gen = module_codegen::ModuleCodeGenerator::new(module, resolver.clone());

        let bad_code = gen.generate()?;

        let result = formatter.format(&bad_code)?;

        println!("{}", result);
    }

    Ok(())
}
