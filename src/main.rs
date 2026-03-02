use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use anyhow::Result;
use clap::{Parser, Subcommand};
use rayon::prelude::*;

use code_scan::index::{FunctionKindsFilter, OutputMode, ScanResult, Stats};
use code_scan::walk::WalkConfig;
use code_scan::{output, parse, rules, walk};

#[derive(Parser)]
#[command(name = "code-scan", version, about = "Fast TS/JS codebase indexer for AI agents")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Index functions, bindings, and exports
    #[command(alias = "i")]
    Index {
        #[arg(long, default_value = ".")]
        root: PathBuf,

        #[arg(long, default_value = "compact")]
        mode: OutputMode,

        #[arg(long, value_delimiter = ',')]
        include: Vec<String>,

        #[arg(long, value_delimiter = ',')]
        exclude: Vec<String>,

        #[arg(long, default_value = "1048576")]
        max_bytes_per_file: u64,

        #[arg(long, default_value = "all")]
        function_kinds: FunctionKindsFilter,

        /// Scan a single file instead of a directory
        #[arg(long)]
        file: Option<PathBuf>,

        /// Include export info in output
        #[arg(long)]
        with_exports: bool,
    },

    /// Run rules and emit violations
    #[command(alias = "r")]
    Rules {
        #[arg(long, default_value = ".")]
        root: PathBuf,

        #[arg(long, default_value = "compact")]
        mode: OutputMode,

        #[arg(long, value_delimiter = ',')]
        rules: Vec<String>,

        #[arg(long, value_delimiter = ',')]
        include: Vec<String>,

        #[arg(long, value_delimiter = ',')]
        exclude: Vec<String>,

        #[arg(long, default_value = "1048576")]
        max_bytes_per_file: u64,

        #[arg(long, default_value = "all")]
        function_kinds: FunctionKindsFilter,

        /// Check a single file instead of a directory
        #[arg(long)]
        file: Option<PathBuf>,
    },
}

fn main() -> ExitCode {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    match run() {
        Ok(code) => code,
        Err(e) => {
            tracing::error!("{e:#}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Index {
            root,
            mode,
            include,
            exclude,
            max_bytes_per_file,
            function_kinds,
            file,
            ..
        } => {
            let result =
                scan(&root, include, exclude, max_bytes_per_file, function_kinds, file, &[])?;

            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            output::write_result(&result, mode, &mut handle)?;
            handle.write_all(b"\n")?;

            Ok(ExitCode::SUCCESS)
        }

        Commands::Rules {
            root,
            mode,
            rules: enabled_rules,
            include,
            exclude,
            max_bytes_per_file,
            function_kinds,
            file,
        } => {
            let result = scan(
                &root,
                include,
                exclude,
                max_bytes_per_file,
                function_kinds,
                file,
                &enabled_rules,
            )?;

            let has_violations = result.file_indices.iter().any(|fi| !fi.violations.is_empty());

            let stdout = std::io::stdout();
            let mut handle = stdout.lock();
            output::write_rules_result(&result, mode, &mut handle)?;
            handle.write_all(b"\n")?;

            if has_violations { Ok(ExitCode::from(1)) } else { Ok(ExitCode::SUCCESS) }
        }
    }
}

fn scan(
    root: &Path,
    include: Vec<String>,
    exclude: Vec<String>,
    max_bytes: u64,
    filter: FunctionKindsFilter,
    file: Option<PathBuf>,
    enabled_rules: &[String],
) -> Result<ScanResult> {
    let walk_config = WalkConfig {
        root: root.to_path_buf(),
        extensions: include,
        exclude,
        max_bytes,
        single_file: file,
    };

    let files = walk::collect_files(&walk_config)?;
    let total = files.len();

    let canonical_root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());

    let file_indices: Vec<_> = files
        .par_iter()
        .filter_map(|path| match parse::process_file(path, &canonical_root, filter) {
            Ok(fi) => Some(fi),
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "failed to process file");
                None
            }
        })
        .collect();

    let parsed = file_indices.len();
    let error_count = file_indices.iter().filter(|fi| fi.parse_errors > 0).count();

    // Run rules if any enabled
    let mut file_indices = file_indices;
    if !enabled_rules.is_empty() {
        for fi in &mut file_indices {
            rules::run_rules(enabled_rules, fi);
        }
    }

    // Sort by path for deterministic output
    file_indices.sort_by(|a, b| a.path.cmp(&b.path));

    Ok(ScanResult {
        ver: 1,
        root: root.to_string_lossy().to_string(),
        stats: Stats { files: total, parsed, skipped: total - parsed, errors: error_count },
        file_indices,
        errors: Vec::new(),
    })
}
