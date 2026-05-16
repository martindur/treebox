use std::env;
use std::path::PathBuf;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Config {
    pub out_dir: PathBuf,
    pub cache_dir: PathBuf,
}

impl Config {
    pub fn load(out: Option<PathBuf>) -> Result<Self> {
        let out_dir = match out {
            Some(path) => path,
            None => env::var_os("TREEBOX_OUT")
                .map(PathBuf::from)
                .unwrap_or(default_out_dir()?),
        };

        let cache_dir = xdg_dir("XDG_CACHE_HOME", ".cache", "cache")?.join("treebox");

        Ok(Self { out_dir, cache_dir })
    }

    pub fn metadata_path(&self) -> PathBuf {
        self.out_dir.join(".treebox").join("installed.json")
    }

    pub fn parser_dir(&self) -> PathBuf {
        self.out_dir.join("parser")
    }

    pub fn queries_dir(&self) -> PathBuf {
        self.out_dir.join("queries")
    }
}

fn default_out_dir() -> Result<PathBuf> {
    Ok(xdg_dir("XDG_DATA_HOME", ".local/share", "data")?.join("treebox"))
}

fn xdg_dir(env_var: &str, home_suffix: &str, description: &str) -> Result<PathBuf> {
    xdg_dir_from(env::var_os(env_var), env::var_os("HOME"), home_suffix).with_context(|| {
        format!("could not determine home directory for XDG {description} directory")
    })
}

fn xdg_dir_from(
    env_value: Option<std::ffi::OsString>,
    home: Option<std::ffi::OsString>,
    home_suffix: &str,
) -> Result<PathBuf> {
    if let Some(path) = env_value
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty() && path.is_absolute())
    {
        return Ok(path);
    }

    Ok(PathBuf::from(home.context("HOME is not set")?).join(home_suffix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn xdg_dir_prefers_explicit_env_value() {
        let path = xdg_dir_from(
            Some("/tmp/treebox-data".into()),
            Some("/home/alice".into()),
            ".local/share",
        )
        .unwrap();

        assert_eq!(path, PathBuf::from("/tmp/treebox-data"));
    }

    #[test]
    fn xdg_dir_ignores_empty_env_value() {
        let path =
            xdg_dir_from(Some("".into()), Some("/home/alice".into()), ".local/share").unwrap();

        assert_eq!(path, PathBuf::from("/home/alice/.local/share"));
    }

    #[test]
    fn xdg_dir_ignores_relative_env_value() {
        let path = xdg_dir_from(
            Some("relative/data".into()),
            Some("/home/alice".into()),
            ".local/share",
        )
        .unwrap();

        assert_eq!(path, PathBuf::from("/home/alice/.local/share"));
    }

    #[test]
    fn xdg_dir_defaults_to_home_suffix() {
        let path = xdg_dir_from(None, Some("/home/alice".into()), ".cache").unwrap();

        assert_eq!(path, PathBuf::from("/home/alice/.cache"));
    }

    #[test]
    fn xdg_dir_errors_without_env_value_or_home() {
        let error = xdg_dir_from(None, None, ".local/share").unwrap_err();

        assert_eq!(error.to_string(), "HOME is not set");
    }
}
