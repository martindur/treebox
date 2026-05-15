# treebox

`treebox` installs curated Tree-sitter parsers and queries.

## Install

```sh
cargo install --git https://github.com/martindur/treebox
```

Treebox also expects these tools to be available:

```text
git
tree-sitter
cc, gcc, or clang
```

Run `treebox doctor` to check the local environment.

## Usage

```sh
treebox list
treebox add typescript html css
```

Add Treebox to your Neovim runtime path:

```lua
vim.opt.runtimepath:prepend(vim.env.TREEBOX_OUT or vim.fn.expand('~/.local/share/treebox'))

vim.api.nvim_create_autocmd('FileType', {
  callback = function()
    pcall(vim.treesitter.start)
  end,
})
```

Installed files are written to:

```text
~/.local/share/treebox
```

You can override that with `TREEBOX_OUT` or `--out`.

## Commands

```sh
treebox list
treebox list --installed
treebox add <lang...>
treebox remove <lang...>
treebox update [lang...]
treebox status
treebox doctor
```

## Registry

Treebox uses a bundled snapshot of the registry maintained by
[`neovim-treesitter`](https://github.com/neovim-treesitter/treesitter-parser-registry).

Thanks for making such a registry available in the first place!
