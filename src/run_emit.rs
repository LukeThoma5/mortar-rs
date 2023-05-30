use anyhow::{anyhow, Context};
use std::fs::ReadDir;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::{collections::HashMap, rc::Rc};

use crate::swagger::Swagger;
use crate::{
    formatter, module_codegen, parser::SwaggerParser, settings::Settings, swagger::SwaggerApi,
};

use crate::module_codegen::{action_gen, standalone_request_gen, types_gen};
use crate::schema_resolver::SchemaResolver;
use tokio::fs::{create_dir_all, File};
use tokio::io::AsyncWriteExt;

pub async fn run_emit_from_swagger(swagger: Swagger, settings: &Settings) -> anyhow::Result<()> {
    let mut parser = SwaggerParser::new(swagger);

    parser.parse_swagger().context("Failed to parse swagger")?;

    let SwaggerParser {
        modules, schemas, ..
    } = parser;

    let schemas_to_generate = schemas.values().cloned().collect::<Vec<_>>();

    let resolver = Rc::new(SchemaResolver::new(schemas));

    let formatter = formatter::Formatter::new();

    let output_root = Path::new(&settings.output_dir);

    if !output_root.is_relative() {
        Err(anyhow!("Output directory must be relative"))?;
    }

    let _ = tokio::fs::remove_dir_all(&output_root).await;
    create_dir_all(&output_root).await?;
    add_mortar_lib(&output_root).await?;

    let module_root = output_root.join("endpoints");
    create_dir_all(&module_root).await?;
    for (path, module) in modules.into_iter() {
        let bad_code = if settings.skip_endpoint_generation {
            standalone_request_gen::generate_requests_file(module, resolver.clone())?
        } else {
            action_gen::generate_actions_file(module, resolver.clone())?
        };

        let file_path = module_root.join(format!("{}.ts", path));

        let result = formatter
            .format(&file_path, &bad_code)
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

    let type_files = types_gen::create_type_files(schemas_to_generate, &resolver)?;

    for source_file in type_files.iter() {
        // Remove mortar/
        let file_path_from_root = &source_file.path.as_str()[7..];
        let file_path = output_root.join(format!("{}.ts", file_path_from_root));

        let result = formatter
            .format(&file_path, &source_file.source)
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

async fn add_mortar_lib(output_root: &Path) -> anyhow::Result<()> {
    let mortar_lib = include_bytes!("mortar_lib.ts");
    let file_path = output_root.join("lib.ts");
    let mut file = File::create(&file_path).await?;
    file.write_all(mortar_lib).await?;

    Ok(())
}

pub async fn run_emit(swagger_api: &SwaggerApi, settings: &Settings) -> anyhow::Result<()> {
    let swagger = swagger_api
        .get_swagger_info(&settings.swagger_endpoint)
        .await?;

    run_emit_from_swagger(swagger, settings).await
}
