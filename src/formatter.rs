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
            .prefer_hanging(true)
            .prefer_single_line(false)
            .quote_style(QuoteStyle::PreferSingle)
            .next_control_flow_position(NextControlFlowPosition::SameLine)
            .build();

        Formatter { config }
    }
    pub fn format(&self, text: &str) -> anyhow::Result<String> {
        let result = dprint_plugin_typescript::format_string(text, &self.config)
            .map_err(|e| anyhow!("dprint error: {}", e))?;

        Ok(result)
    }
}
