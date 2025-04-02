
# md-role-sync

**`md-role-sync`** is a Rust-based CLI tool for synchronizing role descriptions between two Markdown files.

## ✨ Features

- Parses HTML-style Markdown tables.
- Locates source and target tables by their Markdown section headers.
- Syncs role descriptions by matching identifiers.

## 📦 Usage

```sh
md-role-sync --source instruction-ecpk.md --target alaudaconcept.md

