use super::*;
use clap::Parser;
use std::fs;
use tempfile::TempDir;

#[test]
fn scan_collects_and_applies_rules() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    fs::write(root.join("a.ts"), "export function a(){}\nexport function b(){}\n").unwrap();

    let result = scan(
        root,
        vec![],
        vec![],
        1_048_576,
        FunctionKindsFilter::All,
        None,
        &["one_exported_function_per_file".to_string()],
    )
    .unwrap();

    assert_eq!(result.stats.files, 1);
    assert_eq!(result.stats.parsed, 1);
    assert!(
        result.file_indices[0]
            .violations
            .iter()
            .any(|v| v.rule == "one_exported_function_per_file")
    );
}

#[test]
fn scan_single_file_parse_failure_is_skipped() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("bad.txt");
    fs::write(&file, "not ts/js").unwrap();

    let result =
        scan(dir.path(), vec![], vec![], 1_048_576, FunctionKindsFilter::All, Some(file), &[])
            .unwrap();

    assert_eq!(result.stats.files, 1);
    assert_eq!(result.stats.parsed, 0);
    assert_eq!(result.stats.skipped, 1);
}

#[test]
fn scan_uses_noncanonical_root_fallback() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("ok.ts");
    fs::write(&file, "export function ok(){}\n").unwrap();
    let fake_root = dir.path().join("does-not-exist");

    let result =
        scan(&fake_root, vec![], vec![], 1_048_576, FunctionKindsFilter::All, Some(file), &[])
            .unwrap();

    assert_eq!(result.stats.files, 1);
    assert_eq!(result.stats.parsed, 1);
}

#[test]
fn run_with_cli_index_and_rules_paths() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    fs::write(root.join("a.ts"), "export function a(){}\nexport function b(){}\n").unwrap();

    let cli_index = Cli::parse_from([
        "code-scan",
        "index",
        "--root",
        root.to_str().unwrap(),
        "--mode",
        "compact",
    ]);
    let code = run_with_cli(cli_index).unwrap();
    assert_eq!(code, ExitCode::SUCCESS);

    let cli_rules = Cli::parse_from([
        "code-scan",
        "rules",
        "--root",
        root.to_str().unwrap(),
        "--rules",
        "one_exported_function_per_file",
        "--mode",
        "compact",
    ]);
    let code = run_with_cli(cli_rules).unwrap();
    assert_eq!(code, ExitCode::from(1));
}

#[test]
fn run_with_cli_rules_success_without_violations() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    fs::write(root.join("ok.ts"), "export function only(){}\n").unwrap();

    let cli_rules = Cli::parse_from([
        "code-scan",
        "rules",
        "--root",
        root.to_str().unwrap(),
        "--rules",
        "one_exported_function_per_file",
        "--mode",
        "compact",
    ]);
    let code = run_with_cli(cli_rules).unwrap();
    assert_eq!(code, ExitCode::SUCCESS);
}

#[test]
fn run_with_cli_rules_with_default_rule_set() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    fs::write(root.join("ok.ts"), "export function only(){}\n").unwrap();

    let cli_rules = Cli::parse_from([
        "code-scan",
        "rules",
        "--root",
        root.to_str().unwrap(),
        "--mode",
        "compact",
    ]);
    let code = run_with_cli(cli_rules).unwrap();
    assert_eq!(code, ExitCode::from(1));
}

#[test]
fn run_with_cli_rules_single_file_path() {
    let dir = TempDir::new().unwrap();
    let root = dir.path();
    let file = root.join("ok.ts");
    fs::write(&file, "export function only(){}\n").unwrap();

    let cli_rules = Cli::parse_from([
        "code-scan",
        "rules",
        "--root",
        root.to_str().unwrap(),
        "--file",
        file.to_str().unwrap(),
        "--rules",
        "one_exported_function_per_file",
        "--mode",
        "compact",
    ]);
    let code = run_with_cli(cli_rules).unwrap();
    assert_eq!(code, ExitCode::SUCCESS);
}
