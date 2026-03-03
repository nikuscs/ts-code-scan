# 🔍 code-scan

![CI](https://github.com/nikuscs/ts-code-scan/actions/workflows/ci.yml/badge.svg)
![Release](https://img.shields.io/github/v/release/nikuscs/ts-code-scan)
![License](https://img.shields.io/badge/license-MIT-blue.svg)

Fast, single-binary Rust CLI that indexes TypeScript/JavaScript codebases for AI and automation. It walks your project, parses files in parallel, and returns a deterministic JSON index so tools can understand a repo’s structure instantly.

The index includes functions, bindings with reference counts, exports, and optional rule violations. Output is available as compact tuple arrays (token-friendly for LLMs) or verbose objects (human-readable) — ideal for AI assistants, code review automation, repo analytics, and refactoring planning.

## Quick start

Scan a folder and get every function in every file:

```bash
code-scan index --root src/
```

```json
{
  "ver": 1,
  "stats": { "files": 3, "parsed": 3, "skipped": 0, "errors": 0 },
  "f": [
    ["src/auth.ts", 5, 0, "login", 1, "fn"],
    ["src/auth.ts", 18, 0, "logout", 1, "fn"],
    ["src/auth.ts", 30, 0, "refreshToken", 1, "arr"],
    ["src/hooks.ts", 3, 0, "useUser", 1, "arr"],
    ["src/hooks.ts", 15, 0, "useAuth", 1, "arr"],
    ["src/utils.ts", 1, 0, "formatDate", 1, "fn"]
  ]
}
```

Each entry is `[file, line, col, name, exported, kind]`. You get all functions grouped by file — feed it to an LLM and it instantly knows what every file in the project does.

## Why?

- **Fast** — indexes 5,000+ files in under 0.5s. Native Rust with [oxc](https://github.com/nicolo-ribaudo/oxc) parser + rayon parallelism.
- **Token-efficient** — compact mode uses tuple arrays and short keys (`f`, `b`, `x`) to minimize tokens when feeding to LLMs.
- **Complete** — functions (declarations, arrows, class methods, getters/setters, constructors), bindings with reference counts, exports, and lint violations.
- **Zero config** — respects `.gitignore`, auto-skips `node_modules`/`dist`/`build`/`.next`, detects TS/JS/JSX/TSX by extension.
- **Single binary** — ~3MB stripped. No runtime dependencies.

## Install

### Binary

```bash
# From source (requires Rust 1.91+)
cargo install --git https://github.com/nikuscs/ts-code-scan

# Or clone and build
git clone https://github.com/nikuscs/ts-code-scan
cd ts-code-scan
cargo build --release
# Binary at target/release/code-scan
```

Pre-built binaries available in [Releases](https://github.com/nikuscs/ts-code-scan/releases).

### Claude Code Skill

This repo ships as a [Claude Code plugin](https://code.claude.com/docs/en/plugins) with a ready-to-use `/code-scan:scan` skill.

```shell
# Add the marketplace
/plugin marketplace add nikuscs/ts-code-scan

# Install the plugin
/plugin install code-scan@ts-code-scan
```

Then use it:

```shell
/code-scan:scan                  # index the whole project
/code-scan:scan src/utils.ts     # index a single file
/code-scan:scan rules            # run lint rules
/code-scan:scan verbose          # verbose output
```

> **Requires** the `code-scan` binary in your `$PATH`. Install it first with `cargo install` or download from [Releases](https://github.com/nikuscs/ts-code-scan/releases).

## Usage

### Index a project

```bash
# Compact JSON (default, token-efficient for LLMs)
code-scan index --root ./my-project

# Verbose JSON (pretty-printed, human-readable)
code-scan index --root ./my-project --mode verbose

# Single file
code-scan index --file src/utils.ts

# Filter function kinds
code-scan index --root . --function-kinds top+arrow

# Only TypeScript files
code-scan index --root . --include ts,tsx
```

### Run rules

```bash
# Run all rules
code-scan rules --root ./my-project

# Specific rules
code-scan rules --root . --rules no_unused_bindings,max_functions_per_file

# Single file check
code-scan rules --file src/large-file.ts --rules max_functions_per_file
```

## Output

### Compact mode (default)

Tuple arrays for minimal token usage:

```json
{
  "ver": 1,
  "stats": { "files": 842, "parsed": 830, "skipped": 12, "errors": 0 },
  "f": [["src/utils.ts", 10, 3, "sum", 1, "fn"]],
  "b": [["src/utils.ts", 5, 7, "tmp", "const", 0]],
  "x": [["src/utils.ts", "sum", 1]]
}
```

| Key | Contents | Tuple format |
|-----|----------|-------------|
| `f` | Functions | `[file, line, col, name, exported(0/1), kind]` |
| `b` | Bindings | `[file, line, col, name, kind, refs]` |
| `x` | Exports | `[file, name, kind_code]` |
| `viol` | Violations | `[file, rule, count, details]` |

### Verbose mode

Full objects with spans:

```json
{
  "ver": 1,
  "functions": [{
    "file": "src/utils.ts",
    "name": "sum",
    "kind": "FunctionDeclaration",
    "exported": true,
    "isAsync": false,
    "isGenerator": false,
    "span": { "start": { "line": 10, "col": 3 }, "end": { "line": 25, "col": 0 } }
  }]
}
```

## Options

### Index

| Flag | Default | Description |
|------|---------|-------------|
| `--root` | `.` | Project root directory |
| `--mode` | `compact` | Output mode: `compact` or `verbose` |
| `--include` | all | File extensions to include (comma-separated) |
| `--exclude` | | Patterns to exclude (comma-separated) |
| `--max-bytes-per-file` | `1048576` | Skip files larger than N bytes |
| `--function-kinds` | `all` | Filter: `top`, `top+arrow`, `top+arrow+class`, `all` |
| `--file` | | Scan a single file instead of a directory |
| `--with-exports` | `false` | Include export info in output |

### Rules

| Flag | Default | Description |
|------|---------|-------------|
| `--root` | `.` | Project root directory |
| `--mode` | `compact` | Output mode: `compact` or `verbose` |
| `--rules` | all | Rules to run (comma-separated) |
| `--file` | | Check a single file instead of a directory |

### Available rules

| Rule | Description |
|------|-------------|
| `no_unused_bindings` | Flags bindings with zero references (skips `_`-prefixed and imports) |
| `one_exported_function_per_file` | Flags files with more than one exported function |
| `max_functions_per_file` | Flags files with more than 20 functions |

## Function kinds

| Code | Kind |
|------|------|
| `fn` | Function declaration |
| `arr` | Arrow function |
| `fex` | Function expression |
| `cls` | Class method |
| `obj` | Object method |
| `get` | Getter |
| `set` | Setter |
| `ctor` | Constructor |

## How It Works

1. **File discovery** — walks the project tree with [ignore](https://github.com/BurntSushi/ripgrep/tree/master/crates/ignore), respects `.gitignore`, filters by extension
2. **Parallel parsing** — each file gets its own arena allocator, parsed by [oxc](https://github.com/nicolo-ribaudo/oxc) into AST + semantic model
3. **AST extraction** — visitor walks the AST collecting functions (declarations, arrows, methods, getters/setters, constructors) and exports
4. **Binding analysis** — scoping API iterates all symbols, counts references, maps to binding kinds
5. **Rule checking** — optional lint rules run against each file's extracted index
6. **JSON output** — compact tuples or verbose objects, sorted by file path for deterministic output

## Related Projects

- [🦎 amz-crawler](https://github.com/nikuscs/amz-crawler) — Fast Amazon product search CLI with TLS fingerprinting
- [🚀 crauler](https://github.com/nikuscs/crauler) — Fast web crawler with social media extraction
- [🕹️ scrauper](https://github.com/nikuscs/scrauper) — Multi-threaded ScreenScraper.fr scraper for ES-DE
- [⚖️ kante-kusta](https://github.com/nikuscs/kante-kusta) — KuantoKusta.pt price comparison CLI
- [🕵️ olx-tracker](https://github.com/nikuscs/olx-tracker) — Track OLX.pt listings and get alerts on deals

## Disclaimer

> This project is for **educational purposes and AI automation research only**.
> The authors are not responsible for any misuse or for any damages resulting from the use of this tool.
> Users are solely responsible for ensuring compliance with applicable laws and the terms of service
> of any websites accessed. This software is provided "as-is" without warranty of any kind.
>
> If you are a rights holder and wish to have this project removed, please [contact me](https://github.com/nikuscs).

> **Note:** This project was partially developed with AI assistance and may contain bugs or unexpected behavior. Use at your own risk.

## License

MIT — see `LICENSE`.
