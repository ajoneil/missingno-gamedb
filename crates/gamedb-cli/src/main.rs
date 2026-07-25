mod fix_slugs;
mod fix_titles;
mod import_flags;
mod import_nointro;
mod legacy;
mod migrate_homebrew;
mod migrate_vcs;
mod report;
mod text;
mod tree;
mod verify_hashes;

use std::{
    path::{Path, PathBuf},
    process::ExitCode,
};

use clap::{Parser, Subcommand};
use missingno_gamedb::{Severity, format_all, validate};

use report::Report;

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
    /// One-shot: collapse per-variant VCS entries into games with releases
    MigrateVcs {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Ambiguity report output
        #[arg(long, default_value = "migration-report.md")]
        report: PathBuf,
    },
    /// One-shot: rewrite homebrew (sourced) GB/GBC entries in the new schema
    MigrateHomebrew {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "migration-report.md")]
        report: PathBuf,
    },
    /// One-shot: drop hardware qualifiers from slugs
    FixSlugs {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "migration-report.md")]
        report: PathBuf,
        /// Report the renames without touching the tree
        #[arg(long)]
        dry_run: bool,
    },
    /// Ask a signature database what each dump actually is
    VerifyHashes {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "migration-report.md")]
        report: PathBuf,
        /// Milliseconds between requests
        #[arg(long, default_value_t = 500)]
        delay_ms: u64,
        /// Only this entry, as tree/slug (e.g. vcs/adventure)
        #[arg(long)]
        key: Option<String>,
        /// Stop after this many fresh lookups
        #[arg(long)]
        limit: Option<usize>,
        /// Sweep the whole database — thousands of requests at someone else's
        /// API; prefer --key as part of curating an entry
        #[arg(long)]
        all: bool,
        /// Answers already fetched, so a re-run resumes
        #[arg(long, default_value = "hash-cache.json")]
        cache: PathBuf,
    },
    /// One-shot: move status/kind/licence qualifiers out of game titles
    FixTitles {
        #[arg(default_value = ".")]
        path: PathBuf,
        #[arg(long, default_value = "migration-report.md")]
        report: PathBuf,
    },
    /// Seed curation/flags.ron from migration-report markdown files
    ImportFlags {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Report file(s) to import
        #[arg(long, required = true)]
        report: Vec<PathBuf>,
    },
    /// One-shot: re-import commercial GB/GBC entries from No-Intro DATs
    ImportNointro {
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Logiqx XML DAT file(s); pass GB and GBC together
        #[arg(long, required = true)]
        dat: Vec<PathBuf>,
        #[arg(long, default_value = "migration-report.md")]
        report: PathBuf,
    },
}

fn main() -> ExitCode {
    match Cli::parse().command {
        Command::Validate { path } => run_validate(&resolve_db_root(&path)),
        Command::Fmt { path } => run_fmt(&resolve_db_root(&path)),
        Command::MigrateVcs { path, report } => {
            let path = resolve_db_root(&path);
            run_migration(&path.clone(), &report, "migrate vcs", |r| {
                migrate_vcs::run(&path, r).map(|s| {
                    format!(
                        "{} variant entries collapsed into {} games ({} releases)",
                        s.entries_before, s.games_after, s.releases
                    )
                })
            })
        }
        Command::MigrateHomebrew { path, report } => {
            let path = resolve_db_root(&path);
            run_migration(&path.clone(), &report, "migrate homebrew", |r| {
                migrate_homebrew::run(&path, r).map(|s| format!("{} entries rewritten", s.migrated))
            })
        }
        Command::FixSlugs {
            path,
            report,
            dry_run,
        } => {
            let data_root = resolve_db_root(&path);
            if !dry_run && let Err(e) = ensure_clean_git(&path) {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
            let mut findings = Report::default();
            match fix_slugs::run(&path, &data_root, &mut findings, dry_run) {
                Ok(s) => {
                    let verb = if dry_run { "would rename" } else { "renamed" };
                    println!(
                        "{verb} {} slugs ({} flag subjects, {} left for merge)",
                        s.renamed, s.subjects, s.collisions
                    );
                    if let Err(e) = findings.write(&report, "gamedb fix-slugs report") {
                        eprintln!("failed to write report: {e}");
                        return ExitCode::FAILURE;
                    }
                    println!(
                        "{} review items → {}",
                        findings.item_count(),
                        report.display()
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("aborted: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::VerifyHashes {
            path,
            report,
            delay_ms,
            key,
            limit,
            all,
            cache,
        } => {
            if key.is_none() && limit.is_none() && !all {
                eprintln!(
                    "refusing to sweep the whole database implicitly: pass --key <tree/slug> to \
                     check one entry, --limit N to bound the run, or --all if you really mean it"
                );
                return ExitCode::from(2);
            }
            let data_root = resolve_db_root(&path);
            let options = verify_hashes::Options {
                delay: std::time::Duration::from_millis(delay_ms),
                key,
                limit,
                cache_path: cache,
            };
            let mut findings = Report::default();
            match verify_hashes::run(&data_root, &mut findings, &options) {
                Ok(s) => {
                    println!(
                        "{} confirmed, {} derived dumps in releases, {} unknown ({} fetched, {} cached)",
                        s.confirmed, s.derived, s.unknown, s.looked_up, s.from_cache
                    );
                    if let Err(e) = findings.write(&report, "gamedb verify-hashes report") {
                        eprintln!("failed to write report: {e}");
                        return ExitCode::FAILURE;
                    }
                    println!(
                        "{} review items \u{2192} {}",
                        findings.item_count(),
                        report.display()
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("aborted: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::FixTitles { path, report } => {
            let data_root = resolve_db_root(&path);
            if let Err(e) = ensure_clean_git(&path) {
                eprintln!("{e}");
                return ExitCode::from(2);
            }
            let mut findings = Report::default();
            match fix_titles::run(&path, &data_root, &mut findings) {
                Ok(s) => {
                    println!(
                        "{} titles cleaned ({} release statuses set, {} demo flags)",
                        s.cleaned, s.statuses, s.demo_flags
                    );
                    if let Err(e) = findings.write(&report, "gamedb fix-titles report") {
                        eprintln!("failed to write report: {e}");
                        return ExitCode::FAILURE;
                    }
                    println!(
                        "{} review items → {}",
                        findings.item_count(),
                        report.display()
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("aborted: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::ImportFlags { path, report } => {
            let mut unused = Report::default();
            match import_flags::run(&path, &report, &mut unused) {
                Ok(s) => {
                    println!(
                        "{} flags imported, {} duplicates skipped",
                        s.imported, s.duplicates
                    );
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("{e}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::ImportNointro { path, dat, report } => {
            let path = resolve_db_root(&path);
            run_migration(&path.clone(), &report, "import nointro", |r| {
                import_nointro::run(&path, &dat, r).map(|s| {
                    format!(
                        "{} DAT entries → {} gb + {} gbc games ({} moved gbc→gb, {} merged with old entries, {} leftovers)",
                        s.dat_entries, s.gb_games, s.gbc_games, s.moved_to_gb, s.merged, s.leftovers
                    )
                })
            })
        }
    }
}

fn has_platform_tree(root: &Path) -> bool {
    ["gb", "gbc", "vcs"]
        .iter()
        .any(|dir| root.join(dir).is_dir())
}

/// The platform trees live under `data/` (or at the given path directly, e.g.
/// in test fixtures).
fn resolve_db_root(root: &Path) -> PathBuf {
    let data = root.join("data");
    if !has_platform_tree(root) && has_platform_tree(&data) {
        data
    } else {
        root.to_owned()
    }
}

fn ensure_clean_git(root: &Path) -> Result<(), String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(root)
        .args(["status", "--porcelain"])
        .output()
        .map_err(|e| format!("git status failed: {e}"))?;
    if !output.status.success() {
        return Err("git status failed — is this a git repository?".to_owned());
    }
    if output.stdout.is_empty() {
        Ok(())
    } else {
        Err("working tree is not clean — commit or stash first (the diff is the review)".to_owned())
    }
}

fn run_migration(
    root: &Path,
    report_path: &Path,
    title: &str,
    body: impl FnOnce(&mut Report) -> Result<String, String>,
) -> ExitCode {
    if !has_platform_tree(root) {
        eprintln!(
            "no platform trees (gb/, gbc/, vcs/) under {}",
            root.display()
        );
        return ExitCode::from(2);
    }
    if let Err(e) = ensure_clean_git(root) {
        eprintln!("{e}");
        return ExitCode::from(2);
    }
    let mut report = Report::default();
    match body(&mut report) {
        Ok(summary) => {
            println!("{summary}");
            if let Err(e) = report.write(report_path, &format!("gamedb {title} report")) {
                eprintln!("failed to write report: {e}");
                return ExitCode::FAILURE;
            }
            println!(
                "{} review items → {}",
                report.item_count(),
                report_path.display()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("aborted, nothing written: {e}");
            ExitCode::FAILURE
        }
    }
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
