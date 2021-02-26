mod make_module;
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

#[macro_use]
extern crate swc_common;
extern crate swc_ecma_parser;
use swc_common::sync::Lrc;
use swc_common::{
    errors::{ColorConfig, Handler},
    FileName, FilePathMapping, SourceMap,
};
use swc_ecma_codegen::{text_writer::JsWriter, Config, Emitter};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = Settings::new().context("Failed to create settings")?;

    println!("Settings {:?}", settings);

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

    let cm: Lrc<SourceMap> = Default::default();
    let handler = Handler::with_tty_emitter(ColorConfig::Auto, true, false, Some(cm.clone()));

    // Real usage
    // let fm = cm
    //     .load_file(Path::new("test.js"))
    //     .expect("failed to load test.js");
    let fm = cm.new_source_file(
        FileName::Custom("test.js".into()),
        "export const someActionName = (formData, submitParams) => apiGet<string>(\"SOMELITERAL\")"
            .into(),
    );
    let lexer = Lexer::new(
        // We want to parse ecmascript
        Syntax::Typescript(Default::default()),
        // JscTarget defaults to es5
        Default::default(),
        StringInput::from(&*fm),
        None,
    );
    
    // TODO - completely remove swc from mortar, update dprint to something modern, modify to allow passing in a string content. Make mortar emit (crappy) ts

    // let mut parser = Parser::new_from(lexer);

    // for e in parser.take_errors() {
    //     e.into_diagnostic(&handler).emit();
    // }
    // 
    // let module = parser
    //     .parse_module()
    //     .map_err(|mut e| {
    //         // Unrecoverable fatal error occurred
    //         e.into_diagnostic(&handler).emit()
    //     })
    //     .expect("failed to parser module");

    // println!("{:#?}", module);

    // let w = JsWriter::new(cm.clone(), "\n", std::io::stdout(), None);

    // let mut emitter = Emitter {
    //     cfg: Config { minify: false },
    //     cm: cm.clone(),
    //     comments: None,
    //     wr: Box::new(w),
    // };

    // build the configuration once
    let config = dprint_plugin_typescript::configuration::ConfigurationBuilder::new()
        .line_width(80)
        .prefer_hanging(true)
        .prefer_single_line(false)
        .quote_style(QuoteStyle::PreferSingle)
        .next_control_flow_position(NextControlFlowPosition::SameLine)
        .build();

    // let module = make_module::make_example();

    let result = dprint_plugin_typescript::format_from_input(lexer, &config);
    println!("{:?}", result);

    // now format many files (it is recommended to parallelize this)
    // let files_to_format = vec![(PathBuf::from("path/to/file.ts"), "const  t  =  5 ;")];
    // for (file_path, file_text) in files_to_format.iter() {
    //     let result = dprint_plugin_typescript::format_text(file_path, file_text, &config);
    //     println!("{:?}", result);
    //     // save result here...
    // }

    // current problem is swc only has emitters / writers for javascript. So the question is what do we do about actually emitting the code?
    // Options turn mortarModule into swc's ast and then write an emitter for the ast or directly try and emit the mortar module.
    // the swc ast can be easily built up (fields all public, span is defaultable)

    // TODO how to integrate prettier?

    // emitter.emit_module(&module).unwrap();

    // println!("{:#?}", module);

    Ok(())
}
