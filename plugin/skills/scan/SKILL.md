---
name: scan
description: Index all functions, bindings, and exports in a TS/JS codebase for quick understanding. Use when you need to understand project structure, find functions, check for unused code, or get an overview before refactoring.
allowed-tools: Bash, Read
---

Run `code-scan index --root . --mode compact` to get a compact JSON overview of all functions and bindings in the current project.

If $ARGUMENTS is provided:
- If it looks like a file path (contains `/` or ends in `.ts`/`.js`/`.tsx`/`.jsx`), use `--file $ARGUMENTS`
- If it looks like a directory path, use `--root $ARGUMENTS`
- If it is "rules" or "lint", run `code-scan rules --root .` instead
- If it is "verbose", run `code-scan index --root . --mode verbose`

Parse the compact JSON output and present a clear summary:
1. **Stats**: files scanned, parsed, errors
2. **Functions**: list by file, noting exported ones
3. **Bindings with zero refs**: potential dead code (skip `_`-prefixed)
4. **Violations** (if rules mode): group by rule, show affected files

Keep the summary concise — this is meant for quick orientation, not exhaustive detail.

## Compact format reference

| Key | Contents | Tuple format |
|-----|----------|-------------|
| `f` | Functions | `[file, line, col, name, exported(0/1), kind]` |
| `b` | Bindings | `[file, line, col, name, kind, refs]` |
| `x` | Exports | `[file, name, kind_code]` |
| `viol` | Violations | `[file, rule, count, details]` |

Function kinds: `fn` (declaration), `arr` (arrow), `fex` (expression), `cls` (class method), `obj` (object method), `get` (getter), `set` (setter), `ctor` (constructor).
