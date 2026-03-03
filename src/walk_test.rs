use super::*;

#[test]
fn test_extension_matching() {
    assert!(has_matching_extension(Path::new("foo.ts"), &["ts", "tsx"]));
    assert!(has_matching_extension(Path::new("foo.TSX"), &["ts", "tsx"]));
    assert!(!has_matching_extension(Path::new("foo.rs"), &["ts", "tsx"]));
    assert!(!has_matching_extension(Path::new("foo"), &["ts", "tsx"]));
}

#[test]
fn collect_files_respects_single_file_and_size() {
    use std::fs;
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let p1 = dir.path().join("a.ts");
    let p2 = dir.path().join("b.rs");
    fs::write(&p1, "export function x(){}\n").unwrap();
    fs::write(&p2, "fn y(){}\n").unwrap();

    let files =
        collect_files(&WalkConfig { root: dir.path().into(), ..Default::default() }).unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].file_name().unwrap().to_str().unwrap(), "a.ts");

    let files = collect_files(&WalkConfig {
        root: dir.path().into(),
        single_file: Some(p2.clone()),
        ..Default::default()
    })
    .unwrap();
    assert_eq!(files, vec![p2]);

    let big = dir.path().join("big.ts");
    fs::write(&big, vec![b'x'; 2_000_000]).unwrap();
    let files = collect_files(&WalkConfig {
        root: dir.path().into(),
        max_bytes: 1024,
        ..Default::default()
    })
    .unwrap();
    assert!(!files.iter().any(|p| p.file_name().unwrap() == "big.ts"));

    let sub = dir.path().join("skip");
    fs::create_dir_all(&sub).unwrap();
    fs::write(sub.join("c.ts"), "export function c(){}\n").unwrap();
    let files = collect_files(&WalkConfig {
        root: dir.path().into(),
        exclude: vec!["skip".into()],
        ..Default::default()
    })
    .unwrap();
    assert!(!files.iter().any(|p| p.file_name().unwrap() == "c.ts"));

    let nm = dir.path().join("node_modules");
    fs::create_dir_all(&nm).unwrap();
    fs::write(nm.join("pkg.ts"), "export function nm(){}\n").unwrap();
    let files =
        collect_files(&WalkConfig { root: dir.path().into(), ..Default::default() }).unwrap();
    assert!(!files.iter().any(|p| p.to_string_lossy().contains("node_modules")));
}

#[test]
fn invalid_exclude_pattern_errors() {
    use tempfile::TempDir;
    let dir = TempDir::new().unwrap();
    let res = collect_files(&WalkConfig {
        root: dir.path().into(),
        exclude: vec!["foo[".into()],
        ..Default::default()
    });
    assert!(res.is_err());
}

#[test]
fn single_file_mode_errors_for_missing_path() {
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    let missing = dir.path().join("missing.ts");
    let res = collect_files(&WalkConfig {
        root: dir.path().into(),
        single_file: Some(missing),
        ..Default::default()
    });
    assert!(res.is_err());
}

#[test]
fn custom_extensions_are_used_when_provided() {
    use std::fs;
    use tempfile::TempDir;

    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.rs"), "fn a() {}\n").unwrap();
    fs::write(dir.path().join("b.ts"), "export function b() {}\n").unwrap();

    let files = collect_files(&WalkConfig {
        root: dir.path().into(),
        extensions: vec!["rs".into()],
        ..Default::default()
    })
    .unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].extension().and_then(|e| e.to_str()), Some("rs"));
}
