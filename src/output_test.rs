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
fn dot_names_picks_nearest_parent() {
    let fi = FileIndex {
        path: "x.ts".into(),
        functions: vec![
            FunctionInfo {
                name: Some("outer".into()),
                kind: FunctionKind::Declaration,
                exported: false,
                is_async: false,
                is_generator: false,
                line: 1,
                col: 1,
                line_end: 200,
            },
            FunctionInfo {
                name: Some("inner".into()),
                kind: FunctionKind::Declaration,
                exported: false,
                is_async: false,
                is_generator: false,
                line: 20,
                col: 1,
                line_end: 150,
            },
            FunctionInfo {
                name: Some("leaf".into()),
                kind: FunctionKind::Declaration,
                exported: false,
                is_async: false,
                is_generator: false,
                line: 50,
                col: 1,
                line_end: 60,
            },
        ],
        bindings: vec![],
        exports: vec![],
        violations: vec![],
        parse_errors: 0,
    };
    let names = FilesOutput::from(&ScanResult {
        ver: 1,
        root: ".".into(),
        stats: Stats { files: 1, parsed: 1, skipped: 0, errors: 0 },
        file_indices: vec![fi],
        errors: vec![],
    })
    .files
    .remove("x.ts")
    .unwrap();
    assert!(names.contains(&"inner.leaf".to_string()));
    assert!(!names.contains(&"outer.leaf".to_string()));
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
    for mode in [OutputMode::Compact, OutputMode::Verbose, OutputMode::Files, OutputMode::Folders] {
        let mut buf = Vec::new();
        write_result(&r, mode, &mut buf).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert!(v.get("ver").is_some());
    }
}

#[test]
fn write_verbose_and_rules_include_errors_and_violations() {
    let mut fi = FileIndex {
        path: "a.ts".into(),
        functions: vec![FunctionInfo {
            name: Some("a".into()),
            kind: FunctionKind::Declaration,
            exported: true,
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
    fi.violations.push(crate::index::Violation {
        rule: "demo".into(),
        count: 1,
        details: vec!["a".into()],
    });

    let r = ScanResult {
        ver: 1,
        root: "/repo".into(),
        stats: Stats { files: 1, parsed: 1, skipped: 0, errors: 1 },
        file_indices: vec![fi],
        errors: vec!["oops".into()],
    };

    let mut verbose_buf = Vec::new();
    write_result(&r, OutputMode::Verbose, &mut verbose_buf).unwrap();
    let verbose: serde_json::Value = serde_json::from_slice(&verbose_buf).unwrap();
    assert_eq!(verbose["root"], "/repo");
    assert_eq!(verbose["violations"][0]["rule"], "demo");
    assert_eq!(verbose["errors"][0], "oops");

    let mut rules_buf = Vec::new();
    write_rules_result(&r, OutputMode::Verbose, &mut rules_buf).unwrap();
    let rules: serde_json::Value = serde_json::from_slice(&rules_buf).unwrap();
    assert_eq!(rules["violations"][0]["rule"], "demo");
    assert_eq!(rules["errors"][0], "oops");
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
    for mode in [OutputMode::Compact, OutputMode::Verbose, OutputMode::Files, OutputMode::Folders] {
        let mut buf = Vec::new();
        super::write_rules_result(&r, mode, &mut buf).unwrap();
        let v: serde_json::Value = serde_json::from_slice(&buf).unwrap();
        assert!(v.get("ver").is_some());
    }
}

#[test]
fn write_rules_result_handles_empty() {
    let fi = FileIndex {
        path: "a.ts".into(),
        functions: vec![],
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
    let mut buf = Vec::new();
    super::write_rules_result(&r, OutputMode::Compact, &mut buf).unwrap();
    let v: serde_json::Value = serde_json::from_slice(&buf).unwrap();
    assert!(v.get("ver").is_some());
    assert!(v.get("viol").is_some());
}

#[test]
fn verbose_output_includes_exports_and_folder_none_parent_path() {
    let fi = FileIndex {
        path: String::new(),
        functions: vec![FunctionInfo {
            name: Some("top".into()),
            kind: FunctionKind::Declaration,
            exported: true,
            is_async: false,
            is_generator: false,
            line: 1,
            col: 1,
            line_end: 1,
        }],
        bindings: vec![],
        exports: vec![crate::index::ExportInfo { name: "default".into(), kind_code: 2 }],
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

    let mut verbose = Vec::new();
    write_result(&r, OutputMode::Verbose, &mut verbose).unwrap();
    let v: serde_json::Value = serde_json::from_slice(&verbose).unwrap();
    assert_eq!(v["exports"][0]["name"], "default");
    assert_eq!(v["exports"][0]["kindCode"], 2);

    let folders = FoldersOutput::from(&r);
    assert!(folders.folders.contains_key("."));
}
