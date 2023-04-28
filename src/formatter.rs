use std::path::Path;

use anyhow::anyhow;
use dprint_plugin_typescript::configuration::{Configuration, NextControlFlowPosition, QuoteStyle};

pub struct Formatter {
    config: Configuration,
}

impl Formatter {
    pub fn new() -> Self {
        // build the configuration once
        let config = dprint_plugin_typescript::configuration::ConfigurationBuilder::new()
            .line_width(80)
            .prefer_single_line(false)
            .quote_style(QuoteStyle::AlwaysDouble)
            .next_control_flow_position(NextControlFlowPosition::SameLine)
            .indent_width(4)
            .build();

        Formatter { config }
    }
    pub fn format(&self, path: &Path, text: &str) -> anyhow::Result<String> {
        let result = dprint_plugin_typescript::format_text(path, text, &self.config)
            .map_err(|e| anyhow!("dprint error: {}", e))?
            .ok_or_else(|| anyhow!("dprint returned None"))?;

        Ok(result)
    }
}
