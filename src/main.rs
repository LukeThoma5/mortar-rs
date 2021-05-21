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
mod run_emit;
mod settings;

use crate::parser::SwaggerParser;
use crate::swagger::SwaggerApi;
use settings::Settings;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Settings::new().context("Failed to create settings")?;

    dbg!("{:?}", &settings);

    // maintain swagger api for lifetime of program to avoid port exhaustion
    let swagger_api = SwaggerApi::new();

    loop {
        run_emit::run_emit(&swagger_api, &settings).await?;

        // TODO make a request to the BE and await a rebuild
        break;
    }

    Ok(())
}
