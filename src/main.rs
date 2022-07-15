mod formatter;
mod module_codegen;
mod mortar_type;
mod string_tools;
mod swagger;
use anyhow::Context;
use std::path::PathBuf;
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{collections::HashMap, rc::Rc};

mod parser;
mod run_emit;
mod settings;

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
            loop {
                block_on_matching_build_id(&mut last_build_id, &swagger_api, &settings).await;
                println!("Running emit");
                run_emit::run_emit(&swagger_api, &settings).await?;

                if !args.watch {
                    break;
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
            _ => {
                dbg!("Request for build id failed");
                sleep(Duration::from_millis(1000)).await;
                continue;
            }
        }
    }
}
