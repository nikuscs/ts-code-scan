use std::io::Write;

use serde::Serialize;

use crate::index::{OutputMode, ScanResult};

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
    }
    Ok(())
}

// ── Compact output ───────────────────────────────────────────────
// Tuple arrays for token efficiency

#[derive(Serialize)]
struct CompactOutput {
    ver: u8,
    stats: CompactStats,
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

#[derive(Serialize)]
struct CompactStats {
    files: usize,
    parsed: usize,
    skipped: usize,
    errors: usize,
}

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

        Self {
            ver: r.ver,
            stats: CompactStats {
                files: r.stats.files,
                parsed: r.stats.parsed,
                skipped: r.stats.skipped,
                errors: r.stats.errors,
            },
            f,
            b,
            x,
            viol,
            err: r.errors.clone(),
        }
    }
}

// ── Verbose output ───────────────────────────────────────────────

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VerboseOutput {
    ver: u8,
    root: String,
    stats: VerboseStats,
    functions: Vec<VerboseFunction>,
    bindings: Vec<VerboseBinding>,
    exports: Vec<VerboseExport>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    violations: Vec<VerboseViolation>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    errors: Vec<String>,
}

#[derive(Serialize)]
struct VerboseStats {
    files: usize,
    parsed: usize,
    skipped: usize,
    errors: usize,
}

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
            stats: VerboseStats {
                files: r.stats.files,
                parsed: r.stats.parsed,
                skipped: r.stats.skipped,
                errors: r.stats.errors,
            },
            functions,
            bindings,
            exports,
            violations,
            errors: r.errors.clone(),
        }
    }
}

// ── Rules-only output ────────────────────────────────────────────

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
        OutputMode::Verbose => {
            let verbose = VerboseRulesOutput::from(result);
            serde_json::to_writer_pretty(w, &verbose)?;
        }
    }
    Ok(())
}

#[derive(Serialize)]
struct CompactRulesOutput {
    ver: u8,
    stats: CompactStats,
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
        Self {
            ver: r.ver,
            stats: CompactStats {
                files: r.stats.files,
                parsed: r.stats.parsed,
                skipped: r.stats.skipped,
                errors: r.stats.errors,
            },
            viol,
            err: r.errors.clone(),
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct VerboseRulesOutput {
    ver: u8,
    stats: VerboseStats,
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
        Self {
            ver: r.ver,
            stats: VerboseStats {
                files: r.stats.files,
                parsed: r.stats.parsed,
                skipped: r.stats.skipped,
                errors: r.stats.errors,
            },
            violations,
            errors: r.errors.clone(),
        }
    }
}
