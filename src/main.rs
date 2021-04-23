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

    let schemas_to_generate = schemas.values().cloned().collect::<Vec<_>>();

    let resolver = Rc::new(module_codegen::SchemaResolver::new(schemas));

    for (_path, module) in modules.into_iter().take(1) {
        // println!("{:?}\n\n", module);

        let bad_code = module_codegen::generate_actions_file(module, resolver.clone())?;

        let result = formatter
            .format(&bad_code)
            .with_context(|| format!("Failed to format the module: {}\n", _path));

        match result {
            Err(e) => {
                println!("{:?}\n{}", e, bad_code)
            }
            Ok(file) => {
                println!("{}", file);
            }
        }
    }

    let type_files = module_codegen::create_type_files(schemas_to_generate, &resolver)?;

    for file in type_files.iter().take(5) {
        let result = formatter
            .format(&file.source)
            .with_context(|| format!("Failed to format the module: {}\n", file.path));

        match result {
            Err(e) => {
                println!("{:?}\n{}", e, file.source)
            }
            Ok(file) => {
                println!("{}", file);
            }
        }
    }

    Ok(())
}
