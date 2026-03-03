use std::io::Write;

use serde::Serialize;

use crate::index::{FunctionInfo, OutputMode, ScanResult, Stats};

pub fn write_result<W: Write>(
    result: &ScanResult,
    mode: OutputMode,
    w: &mut W,
) -> anyhow::Result<()> {
    match mode {
        OutputMode::Compact => {
            let compact = CompactOutput::from(result);
            serde_json::to_writer(w, &compact)?;
        }
        OutputMode::Verbose => {
            let verbose = VerboseOutput::from(result);
            serde_json::to_writer_pretty(w, &verbose)?;
        }
        OutputMode::Files => {
            let files = FilesOutput::from(result);
            serde_json::to_writer_pretty(w, &files)?;
        }
        OutputMode::Folders => {
            let folders = FoldersOutput::from(result);
            serde_json::to_writer_pretty(w, &folders)?;
        }
    }
    Ok(())
}

// ── Compact output ───────────────────────────────────────────────
// Tuple arrays for token efficiency

#[derive(Serialize)]
struct CompactOutput {
    ver: u8,
    stats: Stats,
    /// Functions: [file, line, col, name, exported(0/1), kind]
    f: Vec<(String, u32, u32, String, u8, String)>,
    /// Bindings: [file, line, col, name, kind, refs]
    b: Vec<(String, u32, u32, String, String, usize)>,
    /// Exports: `[file, name, kind_code]`
    x: Vec<(String, String, u8)>,
    /// Violations: [file, rule, count, details]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    viol: Vec<(String, String, usize, Vec<String>)>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    err: Vec<String>,
}

// use shared Stats from index

impl From<&ScanResult> for CompactOutput {
    fn from(r: &ScanResult) -> Self {
        let mut f = Vec::new();
        let mut b = Vec::new();
        let mut x = Vec::new();
        let mut viol = Vec::new();

        for fi in &r.file_indices {
            for func in &fi.functions {
                f.push((
                    fi.path.clone(),
                    func.line,
                    func.col,
                    func.name.clone().unwrap_or_default(),
                    u8::from(func.exported),
                    func.kind.code().to_string(),
                ));
            }
            for binding in &fi.bindings {
                b.push((
                    fi.path.clone(),
                    binding.line,
                    binding.col,
                    binding.name.clone(),
                    serde_json::to_value(binding.kind)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_default(),
                    binding.refs,
                ));
            }
            for exp in &fi.exports {
                x.push((fi.path.clone(), exp.name.clone(), exp.kind_code));
            }
            for v in &fi.violations {
                viol.push((fi.path.clone(), v.rule.clone(), v.count, v.details.clone()));
            }
        }

        Self { ver: r.ver, stats: r.stats.clone(), f, b, x, viol, err: r.errors.clone() }
    }
}

// ── Verbose output ───────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VerboseOutput {
    ver: u8,
    root: String,
    stats: Stats,
    functions: Vec<VerboseFunction>,
    bindings: Vec<VerboseBinding>,
    exports: Vec<VerboseExport>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    violations: Vec<VerboseViolation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<String>,
}

// use shared Stats from index

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VerboseFunction {
    file: String,
    name: Option<String>,
    kind: String,
    exported: bool,
    is_async: bool,
    is_generator: bool,
    span: VerboseSpan,
}

#[derive(Serialize)]
struct VerboseSpan {
    start: VerbosePos,
    end: VerbosePos,
}

#[derive(Serialize)]
struct VerbosePos {
    line: u32,
    col: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VerboseBinding {
    file: String,
    name: String,
    kind: String,
    exported: bool,
    refs: usize,
    decl: VerbosePos,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VerboseExport {
    file: String,
    name: String,
    kind_code: u8,
}

#[derive(Serialize)]
struct VerboseViolation {
    file: String,
    rule: String,
    count: usize,
    details: Vec<String>,
}

impl From<&ScanResult> for VerboseOutput {
    fn from(r: &ScanResult) -> Self {
        let mut functions = Vec::new();
        let mut bindings = Vec::new();
        let mut exports = Vec::new();
        let mut violations = Vec::new();

        for fi in &r.file_indices {
            for func in &fi.functions {
                functions.push(VerboseFunction {
                    file: fi.path.clone(),
                    name: func.name.clone(),
                    kind: func.kind.label().to_string(),
                    exported: func.exported,
                    is_async: func.is_async,
                    is_generator: func.is_generator,
                    span: VerboseSpan {
                        start: VerbosePos { line: func.line, col: func.col },
                        end: VerbosePos { line: func.line_end, col: 0 },
                    },
                });
            }
            for binding in &fi.bindings {
                bindings.push(VerboseBinding {
                    file: fi.path.clone(),
                    name: binding.name.clone(),
                    kind: serde_json::to_value(binding.kind)
                        .ok()
                        .and_then(|v| v.as_str().map(String::from))
                        .unwrap_or_default(),
                    exported: binding.exported,
                    refs: binding.refs,
                    decl: VerbosePos { line: binding.line, col: binding.col },
                });
            }
            for exp in &fi.exports {
                exports.push(VerboseExport {
                    file: fi.path.clone(),
                    name: exp.name.clone(),
                    kind_code: exp.kind_code,
                });
            }
            for v in &fi.violations {
                violations.push(VerboseViolation {
                    file: fi.path.clone(),
                    rule: v.rule.clone(),
                    count: v.count,
                    details: v.details.clone(),
                });
            }
        }

        Self {
            ver: r.ver,
            root: r.root.clone(),
            stats: r.stats.clone(),
            functions,
            bindings,
            exports,
            violations,
            errors: r.errors.clone(),
        }
    }
}

// ── Files (grouped) output ───────────────────────────────────────

use std::collections::BTreeMap;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FilesOutput {
    ver: u8,
    stats: Stats,
    files: BTreeMap<String, Vec<String>>,
}

impl From<&ScanResult> for FilesOutput {
    fn from(r: &ScanResult) -> Self {
        let mut map: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for fi in &r.file_indices {
            let out = compute_dot_names(&fi.functions);
            map.insert(fi.path.clone(), out);
        }
        Self { ver: r.ver, stats: r.stats.clone(), files: map }
    }
}

// ── Folders (grouped) output ─────────────────────────────────────

#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
struct FolderSummary {
    functions: usize,
    names: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FoldersOutput {
    ver: u8,
    stats: Stats,
    folders: BTreeMap<String, FolderSummary>,
}

impl From<&ScanResult> for FoldersOutput {
    fn from(r: &ScanResult) -> Self {
        let mut map: BTreeMap<String, FolderSummary> = BTreeMap::new();
        for fi in &r.file_indices {
            let dir = std::path::Path::new(&fi.path).parent().map_or_else(
                || ".".to_string(),
                |p| {
                    let s = p.to_string_lossy();
                    if s.is_empty() { ".".to_string() } else { s.to_string() }
                },
            );
            let entry = map.entry(dir).or_default();
            entry.functions += fi.functions.len();
            let dot_names = compute_dot_names(&fi.functions);
            entry.names.extend(dot_names);
        }
        // Dedup and sort names for determinism
        for entry in map.values_mut() {
            entry.names.sort();
            entry.names.dedup();
        }
        Self { ver: r.ver, stats: r.stats.clone(), folders: map }
    }
}

// (duplicate helper removed)

// Compute dot-notation names for a file's functions
fn compute_dot_names(funcs: &[FunctionInfo]) -> Vec<String> {
    struct Named<'a> {
        name: &'a str,
        start: u32,
        end: u32,
    }
    let named: Vec<Named> = funcs
        .iter()
        .filter_map(|f| {
            f.name.as_deref().map(|n| Named { name: n, start: f.line, end: f.line_end })
        })
        .collect();

    let mut out: Vec<String> = Vec::new();
    for (i, child) in named.iter().enumerate() {
        let mut parent_name: Option<&str> = None;
        let mut parent_span: Option<(u32, u32)> = None;
        for (j, cand) in named.iter().enumerate() {
            if i == j {
                continue;
            }
            if cand.start <= child.start && cand.end >= child.end {
                if let Some((ps, pe)) = parent_span {
                    let cur_len = pe.saturating_sub(ps);
                    let new_len = cand.end.saturating_sub(cand.start);
                    if new_len < cur_len {
                        parent_span = Some((cand.start, cand.end));
                        parent_name = Some(cand.name);
                    }
                } else {
                    parent_span = Some((cand.start, cand.end));
                    parent_name = Some(cand.name);
                }
            }
        }
        if let Some(p) = parent_name {
            out.push(format!("{}.{}", p, child.name));
        } else {
            out.push(child.name.to_string());
        }
    }
    out.sort();
    out.dedup();
    out
}

// ── Rules-only output ────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{
        BindingInfo, BindingKind, FileIndex, FunctionInfo, FunctionKind, ScanResult, Stats,
    };

    fn scan_result_example() -> ScanResult {
        let fi1 = FileIndex {
            path: "dir/a.ts".to_string(),
            functions: vec![
                FunctionInfo {
                    name: Some("foo".into()),
                    kind: FunctionKind::Declaration,
                    exported: true,
                    is_async: false,
                    is_generator: false,
                    line: 1,
                    col: 1,
                    line_end: 1,
                },
                FunctionInfo {
                    name: None,
                    kind: FunctionKind::Arrow,
                    exported: false,
                    is_async: false,
                    is_generator: false,
                    line: 2,
                    col: 1,
                    line_end: 2,
                },
                FunctionInfo {
                    name: Some("foo".into()),
                    kind: FunctionKind::Declaration,
                    exported: true,
                    is_async: false,
                    is_generator: false,
                    line: 3,
                    col: 1,
                    line_end: 3,
                },
            ],
            bindings: vec![BindingInfo {
                name: "x".into(),
                kind: BindingKind::Const,
                exported: false,
                refs: 0,
                line: 1,
                col: 1,
            }],
            exports: vec![],
            violations: vec![],
            parse_errors: 0,
        };
        let fi2 = FileIndex {
            path: "b.ts".to_string(),
            functions: vec![FunctionInfo {
                name: Some("bar".into()),
                kind: FunctionKind::Declaration,
                exported: false,
                is_async: false,
                is_generator: false,
                line: 1,
                col: 1,
                line_end: 1,
            }],
            bindings: vec![],
            exports: vec![],
            violations: vec![],
            parse_errors: 0,
        };
        ScanResult {
            ver: 1,
            root: ".".into(),
            stats: Stats { files: 2, parsed: 2, skipped: 0, errors: 0 },
            file_indices: vec![fi1, fi2],
            errors: vec![],
        }
    }

    #[test]
    fn files_mode_groups_named_functions() {
        let r = scan_result_example();
        let files = FilesOutput::from(&r);

        assert_eq!(files.ver, 1);
        assert_eq!(files.stats.parsed, 2);
        // dot-notation when nested: still only foo here (no nested in example)
        assert_eq!(files.files.get("dir/a.ts").unwrap(), &vec!["foo".to_string()]);
        assert_eq!(files.files.get("b.ts").unwrap(), &vec!["bar".to_string()]);
    }

    #[test]
    fn folders_mode_summarizes_by_parent_dir() {
        let r = scan_result_example();
        let folders = FoldersOutput::from(&r);

        let dir = folders.folders.get("dir").unwrap();
        assert_eq!(dir.functions, 3);
        assert_eq!(dir.names, vec!["foo".to_string()]);

        let root_dir = folders.folders.get(".").unwrap();
        assert_eq!(root_dir.functions, 1);
        assert_eq!(root_dir.names, vec!["bar".to_string()]);
    }

    #[test]
    fn folders_mode_uses_dot_names() {
        let fi = FileIndex {
            path: "dir/x.ts".into(),
            functions: vec![
                FunctionInfo {
                    name: Some("builder".into()),
                    kind: FunctionKind::Declaration,
                    exported: false,
                    is_async: false,
                    is_generator: false,
                    line: 1,
                    col: 1,
                    line_end: 50,
                },
                FunctionInfo {
                    name: Some("get".into()),
                    kind: FunctionKind::ObjectMethod,
                    exported: false,
                    is_async: false,
                    is_generator: false,
                    line: 10,
                    col: 1,
                    line_end: 20,
                },
            ],
            bindings: vec![],
            exports: vec![],
            violations: vec![],
            parse_errors: 0,
        };
        let r = ScanResult {
            ver: 1,
            root: ".".into(),
            stats: Stats { files: 1, parsed: 1, skipped: 0, errors: 0 },
            file_indices: vec![fi],
            errors: vec![],
        };
        let folders = FoldersOutput::from(&r);
        let entry = folders.folders.get("dir").unwrap();
        assert!(entry.names.contains(&"builder.get".to_string()));
        assert!(entry.names.contains(&"builder".to_string()));
    }

    #[test]
    fn dot_names_for_nested_methods() {
        // Build a result with a parent function and a nested method
        let fi = FileIndex {
            path: "x.ts".into(),
            functions: vec![
                FunctionInfo {
                    name: Some("builder".into()),
                    kind: FunctionKind::Declaration,
                    exported: false,
                    is_async: false,
                    is_generator: false,
                    line: 1,
                    col: 1,
                    line_end: 100,
                },
                FunctionInfo {
                    name: Some("get".into()),
                    kind: FunctionKind::ObjectMethod,
                    exported: false,
                    is_async: false,
                    is_generator: false,
                    line: 10,
                    col: 1,
                    line_end: 20,
                },
                FunctionInfo {
                    name: Some("util".into()),
                    kind: FunctionKind::Declaration,
                    exported: false,
                    is_async: false,
                    is_generator: false,
                    line: 150,
                    col: 1,
                    line_end: 160,
                },
            ],
            bindings: vec![],
            exports: vec![],
            violations: vec![],
            parse_errors: 0,
        };
        let r = ScanResult {
            ver: 1,
            root: ".".into(),
            stats: Stats { files: 1, parsed: 1, skipped: 0, errors: 0 },
            file_indices: vec![fi],
            errors: vec![],
        };
        let files = FilesOutput::from(&r);
        let names = files.files.get("x.ts").unwrap();
        assert!(names.contains(&"builder.get".to_string()));
        assert!(names.contains(&"builder".to_string()));
        assert!(names.contains(&"util".to_string()));
    }

    #[test]
    fn write_result_emits_valid_json_all_modes() {
        let fi = FileIndex {
            path: "p.ts".into(),
            functions: vec![
                FunctionInfo {
                    name: Some("parent".into()),
                    kind: FunctionKind::Declaration,
                    exported: true,
                    is_async: false,
                    is_generator: false,
                    line: 1,
                    col: 1,
                    line_end: 50,
                },
                FunctionInfo {
                    name: Some("child".into()),
                    kind: FunctionKind::ObjectMethod,
                    exported: false,
                    is_async: false,
                    is_generator: false,
                    line: 10,
                    col: 1,
                    line_end: 20,
                },
            ],
            bindings: vec![BindingInfo {
                name: "x".into(),
                kind: BindingKind::Const,
                exported: false,
                refs: 0,
                line: 1,
                col: 1,
            }],
            exports: vec![],
            violations: vec![],
            parse_errors: 0,
        };
        let r = ScanResult {
            ver: 1,
            root: ".".into(),
            stats: Stats { files: 1, parsed: 1, skipped: 0, errors: 0 },
            file_indices: vec![fi],
            errors: vec![],
        };
        for mode in
            [OutputMode::Compact, OutputMode::Verbose, OutputMode::Files, OutputMode::Folders]
        {
            let mut buf = Vec::new();
            write_result(&r, mode, &mut buf).unwrap();
            let v: serde_json::Value = serde_json::from_slice(&buf).unwrap();
            assert!(v.get("ver").is_some());
        }
    }

    #[test]
    fn write_rules_result_emits_valid_json() {
        let mut fi = FileIndex {
            path: "a.ts".into(),
            functions: vec![],
            bindings: vec![],
            exports: vec![],
            violations: vec![],
            parse_errors: 0,
        };
        fi.violations.push(crate::index::Violation {
            rule: "demo".into(),
            count: 1,
            details: vec!["x".into()],
        });
        let r = ScanResult {
            ver: 1,
            root: ".".into(),
            stats: Stats { files: 1, parsed: 1, skipped: 0, errors: 0 },
            file_indices: vec![fi],
            errors: vec![],
        };
        for mode in
            [OutputMode::Compact, OutputMode::Verbose, OutputMode::Files, OutputMode::Folders]
        {
            let mut buf = Vec::new();
            super::write_rules_result(&r, mode, &mut buf).unwrap();
            let v: serde_json::Value = serde_json::from_slice(&buf).unwrap();
            assert!(v.get("ver").is_some());
        }
    }
}

pub fn write_rules_result<W: Write>(
    result: &ScanResult,
    mode: OutputMode,
    w: &mut W,
) -> anyhow::Result<()> {
    match mode {
        OutputMode::Compact => {
            let compact = CompactRulesOutput::from(result);
            serde_json::to_writer(w, &compact)?;
        }
        OutputMode::Verbose | OutputMode::Files | OutputMode::Folders => {
            let verbose = VerboseRulesOutput::from(result);
            serde_json::to_writer_pretty(w, &verbose)?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct CompactRulesOutput {
    ver: u8,
    stats: Stats,
    viol: Vec<(String, String, usize, Vec<String>)>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    err: Vec<String>,
}

impl From<&ScanResult> for CompactRulesOutput {
    fn from(r: &ScanResult) -> Self {
        let mut viol = Vec::new();
        for fi in &r.file_indices {
            for v in &fi.violations {
                viol.push((fi.path.clone(), v.rule.clone(), v.count, v.details.clone()));
            }
        }
        Self { ver: r.ver, stats: r.stats.clone(), viol, err: r.errors.clone() }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VerboseRulesOutput {
    ver: u8,
    stats: Stats,
    violations: Vec<VerboseViolation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<String>,
}

impl From<&ScanResult> for VerboseRulesOutput {
    fn from(r: &ScanResult) -> Self {
        let mut violations = Vec::new();
        for fi in &r.file_indices {
            for v in &fi.violations {
                violations.push(VerboseViolation {
                    file: fi.path.clone(),
                    rule: v.rule.clone(),
                    count: v.count,
                    details: v.details.clone(),
                });
            }
        }
        Self { ver: r.ver, stats: r.stats.clone(), violations, errors: r.errors.clone() }
    }
}
