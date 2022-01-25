use crate::anki::DeckModelInfo;
use anyhow::Context;
use anyhow::Result;
use serde_derive::{Deserialize, Serialize};
use std::{collections::HashMap, fs};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub words_file: String,
    pub failed_words_file: String,
    pub anki: DeckModelInfo,
    pub dict: HashMap<String, DictInfo>,
    pub media: MediaConfig,
    pub ankiconnect: AnkiConnectConfig,
}

#[derive(Serialize, Hash, Deserialize, Debug)]
pub struct DictInfo {
    pub enabled: bool,
    pub priority: i64,
    pub fallback: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct MediaConfig {
    pub custom_audio_dir: String,
    pub regex: String,
    pub fallback_forvo: bool,
    pub bail_on_empty: bool,
    pub limit: usize,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AnkiConnectConfig {
    pub port: usize,
    pub address: String,
}

impl Config {
    pub fn from_path(path: &str) -> Result<Self> {
        let conf_text = fs::read_to_string(path)?;
        let config: Config = toml::from_str(&conf_text)
            .with_context(|| format!("Unable to parse the configuration file"))?;
        Ok(config)
    }
}
