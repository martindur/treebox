use std::path::PathBuf;

use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Install curated Tree-sitter parser/query bundles for Neovim"
)]
pub struct Cli {
    /// Runtime output directory. Defaults to $TREEBOX_OUT or ~/.local/share/treebox.
    #[arg(long, global = true)]
    pub out: Option<PathBuf>,

    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// List languages in the bundled registry.
    List {
        /// Show only installed languages.
        #[arg(long)]
        installed: bool,
    },
    /// Install one or more languages.
    #[command(alias = "install")]
    Add {
        #[arg(required = true)]
        languages: Vec<String>,
    },
    /// Remove one or more installed languages.
    #[command(alias = "rm")]
    Remove {
        #[arg(required = true)]
        languages: Vec<String>,
    },
    /// Reinstall installed languages, or the specified languages.
    Update { languages: Vec<String> },
    /// Show installed languages and paths.
    Status,
    /// Check local tools and paths.
    Doctor,
}
