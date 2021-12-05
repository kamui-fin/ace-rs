use crate::anki::DeckModelInfo;
use serde_derive::{Deserialize, Serialize};
use std::{collections::HashMap, fs};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub words_file: String,
    pub failed_words_file: Option<String>,
    pub anki: DeckModelInfo,
    pub dict: HashMap<String, DictInfo>,
}

#[derive(Serialize, Hash, Deserialize, Debug)]
pub struct DictInfo {
    pub priority: i64,
    pub fallback: bool,
}

impl Config {
    pub fn from_path(path: &str) -> Self {
        let conf_text = fs::read_to_string(path).unwrap();
        let config: Config = toml::from_str(&conf_text).unwrap();
        config
    }
}
