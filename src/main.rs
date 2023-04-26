mod formatter;
mod module_codegen;
mod string_tools;
mod swagger;
use anyhow::Context;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{collections::HashMap, rc::Rc};
use tokio::fs;
use tokio::io::AsyncReadExt;

mod parser;
mod run_emit;
mod settings;
mod schema_resolver;

use crate::parser::SwaggerParser;
use crate::swagger::SwaggerApi;
use settings::Settings;
use tokio::time::{sleep, Duration};

use clap::Parser;
use self_update::cargo_crate_version;
/// Simple program to greet a person
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    watch: bool,

    #[clap(long)]
    swagger_file: Option<PathBuf>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let settings = Settings::new().context("Failed to create settings")?;

    dbg!("{:?}", &settings);

    if !settings.prevent_update {
        let updated = update()?;
        if updated {
            return Ok(());
        }
    }

    // maintain swagger api for lifetime of program to avoid port exhaustion
    let swagger_api = SwaggerApi::new();

    let mut last_build_id: Option<String> = None;

    let result: anyhow::Result<()> = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            if let Some(fp) = args.swagger_file {
                run_emit_from_file(&fp, &settings).await?;
            } else {
                loop {
                    block_on_matching_build_id(&mut last_build_id, &swagger_api, &settings).await;
                    println!("Running emit");
                    run_emit::run_emit(&swagger_api, &settings).await?;

                    if !args.watch {
                        break;
                    }
                }
            }

            Ok(())
        });

    result
}

fn update() -> anyhow::Result<bool> {
    let status = match self_update::backends::github::Update::configure()
        .repo_owner("LukeThoma5")
        .repo_name("mortar-rs")
        .bin_name("mortar")
        .show_download_progress(true)
        .current_version(cargo_crate_version!())
        .build()?
        .update()
    {
        Ok(status) => status,
        Err(err) => {
            println!("WARN: Error updating: {:?}", err);
            return Ok(false);
        }
    };

    let updated = status.updated();

    if updated {
        println!(
            "Successfully updated to {}. Please restart.",
            status.version()
        );
    }

    Ok(updated)
}

async fn block_on_matching_build_id(
    last_build_id: &mut Option<String>,
    swagger_api: &SwaggerApi,
    settings: &Settings,
) {
    loop {
        let current_build_id = swagger_api
            .get_current_build_id(&settings.mortar_endpoint, last_build_id.as_deref())
            .await;

        match current_build_id {
            Ok(next_build_id) => {
                match last_build_id {
                    Some(last) if last == &next_build_id => {
                        // Should never happen, delay and re-run
                        println!("WARN: BE returned the same id, should block");
                        sleep(Duration::from_millis(1000)).await;
                        continue;
                    }
                    _ => {
                        dbg!("Updating build id to {}", &next_build_id);
                        last_build_id.insert(next_build_id);
                        break;
                    }
                }
            }
            Err(err) => {
                eprintln!("Failed to contact saffron backend build-id endpoint. Is your BE running? Is saffron up to date?. Error:\n{:?}", err);
                sleep(Duration::from_millis(1000)).await;
                continue;
            }
        }
    }
}

pub async fn run_emit_from_file(path: &std::path::Path, settings: &Settings) -> anyhow::Result<()> {
    let mut string = String::new();
    fs::File::open(path)
        .await?
        .read_to_string(&mut string)
        .await?;

    let swagger: swagger::Swagger =
        serde_json::from_str(&string).expect("file should be proper JSON");

    run_emit::run_emit_from_swagger(swagger, &settings).await?;

    Ok(())
}
