use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use ignore::WalkBuilder;

const DEFAULT_EXTENSIONS: &[&str] = &["ts", "tsx", "js", "jsx"];
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

    // Add custom ignore patterns for excluded dirs
    let mut overrides = ignore::overrides::OverrideBuilder::new(&config.root);
    for pattern in &config.exclude {
        overrides
            .add(&format!("!{pattern}/"))
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
}
