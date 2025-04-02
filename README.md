
# md-role-sync

**`md-role-sync`** is a Rust-based CLI tool for synchronizing table content between two Markdown files.

## ✨ Features

- Parses HTML-style Markdown tables.
- Locates source and target tables by their Markdown section headers.
- Syncs content by matching identifiers.

## 📦 Usage

```sh
md-role-sync --target target.md --source source.md --header-target "### Target Header {#anchor}" --header-source "### Source Header" --field "TargetField=SourceField" [--field "AnotherTarget=AnotherSource"] [--verbose]
```

- You can add as many fields as needed and if there are same headers in Target and Source files you can mention it once just as --header
- Also u can exclude --verbose flag if you dont need additional logging


