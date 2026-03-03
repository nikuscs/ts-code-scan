use std::path::Path;

use anyhow::{Context, Result};
use oxc::allocator::Allocator;
use oxc::parser::{ParseOptions, Parser};
use oxc::semantic::SemanticBuilder;
use oxc::span::SourceType;

use crate::extract;
use crate::index::{FileIndex, FunctionKindsFilter};

pub fn process_file(path: &Path, root: &Path, filter: FunctionKindsFilter) -> Result<FileIndex> {
    let source = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    let source_type = SourceType::from_path(path)
        .map_err(|_| anyhow::anyhow!("unsupported file type: {}", path.display()))?;

    // Relative path for output
    let canonical_path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let rel_path =
        canonical_path.strip_prefix(root).unwrap_or(&canonical_path).to_string_lossy().to_string();

    let allocator = Allocator::default();

    let parser_ret =
        Parser::new(&allocator, &source, source_type).with_options(ParseOptions::default()).parse();

    let parse_errors = parser_ret.errors.len();

    if parser_ret.panicked {
        tracing::warn!(path = %path.display(), "parser panicked");
        return Ok(FileIndex {
            path: rel_path,
            functions: Vec::new(),
            bindings: Vec::new(),
            exports: Vec::new(),
            violations: Vec::new(),
            parse_errors,
        });
    }

    let sem_ret = SemanticBuilder::new().build(&parser_ret.program);
    let semantic = sem_ret.semantic;

    let result = extract::extract_file(&parser_ret.program, &semantic, &source, filter);

    Ok(FileIndex {
        path: rel_path,
        functions: result.functions,
        bindings: result.bindings,
        exports: result.exports,
        violations: Vec::new(),
        parse_errors,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn parses_basic_ts_and_sets_relative_path() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let file = root.join("a.ts");
        fs::write(
            &file,
            r"
            export function foo() {}
            export const bar = () => {};
            class C { get x(){ return 1 } }
        ",
        )
        .unwrap();

        let root_canon = root.canonicalize().unwrap();
        let fi = process_file(&file, &root_canon, FunctionKindsFilter::All).unwrap();
        assert_eq!(fi.path, "a.ts");
        let names: Vec<_> = fi.functions.iter().filter_map(|f| f.name.clone()).collect();
        assert!(names.contains(&"foo".to_string()));
        assert!(names.contains(&"bar".to_string()));
        assert!(fi.parse_errors == 0);
    }

    #[test]
    fn unsupported_extension_errors() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        let file = root.join("a.unknownext");
        fs::write(&file, "let x = 1;\n").unwrap();
        let root_canon = root.canonicalize().unwrap();
        let err = process_file(&file, &root_canon, FunctionKindsFilter::All).unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("unsupported file type"));
    }
}
