use config::{ConfigError};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Settings {
    pub debug: bool,
    pub swagger_endpoint: String,
    pub output_dir: String
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
        settings.try_into()
    }
}