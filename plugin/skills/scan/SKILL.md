---
name: scan
description: Index all functions, bindings, and exports in a TS/JS codebase for quick understanding. Use when you need to understand project structure, find functions, check for unused code, or get an overview before refactoring.
allowed-tools: Bash, Read
---

Run `code-scan index --root . --mode files` to get a per-file JSON map of function names (best for LLMs). For summaries, use `--mode folders`. Use `--mode compact` or `--mode verbose` when you need raw or detailed records.

- Note: nested functions are emitted with dot notation based on their nearest enclosing named function (e.g., `createBlogRegistry.get`).

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

### Group by file (built-in)

```bash
code-scan index --root . --mode files
```

Example output (dot-notation for nested functions):

```json
{
  "ver": 1,
  "stats": { "files": 6, "parsed": 6, "skipped": 0, "errors": 0 },
  "files": {
    "services/blog.service.ts": [
      "calculateReadingTime",
      "createBlogRegistry",
      "createBlogRegistry.get",
      "createBlogRegistry.getAllPosts"
    ],
    "services/rss.service.ts": ["createRssResponse", "createRssXml"]
  }
}
```

### Group by file (from compact, with jq)

Per-file function names from compact output:

```bash
code-scan index --root . --mode compact \
  | jq 'reduce .f[] as $t ({}; (.[$t[0]] |= ((. // []) + [$t[3]])))
        | with_entries(.value |= (map(select(. != "")) | unique | sort))'
```

Per-file function objects (name, line, kind, exported):

```bash
code-scan index --root . --mode compact \
  | jq 'reduce .f[] as $t ({}; (.[$t[0]] |= ((. // []) + [{name:$t[3], line:$t[1], kind:$t[5], exported:($t[4]==1)}])))'
```

### Group by folder (built-in)

```bash
code-scan index --root . --mode folders
```

Example output:

```json
{
  "ver": 1,
  "stats": { "files": 6, "parsed": 6, "skipped": 0, "errors": 0 },
  "folders": {
    "services": {
      "functions": 24,
      "names": [
        "calculateReadingTime",
        "createBlogRegistry",
        "createBlogRegistry.get",
        "createRssResponse"
      ]
    }
  }
}
```

### Group by folder (from compact, with jq)

Function names grouped by folder:

```bash
code-scan index --root . --mode compact \
  | jq 'reduce .f[] as $t ({}; (
          ($t[0] | split("/") | .[:-1] | join("/")) as $dir |
          (.[$dir] |= ((. // []) + [$t[3]]))
        ))
        | with_entries(.value |= (map(select(. != "")) | unique | sort))'
```

Folder summary with counts and names:

```bash
code-scan index --root . --mode compact \
  | jq 'reduce .f[] as $t ({ };
        (
          ($t[0] | split("/") | .[:-1] | join("/")) as $dir |
          .[$dir] |= ((. // {functions: 0, names: []})
            | .functions += 1
            | .names += [$t[3]])
        ))
        | with_entries(.value.names |= (map(select(. != "")) | unique | sort))'
```

## Compact format reference

| Key | Contents | Tuple format |
|-----|----------|-------------|
| `f` | Functions | `[file, line, col, name, exported(0/1), kind]` |
| `b` | Bindings | `[file, line, col, name, kind, refs]` |
| `x` | Exports | `[file, name, kind_code]` |
| `viol` | Violations | `[file, rule, count, details]` |

Function kinds: `fn` (declaration), `arr` (arrow), `fex` (expression), `cls` (class method), `obj` (object method), `get` (getter), `set` (setter), `ctor` (constructor).
