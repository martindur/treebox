mod cli;
mod config;
mod install;
mod metadata;
mod registry;

use anyhow::Result;
use clap::Parser;

use crate::cli::{Cli, Command};

fn main() -> Result<()> {
    let cli = Cli::parse();
    let config = config::Config::load(cli.out)?;
    let cache_repos = cli.cache_repos;

    match cli.command {
        Command::List { installed } => install::list(&config, installed),
        Command::Add { languages } => install::add(&config, &languages, cache_repos),
        Command::Remove { languages } => install::remove(&config, &languages),
        Command::Update { languages } => install::update(&config, &languages, cache_repos),
        Command::Status => install::status(&config),
        Command::Doctor => install::doctor(&config),
        Command::Nvim => {
            println!("{}", config.nvim_snippet());
            Ok(())
        }
    }
}
