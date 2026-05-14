# treebox

`treebox` installs curated Tree-sitter parser/query bundles into a Neovim runtime
directory.

The intended happy path is:

```sh
treebox list
treebox add typescript html css
```

Then add the runtime directory to Neovim:

```lua
vim.opt.runtimepath:prepend(vim.env.TREEBOX_OUT or vim.fn.stdpath('data') .. '/treebox')

vim.api.nvim_create_autocmd('FileType', {
  callback = function()
    pcall(vim.treesitter.start)
  end,
})
```

You can also print that snippet with:

```sh
treebox nvim
```

## Commands

```sh
treebox list
treebox list --installed
treebox add <lang...>
treebox remove <lang...>
treebox update [lang...]
treebox status
treebox doctor
treebox nvim
```

`treebox add` resolves required languages from the bundled registry, clones parser
and query sources into temporary directories, builds parser shared libraries with
`tree-sitter build`, and writes Neovim-compatible files:

```text
$TREEBOX_OUT/
  parser/<lang>.so
  queries/<lang>/*.scm
  .treebox/installed.json
```

## Paths

The output directory is chosen in this order:

```text
--out <path>
$TREEBOX_OUT
~/.local/share/treebox
```

The source cache defaults to:

```text
~/.cache/treebox
```

By default, source repositories are removed after the parser and query files are
installed. Use `--cache-repos` to keep cloned repositories under
`~/.cache/treebox/repos` for faster repeated installs and updates:

```sh
treebox --cache-repos add typescript html css
```

## Registry

V1 uses a bundled snapshot of
`neovim-treesitter/treesitter-parser-registry`. This keeps `treebox list`
available offline and avoids depending on an unverified live URL at runtime.

The bundled snapshot checksum is recorded in `assets/registry.sha256`.

## Requirements

V1 expects these tools to be available:

```text
git
tree-sitter
cc, gcc, or clang
```

Run `treebox doctor` to check the local environment.
