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
