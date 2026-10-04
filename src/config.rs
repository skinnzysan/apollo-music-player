use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Language {
    #[serde(rename = "en")]
    English,
    #[serde(rename = "pl")]
    Polish,
}

impl Default for Language {
    fn default() -> Self {
        Language::English
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct AppConfig {
    #[serde(default)]
    pub language: Language,
}

pub fn get_config_path() -> PathBuf {
    if let Some(config_dir) = dirs::config_dir() {
        config_dir.join("apollo/config.toml")
    } else {
        PathBuf::from(".apollo_config.toml")
    }
}

pub fn load_config() -> AppConfig {
    let path = get_config_path();
    if path.exists() {
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(config) = toml::from_str(&content) {
                return config;
            }
        }
    } else {
        // Create default config
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        let config = AppConfig::default();
        if let Ok(content) = toml::to_string_pretty(&config) {
            let _ = fs::write(&path, content);
        }
        return config;
    }
    AppConfig::default()
}
