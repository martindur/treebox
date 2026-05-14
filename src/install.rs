use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use tempfile::TempDir;

use crate::config::Config;
use crate::metadata::{InstalledLanguage, Metadata};
use crate::registry::{self, RegistryEntry, Source};

pub fn list(config: &Config, only_installed: bool) -> Result<()> {
    let registry = registry::load()?;
    let metadata = Metadata::load(config)?;

    println!("{:<24} {:<10} FILETYPES", "LANGUAGE", "INSTALLED");
    for (lang, entry) in registry {
        let installed = metadata.languages.contains_key(&lang);
        if only_installed && !installed {
            continue;
        }
        println!(
            "{:<24} {:<10} {}",
            lang,
            if installed { "yes" } else { "no" },
            entry.filetypes.join(",")
        );
    }

    Ok(())
}

pub fn add(config: &Config, languages: &[String]) -> Result<()> {
    let registry = registry::load()?;
    let resolved = registry::resolve(&registry, languages)?;
    let mut metadata = Metadata::load(config)?;

    ensure_layout(config)?;

    for lang in resolved {
        let entry = registry
            .get(&lang)
            .with_context(|| format!("unknown language '{lang}'"))?;
        println!("Installing {lang}");
        let installed = install_language(config, &lang, entry)?;
        metadata.languages.insert(lang, installed);
        metadata.save(config)?;
    }

    Ok(())
}

pub fn remove(config: &Config, languages: &[String]) -> Result<()> {
    let mut metadata = Metadata::load(config)?;

    for lang in languages {
        let parser = config.parser_dir().join(format!("{lang}.so"));
        if parser.exists() {
            fs::remove_file(&parser)
                .with_context(|| format!("failed to remove {}", parser.display()))?;
        }

        let queries = config.queries_dir().join(lang);
        if queries.exists() {
            fs::remove_dir_all(&queries)
                .with_context(|| format!("failed to remove {}", queries.display()))?;
        }

        metadata.languages.remove(lang);
        println!("Removed {lang}");
    }

    metadata.save(config)
}

pub fn update(config: &Config, languages: &[String]) -> Result<()> {
    let installed = Metadata::load(config)?;
    let requested = if languages.is_empty() {
        installed.languages.keys().cloned().collect()
    } else {
        languages.to_vec()
    };
    add(config, &requested)
}

pub fn status(config: &Config) -> Result<()> {
    let metadata = Metadata::load(config)?;
    println!("Output: {}", config.out_dir.display());
    println!("Cache:  {}", config.cache_dir.display());
    println!();

    if metadata.languages.is_empty() {
        println!("No languages installed.");
        return Ok(());
    }

    println!("{:<24} {:<12} REF", "LANGUAGE", "TYPE");
    for (lang, installed) in metadata.languages {
        let package_type = if installed.parser_ref.is_some() {
            "parser"
        } else {
            "queries"
        };
        let reference = installed
            .parser_ref
            .as_deref()
            .or(installed.queries_ref.as_deref())
            .map(short_ref)
            .unwrap_or_else(|| "-".to_string());
        println!("{:<24} {:<12} {}", lang, package_type, reference);
    }

    Ok(())
}

pub fn doctor(config: &Config) -> Result<()> {
    println!("Output: {}", config.out_dir.display());
    println!("Cache:  {}", config.cache_dir.display());
    check_tool("git")?;
    check_tool("tree-sitter")?;
    check_tool("cc")
        .or_else(|_| check_tool("gcc"))
        .or_else(|_| check_tool("clang"))?;
    Ok(())
}

fn install_language(
    config: &Config,
    lang: &str,
    entry: &RegistryEntry,
) -> Result<InstalledLanguage> {
    match &entry.source {
        Source::ExternalQueries {
            parser_url,
            queries_url,
            ..
        } => {
            let parser_repo = fetch_repo(config, parser_url)?;
            let queries_repo = fetch_repo(config, queries_url)?;
            build_parser(config, lang, parser_repo.path(), &entry.source)?;
            install_queries(config, lang, queries_repo.path(), &entry.source)?;
            Ok(InstalledLanguage {
                parser_url: Some(parser_url.clone()),
                parser_ref: Some(git_head(parser_repo.path())?),
                queries_url: Some(queries_url.clone()),
                queries_ref: Some(git_head(queries_repo.path())?),
            })
        }
        Source::SelfContained { url, .. } => {
            let repo = fetch_repo(config, url)?;
            build_parser(config, lang, repo.path(), &entry.source)?;
            install_queries(config, lang, repo.path(), &entry.source)?;
            Ok(InstalledLanguage {
                parser_url: Some(url.clone()),
                parser_ref: Some(git_head(repo.path())?),
                queries_url: Some(url.clone()),
                queries_ref: Some(git_head(repo.path())?),
            })
        }
        Source::QueriesOnly { url, .. } => {
            let repo = fetch_repo(config, url)?;
            install_queries(config, lang, repo.path(), &entry.source)?;
            Ok(InstalledLanguage {
                parser_url: None,
                parser_ref: None,
                queries_url: Some(url.clone()),
                queries_ref: Some(git_head(repo.path())?),
            })
        }
        Source::Local { path, .. } => {
            let repo = PathBuf::from(path);
            if !repo.exists() {
                bail!("local source does not exist: {}", repo.display());
            }
            build_parser(config, lang, &repo, &entry.source)?;
            install_queries(config, lang, &repo, &entry.source)?;
            Ok(InstalledLanguage {
                parser_url: None,
                parser_ref: None,
                queries_url: None,
                queries_ref: None,
            })
        }
    }
}

fn ensure_layout(config: &Config) -> Result<()> {
    fs::create_dir_all(config.parser_dir())
        .with_context(|| format!("failed to create {}", config.parser_dir().display()))?;
    fs::create_dir_all(config.queries_dir())
        .with_context(|| format!("failed to create {}", config.queries_dir().display()))
}

struct SourceRepo {
    path: PathBuf,
    _temp: Option<TempDir>,
}

impl SourceRepo {
    fn path(&self) -> &Path {
        &self.path
    }
}

fn fetch_repo(config: &Config, url: &str) -> Result<SourceRepo> {
    fs::create_dir_all(&config.cache_dir)
        .with_context(|| format!("failed to create {}", config.cache_dir.display()))?;
    let temp = tempfile::Builder::new()
        .prefix("treebox-repo-")
        .tempdir_in(&config.cache_dir)
        .with_context(|| {
            format!(
                "failed to create temporary clone under {}",
                config.cache_dir.display()
            )
        })?;
    let path = temp.path().join("repo");

    run(Command::new("git")
        .arg("clone")
        .arg("--depth")
        .arg("1")
        .arg(url)
        .arg(&path))?;

    Ok(SourceRepo {
        path,
        _temp: Some(temp),
    })
}

fn build_parser(config: &Config, lang: &str, repo: &Path, source: &Source) -> Result<()> {
    if matches!(source, Source::QueriesOnly { .. }) {
        return Ok(());
    }

    let parser_source = registry::parser_source_dir(repo, source);
    let output = config.parser_dir().join(format!("{lang}.so"));
    let staging = output.with_extension("so.tmp");

    if staging.exists() {
        fs::remove_file(&staging)
            .with_context(|| format!("failed to remove {}", staging.display()))?;
    }

    run(Command::new("tree-sitter")
        .arg("build")
        .arg("-o")
        .arg(&staging)
        .arg(&parser_source))?;

    fs::rename(&staging, &output).with_context(|| {
        format!(
            "failed to move built parser {} to {}",
            staging.display(),
            output.display()
        )
    })
}

fn install_queries(config: &Config, lang: &str, repo: &Path, source: &Source) -> Result<()> {
    let source_dir = registry::query_source_dir(repo, lang, source)?;
    let target_dir = config.queries_dir().join(lang);
    let staging_dir = target_dir.with_extension("tmp");

    if staging_dir.exists() {
        fs::remove_dir_all(&staging_dir)
            .with_context(|| format!("failed to remove {}", staging_dir.display()))?;
    }
    fs::create_dir_all(&staging_dir)
        .with_context(|| format!("failed to create {}", staging_dir.display()))?;

    copy_scm_files(&source_dir, &staging_dir)?;

    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)
            .with_context(|| format!("failed to remove {}", target_dir.display()))?;
    }
    fs::rename(&staging_dir, &target_dir).with_context(|| {
        format!(
            "failed to move queries {} to {}",
            staging_dir.display(),
            target_dir.display()
        )
    })
}

fn copy_scm_files(source: &Path, target: &Path) -> Result<()> {
    if !source.exists() {
        bail!("query directory does not exist: {}", source.display());
    }

    let mut copied = 0;
    for entry in
        fs::read_dir(source).with_context(|| format!("failed to read {}", source.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if path.extension() != Some(OsStr::new("scm")) {
            continue;
        }

        let target_path = target.join(entry.file_name());
        fs::copy(&path, &target_path).with_context(|| {
            format!(
                "failed to copy {} to {}",
                path.display(),
                target_path.display()
            )
        })?;
        copied += 1;
    }

    if copied == 0 {
        bail!("no .scm query files found in {}", source.display());
    }

    Ok(())
}

fn git_head(repo: &Path) -> Result<String> {
    output(
        Command::new("git")
            .arg("-C")
            .arg(repo)
            .arg("rev-parse")
            .arg("HEAD"),
    )
}

fn check_tool(tool: &str) -> Result<()> {
    let version_arg = if tool == "cc" || tool == "gcc" || tool == "clang" {
        "--version"
    } else {
        "--version"
    };
    output(Command::new(tool).arg(version_arg))
        .map(|version| println!("{tool}: {}", version.lines().next().unwrap_or("ok")))
}

fn run(command: &mut Command) -> Result<()> {
    let status = command
        .status()
        .with_context(|| format!("failed to run {}", command_name(command)))?;
    if !status.success() {
        bail!("command failed: {}", command_name(command));
    }
    Ok(())
}

fn output(command: &mut Command) -> Result<String> {
    let output = command
        .output()
        .with_context(|| format!("failed to run {}", command_name(command)))?;
    if !output.status.success() {
        bail!("command failed: {}", command_name(command));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn command_name(command: &Command) -> String {
    format!("{command:?}")
}

fn short_ref(reference: &str) -> String {
    reference.chars().take(12).collect()
}
