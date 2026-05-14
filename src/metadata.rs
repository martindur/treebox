use std::collections::BTreeMap;
use std::fs;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct Metadata {
    #[serde(default)]
    pub languages: BTreeMap<String, InstalledLanguage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstalledLanguage {
    pub parser_url: Option<String>,
    pub parser_ref: Option<String>,
    pub queries_url: Option<String>,
    pub queries_ref: Option<String>,
}

impl Metadata {
    pub fn load(config: &Config) -> Result<Self> {
        let path = config.metadata_path();
        if !path.exists() {
            return Ok(Self::default());
        }

        let data = fs::read_to_string(&path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        serde_json::from_str(&data).with_context(|| format!("failed to parse {}", path.display()))
    }

    pub fn save(&self, config: &Config) -> Result<()> {
        let path = config.metadata_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let data = serde_json::to_string_pretty(self)?;
        fs::write(&path, data).with_context(|| format!("failed to write {}", path.display()))
    }
}
