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
    if let Some(path) = &config.single_file {
        return if path.is_file() {
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

    let mut overrides = ignore::overrides::OverrideBuilder::new(&config.root);
    for pattern in DEFAULT_EXCLUDE_DIRS {
        let pat = pattern.trim_matches('/');
        overrides
            .add(&format!("!**/{pat}/**"))
            .with_context(|| format!("invalid default exclude pattern: {pattern}"))?;
    }
    for pattern in &config.exclude {
        let pat = pattern.trim_matches('/');
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

        if !has_matching_extension(path, &extensions) {
            continue;
        }

        if let Ok(meta) = entry.metadata()
            && meta.len() > config.max_bytes
        {
            tracing::debug!(path = %path.display(), "skipping oversized file");
            continue;
        }

        files.push(path.to_path_buf());
    }

    files.sort();
    Ok(files)
}

fn has_matching_extension(path: &Path, extensions: &[&str]) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| extensions.iter().any(|&e| e.eq_ignore_ascii_case(ext)))
}

#[cfg(test)]
#[path = "walk_test.rs"]
mod tests;
