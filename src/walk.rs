use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ignore::WalkBuilder;

const DEFAULT_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx"];
const DEFAULT_EXCLUDE_DIRS: &[&str] =
    &["node_modules", "dist", "build", ".next", ".git", "coverage", ".turbo", ".cache"];
const DEFAULT_MAX_BYTES: u64 = 1_048_576; // 1 MB

pub struct WalkConfig {
    pub root: PathBuf,
    pub extensions: Vec<String>,
    pub exclude: Vec<String>,
    pub max_bytes: u64,
    pub single_file: Option<PathBuf>,
}

impl Default for WalkConfig {
    fn default() -> Self {
        Self {
            root: PathBuf::from("."),
            extensions: Vec::new(),
            exclude: Vec::new(),
            max_bytes: DEFAULT_MAX_BYTES,
            single_file: None,
        }
    }
}

pub fn collect_files(config: &WalkConfig) -> Result<Vec<PathBuf>> {
    // Single-file mode
    if let Some(path) = &config.single_file {
        return if path.exists() {
            Ok(vec![path.clone()])
        } else {
            anyhow::bail!("file not found: {}", path.display());
        };
    }

    let extensions: Vec<&str> = if config.extensions.is_empty() {
        DEFAULT_EXTENSIONS.to_vec()
    } else {
        config.extensions.iter().map(String::as_str).collect()
    };

    let mut builder = WalkBuilder::new(&config.root);
    builder.hidden(true).git_ignore(true).git_global(true);

    // Add default and custom ignore patterns for excluded dirs
    let mut overrides = ignore::overrides::OverrideBuilder::new(&config.root);
    for pattern in DEFAULT_EXCLUDE_DIRS {
        let pat = pattern.trim_matches('/');
        overrides
            .add(&format!("!**/{pat}/**"))
            .with_context(|| format!("invalid default exclude pattern: {pattern}"))?;
    }
    for pattern in &config.exclude {
        let pat = pattern.trim_matches('/');
        // Exclude the directory and all its children at any depth
        overrides
            .add(&format!("!**/{pat}/**"))
            .with_context(|| format!("invalid exclude pattern: {pattern}"))?;
    }
    let overrides = overrides.build().context("failed to build exclude overrides")?;
    builder.overrides(overrides);

    let mut files = Vec::new();

    for entry in builder.build() {
        let entry = entry.context("walk error")?;

        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }

        let path = entry.path();

        // Check extension
        if !has_matching_extension(path, &extensions) {
            continue;
        }

        // Check file size
        if let Ok(meta) = entry.metadata()
            && meta.len() > config.max_bytes
        {
            tracing::debug!(path = %path.display(), "skipping oversized file");
            continue;
        }

        files.push(path.to_path_buf());
    }

    // Deterministic ordering
    files.sort();
    Ok(files)
}

fn has_matching_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| extensions.iter().any(|&e| e.eq_ignore_ascii_case(ext)))
}

#[cfg(test)]
mod tests {
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
        use tempfile::tempdir;

        let dir = tempdir().unwrap();
        let p1 = dir.path().join("a.ts");
        let p2 = dir.path().join("b.rs");
        fs::write(&p1, "export function x(){}\n").unwrap();
        fs::write(&p2, "fn y(){}\n").unwrap();

        // Directory walk only picks ts by default
        let files =
            collect_files(&WalkConfig { root: dir.path().into(), ..Default::default() }).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].file_name().unwrap().to_str().unwrap(), "a.ts");

        // Single file path works
        let files = collect_files(&WalkConfig {
            root: dir.path().into(),
            single_file: Some(p2.clone()),
            ..Default::default()
        })
        .unwrap();
        assert_eq!(files, vec![p2]);

        // Large file gets skipped
        let big = dir.path().join("big.ts");
        fs::write(&big, vec![b'x'; 2_000_000]).unwrap();
        let files = collect_files(&WalkConfig {
            root: dir.path().into(),
            max_bytes: 1024,
            ..Default::default()
        })
        .unwrap();
        assert!(!files.iter().any(|p| p.file_name().unwrap() == "big.ts"));

        // Exclude directory
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

        // Default excludes: node_modules should be skipped automatically
        let nm = dir.path().join("node_modules");
        fs::create_dir_all(&nm).unwrap();
        fs::write(nm.join("pkg.ts"), "export function nm(){}\n").unwrap();
        let files =
            collect_files(&WalkConfig { root: dir.path().into(), ..Default::default() }).unwrap();
        assert!(!files.iter().any(|p| p.to_string_lossy().contains("node_modules")));
    }
}
