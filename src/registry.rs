use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::Deserialize;

const REGISTRY_JSON: &str = include_str!("../assets/registry.json");

pub type RegistryMap = BTreeMap<String, RegistryEntry>;

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct RegistryEntry {
    pub source: Source,
    #[serde(default)]
    pub filetypes: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    #[serde(default)]
    pub inject_deps: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
#[allow(dead_code)]
pub enum Source {
    #[serde(rename = "external_queries")]
    ExternalQueries {
        parser_url: String,
        parser_semver: bool,
        #[serde(default)]
        parser_location: Option<String>,
        queries_url: String,
        queries_semver: bool,
    },
    #[serde(rename = "self_contained")]
    SelfContained {
        url: String,
        semver: bool,
        #[serde(default)]
        location: Option<String>,
        #[serde(default)]
        queries_path: Option<String>,
        #[serde(default)]
        queries_dir: Option<String>,
        #[serde(default)]
        generate: Option<bool>,
        #[serde(default)]
        generate_from_json: Option<bool>,
    },
    #[serde(rename = "queries_only")]
    QueriesOnly { url: String, semver: bool },
    #[serde(rename = "local")]
    Local {
        path: String,
        #[serde(default)]
        queries_path: Option<String>,
    },
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ParserManifest {
    #[serde(default)]
    pub url: Option<String>,
    #[serde(default)]
    pub parser_version: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub queries_dir: Option<String>,
    #[serde(default)]
    pub queries_path: Option<String>,
    #[serde(default)]
    pub queries_only: Option<bool>,
    #[serde(default)]
    pub generate: Option<bool>,
}

pub fn load() -> Result<RegistryMap> {
    let mut value: serde_json::Value =
        serde_json::from_str(REGISTRY_JSON).context("bundled registry is not valid JSON")?;
    let object = value
        .as_object_mut()
        .context("bundled registry root is not a JSON object")?;
    object.retain(|key, _| !key.starts_with('$'));
    serde_json::from_value(value).context("bundled registry entries are not valid")
}

pub fn resolve(registry: &RegistryMap, requested: &[String]) -> Result<Vec<String>> {
    let mut ordered = Vec::new();
    let mut seen = BTreeSet::new();

    for lang in requested {
        resolve_one(registry, lang, &mut seen, &mut ordered)?;
    }

    Ok(ordered)
}

fn resolve_one(
    registry: &RegistryMap,
    lang: &str,
    seen: &mut BTreeSet<String>,
    ordered: &mut Vec<String>,
) -> Result<()> {
    if seen.contains(lang) {
        return Ok(());
    }

    let entry = registry
        .get(lang)
        .with_context(|| format!("unknown language '{lang}'"))?;

    seen.insert(lang.to_string());
    for required in &entry.requires {
        resolve_one(registry, required, seen, ordered)?;
    }
    ordered.push(lang.to_string());
    Ok(())
}

pub fn parser_manifest(repo: &Path) -> Result<Option<ParserManifest>> {
    let path = repo.join("parser.json");
    if !path.exists() {
        return Ok(None);
    }

    let data = std::fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&data)
        .with_context(|| format!("failed to parse {}", path.display()))
        .map(Some)
}

pub fn query_source_dir(repo: &Path, lang: &str, source: &Source) -> Result<PathBuf> {
    if let Some(manifest) = parser_manifest(repo)? {
        if let Some(path) = manifest.queries_path {
            return Ok(repo.join(path));
        }
        if let Some(dir) = manifest.queries_dir {
            return Ok(repo.join(dir).join(lang));
        }
    }

    match source {
        Source::SelfContained {
            queries_path,
            queries_dir,
            ..
        } => {
            if let Some(path) = queries_path {
                return Ok(repo.join(path));
            }
            if let Some(dir) = queries_dir {
                return Ok(repo.join(dir).join(lang));
            }
        }
        Source::Local { queries_path, .. } => {
            if let Some(path) = queries_path {
                return Ok(repo.join(path));
            }
        }
        _ => {}
    }

    for candidate in [
        repo.join("queries").join(lang),
        repo.join("queries"),
        repo.join(lang),
    ] {
        if candidate.exists() {
            return Ok(candidate);
        }
    }

    bail!(
        "could not find query files for {lang} in {}",
        repo.display()
    )
}

pub fn parser_source_dir(repo: &Path, source: &Source) -> PathBuf {
    match source {
        Source::ExternalQueries {
            parser_location, ..
        } => parser_location
            .as_ref()
            .map(|location| repo.join(location))
            .unwrap_or_else(|| repo.to_path_buf()),
        Source::SelfContained { location, .. } => location
            .as_ref()
            .map(|location| repo.join(location))
            .unwrap_or_else(|| repo.to_path_buf()),
        Source::Local { path, .. } => PathBuf::from(path),
        Source::QueriesOnly { .. } => repo.to_path_buf(),
    }
}
