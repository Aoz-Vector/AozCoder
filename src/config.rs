//! Layered configuration via figment (TOML file → environment variables → defaults).
//!
//! Resolution order (highest priority last):
//!   1. Compile-time defaults (struct field defaults)
//!   2. `$XDG_CONFIG_HOME/aozcoder/config.toml` (or `~/.config/aozcoder/config.toml`)
//!   3. Environment variables prefixed `AOZCODER_`
//!
//! Nested keys use `__` as the environment variable separator, following the
//! figment convention: `AOZCODER_UI__THEME=dark` maps to `ui.theme`.

use std::path::PathBuf;

use figment::{
    Figment,
    providers::{Env, Format, Toml},
};
use serde::{Deserialize, Serialize};

/// Root configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "default_api_url")]
    pub api_url: String,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,

    #[serde(default)]
    pub ui: UiConfig,

    #[serde(default)]
    pub model: ModelConfig,
}

/// UI presentation preferences.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UiConfig {
    #[serde(default = "default_theme")]
    pub theme: String,

    #[serde(default = "default_true")]
    pub show_tool_output: bool,

    #[serde(default)]
    pub compact_mode: bool,
}

/// Per-request model parameters; all optional to allow server-side defaults.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModelConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_model: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
}

fn default_api_url() -> String {
    "http://localhost:8080".to_string()
}

fn default_theme() -> String {
    "default".to_string()
}

fn default_true() -> bool {
    true
}

impl Config {
    /// Loads configuration from the standard path and environment.
    pub fn load() -> anyhow::Result<Self> {
        let path = config_path();
        Ok(Figment::new()
            .merge(Toml::file(path))
            .merge(Env::prefixed("AOZCODER_").split("__"))
            .extract()?)
    }

    /// Serializes and writes the configuration to disk.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}

fn config_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("aozcoder")
        .join("config.toml")
}
