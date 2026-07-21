use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::{Parser, Subcommand};
use missingno_gamedb::{Severity, format_all, validate};

#[derive(Parser)]
#[command(
    name = "gamedb",
    about = "Maintenance tool for the missingno game database",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Check every manifest against the schema and database rules
    Validate {
        /// Database root (contains gb/, gbc/, vcs/)
        #[arg(default_value = ".")]
        path: PathBuf,
    },
    /// Rewrite manifests in canonical formatting
    Fmt {
        /// Database root (contains gb/, gbc/, vcs/)
        #[arg(default_value = ".")]
        path: PathBuf,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Validate { path } => run_validate(&path),
        Command::Fmt { path } => run_fmt(&path),
    }
}

fn has_platform_tree(root: &Path) -> bool {
    ["gb", "gbc", "vcs"]
        .iter()
        .any(|dir| root.join(dir).is_dir())
}

fn run_validate(root: &Path) -> ExitCode {
    if !has_platform_tree(root) {
        eprintln!(
            "no platform trees (gb/, gbc/, vcs/) under {}",
            root.display()
        );
        return ExitCode::from(2);
    }
    let findings = match validate(root) {
        Ok(findings) => findings,
        Err(e) => {
            eprintln!("failed to read database: {e}");
            return ExitCode::from(2);
        }
    };
    for finding in &findings {
        println!(
            "{}: {}: {}",
            finding.path, finding.severity, finding.message
        );
    }
    let errors = findings
        .iter()
        .filter(|f| f.severity == Severity::Error)
        .count();
    let warnings = findings.len() - errors;
    println!("{errors} errors, {warnings} warnings");
    if errors > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

fn run_fmt(root: &Path) -> ExitCode {
    if !has_platform_tree(root) {
        eprintln!(
            "no platform trees (gb/, gbc/, vcs/) under {}",
            root.display()
        );
        return ExitCode::from(2);
    }
    let report = match format_all(root) {
        Ok(report) => report,
        Err(e) => {
            eprintln!("failed to read database: {e}");
            return ExitCode::from(2);
        }
    };
    for path in &report.rewritten {
        println!("rewrote {path}");
    }
    for finding in &report.errors {
        eprintln!(
            "{}: {}: {}",
            finding.path, finding.severity, finding.message
        );
    }
    println!(
        "{} rewritten, {} errors",
        report.rewritten.len(),
        report.errors.len()
    );
    if report.errors.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
