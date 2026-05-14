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

        let cache_dir = dirs_next::cache_dir()
            .context("could not determine cache directory")?
            .join("treebox");

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

    pub fn repo_cache_dir(&self) -> PathBuf {
        self.cache_dir.join("repos")
    }

    pub fn nvim_snippet(&self) -> String {
        "vim.opt.runtimepath:prepend(vim.env.TREEBOX_OUT or vim.fn.stdpath('data') .. '/treebox')\n\nvim.api.nvim_create_autocmd('FileType', {\n  callback = function()\n    pcall(vim.treesitter.start)\n  end,\n})".to_string()
    }
}

fn default_out_dir() -> Result<PathBuf> {
    Ok(dirs_next::data_dir()
        .context("could not determine data directory")?
        .join("treebox"))
}
