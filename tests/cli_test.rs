use tempfile::TempDir;

#[test]
fn cli_index_succeeds_and_prints_json() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("a.ts"), "export function a() {}\nexport const b = () => {}\n")
        .unwrap();

    let output = assert_cmd::cargo::cargo_bin_cmd!("code-scan")
        .args(["index", "--root", dir.path().to_str().unwrap(), "--mode", "compact"])
        .output()
        .unwrap();

    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stdout).contains("\"ver\":1"));
}

#[test]
fn cli_rules_violation_returns_one() {
    let dir = TempDir::new().unwrap();
    std::fs::write(dir.path().join("a.ts"), "export function a() {}\nexport function b() {}\n")
        .unwrap();

    let output = assert_cmd::cargo::cargo_bin_cmd!("code-scan")
        .args([
            "rules",
            "--root",
            dir.path().to_str().unwrap(),
            "--rules",
            "one_exported_function_per_file",
            "--mode",
            "compact",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&output.stdout).contains("one_exported_function_per_file"));
}

#[test]
fn cli_index_missing_file_returns_two() {
    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("nope.ts");

    let output = assert_cmd::cargo::cargo_bin_cmd!("code-scan")
        .args([
            "index",
            "--root",
            dir.path().to_str().unwrap(),
            "--file",
            missing.to_str().unwrap(),
            "--mode",
            "compact",
        ])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(2));
}
