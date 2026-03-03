use crate::index::{BindingKind, FileIndex, Violation};

pub trait Rule: Send + Sync {
    fn name(&self) -> &'static str;
    fn check(&self, index: &FileIndex) -> Option<Violation>;
}

// ── Rule registry ────────────────────────────────────────────────

pub fn all_rules() -> Vec<Box<dyn Rule>> {
    vec![
        Box::new(NoUnusedBindings),
        Box::new(OneExportedFunctionPerFile { path_prefix: None }),
        Box::new(MaxFunctionsPerFile { max: 20 }),
    ]
}

pub fn run_rules(enabled: &[String], index: &mut FileIndex) {
    let rules = all_rules();
    let active: Vec<_> = if enabled.is_empty() {
        rules
    } else {
        rules.into_iter().filter(|r| enabled.iter().any(|e| e == r.name())).collect()
    };

    for rule in &active {
        if let Some(violation) = rule.check(index) {
            index.violations.push(violation);
        }
    }
}

// ── no_unused_bindings ───────────────────────────────────────────

struct NoUnusedBindings;

impl Rule for NoUnusedBindings {
    fn name(&self) -> &'static str {
        "no_unused_bindings"
    }

    fn check(&self, index: &FileIndex) -> Option<Violation> {
        let unused: Vec<String> = index
            .bindings
            .iter()
            .filter(|b| {
                b.refs == 0 && !b.name.starts_with('_') && !matches!(b.kind, BindingKind::Import)
            })
            .map(|b| b.name.clone())
            .collect();

        if unused.is_empty() {
            return None;
        }

        Some(Violation { rule: self.name().to_string(), count: unused.len(), details: unused })
    }
}

// ── one_exported_function_per_file ───────────────────────────────

struct OneExportedFunctionPerFile {
    path_prefix: Option<String>,
}

impl Rule for OneExportedFunctionPerFile {
    fn name(&self) -> &'static str {
        "one_exported_function_per_file"
    }

    fn check(&self, index: &FileIndex) -> Option<Violation> {
        if let Some(prefix) = &self.path_prefix
            && !index.path.starts_with(prefix.as_str())
        {
            return None;
        }

        let exported_fns: Vec<String> =
            index.functions.iter().filter(|f| f.exported).filter_map(|f| f.name.clone()).collect();

        if exported_fns.len() <= 1 {
            return None;
        }

        Some(Violation {
            rule: self.name().to_string(),
            count: exported_fns.len(),
            details: exported_fns,
        })
    }
}

// ── max_functions_per_file ───────────────────────────────────────

struct MaxFunctionsPerFile {
    max: usize,
}

impl Rule for MaxFunctionsPerFile {
    fn name(&self) -> &'static str {
        "max_functions_per_file"
    }

    fn check(&self, index: &FileIndex) -> Option<Violation> {
        let count = index.functions.len();
        if count <= self.max {
            return None;
        }

        let names: Vec<String> = index.functions.iter().filter_map(|f| f.name.clone()).collect();

        Some(Violation { rule: self.name().to_string(), count, details: names })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{BindingInfo, FileIndex, FunctionInfo, FunctionKind};

    fn mk_fi(path: &str, fn_names: &[&str], binding_unused: bool) -> FileIndex {
        let functions = fn_names
            .iter()
            .map(|n| FunctionInfo {
                name: Some((*n).into()),
                kind: FunctionKind::Declaration,
                exported: true,
                is_async: false,
                is_generator: false,
                line: 1,
                col: 1,
                line_end: 1,
            })
            .collect();
        let bindings = if binding_unused {
            vec![BindingInfo { name: "tmp".into(), kind: crate::index::BindingKind::Const, exported: false, refs: 0, line: 1, col: 1 }]
        } else {
            vec![]
        };
        FileIndex { path: path.into(), functions, bindings, exports: vec![], violations: vec![], parse_errors: 0 }
    }

    #[test]
    fn no_unused_bindings_flags_zero_ref_non_imports() {
        let mut fi = mk_fi("a.ts", &[], true);
        run_rules(&[], &mut fi);
        assert!(fi.violations.iter().any(|v| v.rule == "no_unused_bindings" && v.count == 1));
    }

    #[test]
    fn one_exported_function_per_file_flags_multiple() {
        let mut fi = mk_fi("src/a.ts", &["a", "b"], false);
        run_rules(&["one_exported_function_per_file".into()], &mut fi);
        assert!(fi.violations.iter().any(|v| v.rule == "one_exported_function_per_file" && v.count == 2));
    }

    #[test]
    fn max_functions_per_file_flags_excess() {
        let mut fi = mk_fi("a.ts", &["a", "b", "c"], false);
        // activate only this rule with a low threshold by instantiating directly
        let v = MaxFunctionsPerFile { max: 2 }.check(&fi).unwrap();
        assert_eq!(v.rule, "max_functions_per_file");
        assert_eq!(v.count, 3);
    }

    #[test]
    fn run_rules_filters_by_enabled_names() {
        let mut fi = mk_fi("a.ts", &[], true);
        run_rules(&["max_functions_per_file".into()], &mut fi);
        // only max_functions_per_file should run; with 0 fns it won't add a violation
        assert!(fi.violations.is_empty());
    }
}
