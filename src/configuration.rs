use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::Write;
use std::path::Path;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Configuration {
    pub general: GeneralConfiguration,
    pub cards: CardsConfiguration,
    pub tachi: TachiConfiguration,
}

impl Configuration {
    pub fn load() -> Result<Self> {
        if !Path::new("takure.toml").exists() {
            File::create("takure.toml")
                .and_then(|mut file| file.write_all(include_bytes!("../takure.toml")))
                .map_err(|err| anyhow::anyhow!("Could not create default config file: {}", err))?;
        }

        confy::load_path("takure.toml")
            .map_err(|err| anyhow::anyhow!("Could not load config: {}", err))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GeneralConfiguration {
    #[serde(default = "default_true")]
    pub enable: bool,
    #[serde(default = "default_timeout")]
    pub timeout: u64,
    #[serde(default = "default_true")]
    pub auto_update: bool,
}

fn default_true() -> bool {
    true
}

fn default_timeout() -> u64 {
    3000
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CardsConfiguration {
    pub whitelist: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TachiConfiguration {
    pub base_url: String,
    pub api_key: String,
}
