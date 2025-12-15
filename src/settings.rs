use anyhow::Context;
use config::Config;
use directories::BaseDirs;
use global_hotkey::hotkey::HotKey;
use serde::{Deserialize, Serialize};
use std::{fs::create_dir, path::PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub key: KeyConfig,
}

impl Default for Configuration {
    fn default() -> Self {
        Self {
            key: KeyConfig {
                hotkey: "Super+;".to_string(),
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyConfig {
    pub hotkey: String,
}

impl KeyConfig {
    pub fn hotkey(&self) -> anyhow::Result<HotKey> {
        self.hotkey.parse().context("Failed to parse hotkey.")
    }
}

pub fn get_configuration() -> anyhow::Result<Configuration> {
    let path = get_config_path().context("Failed to get config path.")?;
    Config::builder()
        .add_source(config::File::from(path))
        .build()?
        .try_deserialize::<Configuration>()
        .context("Failed to deserialize configuration.")
}

/// Get or create config directory path (~/.config/rimay-type/config.toml)
fn get_config_path() -> anyhow::Result<PathBuf> {
    let base_dirs = BaseDirs::new().context("No valid home directory path found.")?;
    let parent = base_dirs.config_dir().join("rimay-type");
    if !parent.exists() {
        create_dir(&parent).context("Failed to create directory.")?;
    }
    let path = parent.join("config.toml");
    if !path.exists() {
        let config = Configuration::default();
        let s = toml::to_string_pretty(&config).context("Failed to serialize config.")?;
        std::fs::write(&path, s).context("Failed to write to file.")?;
    }
    Ok(path)
}
