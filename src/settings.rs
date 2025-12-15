use crate::groq_client::TranscribeOpts;
use anyhow::Context;
use config::Config;
use directories::BaseDirs;
use global_hotkey::hotkey::HotKey;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs::create_dir, path::PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Configuration {
    pub keys: Vec<KeyConfig>,
}

impl Configuration {
    pub fn parse_keys(self) -> anyhow::Result<HashMap<HotKey, TranscribeOpts>> {
        let keys = self
            .keys
            .into_iter()
            .map(KeyConfig::parse)
            .collect::<Result<Vec<_>, _>>()?;
        let duplicates = keys
            .iter()
            .map(|o| o.0)
            .counts()
            .into_iter()
            .filter(|o| o.1 > 1)
            .collect::<Vec<_>>();
        if !duplicates.is_empty() {
            let details = duplicates
                .into_iter()
                .map(|(o, n)| format!("- {o} defined {n} times."))
                .join("\n");
            anyhow::bail!("Duplicate keys found on configuration file:\n{details}")
        }
        Ok(keys.into_iter().collect())
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KeyConfig {
    pub hotkey: String,
    /// Required ID of the model to use ("whisper-large-v3-turbo" or "whisper-large-v3").
    pub model: String,
    /// The language of the input audio. Supplying the input language in ISO-639-1 (i.e. en, tr`)
    /// format will improve accuracy and latency.
    pub language: Option<String>,
    /// Prompt to guide the model's style or specify how to spell unfamiliar words. (limited to 224
    /// tokens)
    pub prompt: Option<String>,
}

impl KeyConfig {
    fn parse(self) -> anyhow::Result<(HotKey, TranscribeOpts)> {
        let hotkey = self
            .hotkey
            .parse::<HotKey>()
            .context("Failed to parse hotkey.")?;
        let opts = TranscribeOpts {
            model: self.model,
            language: self.language,
            prompt: self.prompt,
        };
        Ok((hotkey, opts))
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

const DEFAULT_CONFIGURATION: &str = r#"[[keys]]
hotkey = "Super+;"
# Required ID of the model to use ("whisper-large-v3-turbo" or "whisper-large-v3").
model = "whisper-large-v3-turbo"
# The language of the input audio. Supplying the input language in ISO-639-1 (i.e. en, tr, es)
# format will improve accuracy and latency.
language = "en"
# Prompt to guide the model's style or specify how to spell unfamiliar words. (limited to 224
# tokens)
# prompt = "some prompt here..."

# You can define more hotkeys with different settings
"#;

/// Get or create config directory path (~/.config/rimay-type/config.toml)
fn get_config_path() -> anyhow::Result<PathBuf> {
    let base_dirs = BaseDirs::new().context("No valid home directory path found.")?;
    let parent = base_dirs.config_dir().join("rimay-type");
    if !parent.exists() {
        create_dir(&parent).context("Failed to create directory.")?;
    }
    let path = parent.join("config.toml");
    if !path.exists() {
        std::fs::write(&path, DEFAULT_CONFIGURATION).context("Failed to write to file.")?;
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_default_config() {
        let config = toml::from_str::<Configuration>(DEFAULT_CONFIGURATION).unwrap();
        println!("{config:#?}");

        for (key, opts) in config.parse_keys().unwrap() {
            println!("{key}: {opts:?}");
        }
    }
}
