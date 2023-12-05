use config::ConfigError;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub debug: bool,
    pub swagger_endpoint: String,
    pub mortar_endpoint: String,
    pub output_dir: String,
    #[serde(default)]
    pub prevent_update: bool,
    #[serde(default)]
    pub skip_endpoint_generation: bool,
    #[serde(default)]
    pub no_format: bool,
}

impl Settings {
    pub fn new() -> Result<Self, ConfigError> {
        let mut settings = config::Config::default();
        settings
            // Add in `./mortar.toml`
            .merge(config::File::with_name("mortar"))?
            // Add in settings from the environment (with a prefix of MORTAR)
            // Eg.. `MORTAR_DEBUG=1 ./target/app` would set the `debug` key
            .merge(config::Environment::with_prefix("MORTAR"))?;

        // You can deserialize (and thus freeze) the entire configuration as
        settings.try_deserialize()
    }
}
