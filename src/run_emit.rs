use anyhow::{anyhow, Context};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{collections::HashMap, rc::Rc};

use crate::{
    formatter, module_codegen, parser::SwaggerParser, settings::Settings, swagger::SwaggerApi,
};

use tokio::fs::{create_dir_all, File};
use tokio::io::AsyncWriteExt;

pub async fn run_emit(swagger_api: &SwaggerApi, settings: &Settings) -> anyhow::Result<()> {
    let swagger = swagger_api
        .get_swagger_info(&settings.swagger_endpoint)
        .await?;

    let mut parser = SwaggerParser::new(swagger);

    parser.parse_swagger().context("Failed to parse swagger")?;

    let SwaggerParser {
        modules, schemas, ..
    } = parser;

    let schemas_to_generate = schemas.values().cloned().collect::<Vec<_>>();

    let resolver = Rc::new(module_codegen::SchemaResolver::new(schemas));

    let formatter = formatter::Formatter::new();

    let output_root = Path::new(&settings.output_dir);

    let module_root = output_root.join("endpoints");

    create_dir_all(&module_root).await?;

    for (path, module) in modules.into_iter() {
        let bad_code = module_codegen::generate_actions_file(module, resolver.clone())?;

        let file_path = module_root.join(format!("{}.ts", path));

        let result = formatter
            .format(&bad_code)
            .with_context(|| format!("Failed to format the endpoint module: {}\n", path));

        match result {
            Err(e) => {
                println!("{:?}\n{}", e, bad_code);

                return Err(anyhow!("Failed to format endpoints {}\n{:?}", path, e));
            }
            Ok(src) => {
                let mut file = File::create(&file_path).await?;
                file.write_all(src.as_bytes()).await?;
            }
        }
    }

    let type_files = module_codegen::create_type_files(schemas_to_generate, &resolver)?;

    for source_file in type_files.iter() {
        // Remove ~mortar/
        let file_path_from_root = &source_file.path.as_str()[8..];
        let file_path = output_root.join(format!("{}.ts", file_path_from_root));

        let result = formatter
            .format(&source_file.source)
            .with_context(|| format!("Failed to format the module: {}\n", source_file.path));

        create_dir_all(&file_path.parent().unwrap()).await?;

        match result {
            Err(e) => {
                println!("{:?}\n{}", e, source_file.source);
                return Err(anyhow!(
                    "Failed to format type file {}\n{:?}",
                    &source_file.path,
                    e
                ));
            }
            Ok(src) => {
                let mut file = File::create(&file_path).await?;
                file.write_all(src.as_bytes()).await?;
            }
        }
    }

    Ok(())
}
