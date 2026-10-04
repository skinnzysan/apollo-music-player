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

#[derive(Serialize, Deserialize, Debug)]
pub struct AppConfig {
    #[serde(default)]
    pub language: Language,
    #[serde(default)]
    pub library_paths: Vec<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            language: Language::default(),
            library_paths: vec!["~/Music".to_string()],
        }
    }
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
        let config = AppConfig::default();
        save_config(&config);
        return config;
    }
    AppConfig::default()
}

pub fn save_config(config: &AppConfig) {
    let path = get_config_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(content) = toml::to_string_pretty(config) {
        let _ = fs::write(&path, content);
    }
}
