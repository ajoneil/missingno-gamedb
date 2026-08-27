//! Ask a signature database what each dump actually is.
//!
//! An entry's artifacts are a claim — that these hashes are dumps of this
//! game. Nothing re-checks that claim after import, so a hack filed as an
//! original stays filed as one. This asks Hasheous per hash and reports the
//! answer — including anything that contradicts where the hash sits. Nothing
//! is written to the manifest: the hash is re-checkable at any time.

use std::{collections::BTreeMap, path::Path, process::Command, thread::sleep, time::Duration};

use missingno_gamedb::{Game, GameBoy, GameBoyColor, Platform, Sg1000, Tree, Vcs};

use crate::report::Report;

const ENDPOINT: &str = "https://hasheous.org/api/v1/Lookup/ByHash/sha1";

#[derive(Default)]
pub struct Stats {
    pub looked_up: usize,
    pub from_cache: usize,
    pub confirmed: usize,
    pub derived: usize,
    pub unknown: usize,
}

pub struct Options {
    pub delay: Duration,
    /// Restrict the sweep to one `tree/slug`.
    pub key: Option<String>,
    pub limit: Option<usize>,
    pub cache_path: std::path::PathBuf,
}

/// What the signature database said about one hash.
#[derive(Clone, PartialEq, Eq)]
enum Answer {
    /// Recognised; the string is the signature entry's own name.
    Known(String),
    /// 404 — no signature database entry. Common for homebrew and protos.
    Unknown,
}

/// TOSEC-style bracket flags that mean "this dump is not the plain original".
/// `[a]` (alternate) and `[!]` (verified good) are deliberately absent: an
/// alternate dump still belongs to the game.
// Longest flag first: a translation must not match [t (trained).
const DERIVED_FLAGS: [(&str, &str); 6] = [
    ("[tr", "translation"),
    ("[cr", "cracked"),
    ("[h", "hack"),
    ("[t", "trained"),
    ("[b", "bad dump"),
    ("[o", "overdump"),
];

fn derived_reason(signature: &str) -> Option<&'static str> {
    let lower = signature.to_lowercase();
    DERIVED_FLAGS
        .iter()
        .find(|(flag, _)| lower.contains(flag))
        .map(|(_, reason)| *reason)
}

/// One `curl` per hash: the crate carries no HTTP client, and a maintenance
/// sweep does not justify pulling an async runtime in.
fn lookup(sha1: &str) -> Result<Answer, String> {
    let output = Command::new("curl")
        .args(["-s", "-H", "accept: application/json", "--max-time", "30"])
        .arg(format!("{ENDPOINT}/{sha1}"))
        .output()
        .map_err(|e| format!("curl failed: {e}"))?;
    let body = String::from_utf8_lossy(&output.stdout);
    let value: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("{sha1}: bad json ({e})"))?;
    // A miss answers with a bare JSON string rather than an object.
    if value.is_string() {
        return Ok(Answer::Unknown);
    }
    let name = value
        .get("signature")
        .and_then(|s| s.get("rom"))
        .and_then(|r| r.get("name"))
        .and_then(|n| n.as_str())
        .or_else(|| value.get("name").and_then(|n| n.as_str()));
    Ok(match name {
        Some(name) => Answer::Known(name.to_owned()),
        None => Answer::Unknown,
    })
}

fn load_cache(path: &Path) -> BTreeMap<String, String> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save_cache(path: &Path, cache: &BTreeMap<String, String>) {
    if let Ok(text) = serde_json::to_string_pretty(cache) {
        let _ = std::fs::write(path, text);
    }
}

pub fn run(data_root: &Path, report: &mut Report, options: &Options) -> Result<Stats, String> {
    let mut stats = Stats::default();
    let mut cache = load_cache(&options.cache_path);
    macro_rules! sweep_each {
        ($($P:ident),* $(,)?) => {$(
            sweep::<$P>(data_root, report, &mut stats, &mut cache, options)?;
        )*};
    }
    missingno_gamedb::with_platforms!(sweep_each);
    save_cache(&options.cache_path, &cache);
    Ok(stats)
}

fn sweep<P: Platform>(
    db_root: &Path,
    report: &mut Report,
    stats: &mut Stats,
    cache: &mut BTreeMap<String, String>,
    options: &Options,
) -> Result<(), String> {
    let (tree, issues) = Tree::<P>::load(db_root).map_err(|e| e.to_string())?;
    if let Some(first) = issues.first() {
        return Err(format!("{}: {}", first.path.display(), first.message));
    }

    for entry in tree.games {
        if options.limit.is_some_and(|n| stats.looked_up >= n) {
            return Ok(());
        }
        let slug = entry.slug.as_str().to_owned();
        if options
            .key
            .as_ref()
            .is_some_and(|want| *want != format!("{}/{slug}", P::DIR))
        {
            continue;
        }
        let game: Game<P> = entry.game;

        let hashes: Vec<String> = game
            .releases
            .iter()
            .flat_map(|r| &r.artifacts)
            .map(|a| a.sha1.as_str().to_owned())
            .collect();

        for sha1 in hashes {
            if options.limit.is_some_and(|n| stats.looked_up >= n) {
                break;
            }
            let answer = match cache.get(&sha1) {
                Some(cached) => {
                    stats.from_cache += 1;
                    if cached.is_empty() {
                        Answer::Unknown
                    } else {
                        Answer::Known(cached.clone())
                    }
                }
                None => {
                    let answer = lookup(&sha1)?;
                    stats.looked_up += 1;
                    cache.insert(
                        sha1.clone(),
                        match &answer {
                            Answer::Known(name) => name.clone(),
                            Answer::Unknown => String::new(),
                        },
                    );
                    // Checkpoint often: a sweep of this size will be interrupted.
                    if stats.looked_up.is_multiple_of(50) {
                        save_cache(&options.cache_path, cache);
                    }
                    sleep(options.delay);
                    answer
                }
            };

            let signature = match answer {
                Answer::Unknown => {
                    stats.unknown += 1;
                    report.add(
                        "Unknown to the signature database",
                        format!("{}/{slug}: {sha1}", P::DIR),
                    );
                    continue;
                }
                Answer::Known(name) => name,
            };

            if let Some(reason) = derived_reason(&signature) {
                stats.derived += 1;
                report.add(
                    "Derived dump sitting in a release",
                    format!("{}/{slug}: {sha1} is {signature:?} ({reason})", P::DIR),
                );
                continue;
            }

            stats.confirmed += 1;
            report.add(
                "Confirmed by the signature database",
                format!("{}/{slug}: {sha1} is {signature:?}", P::DIR),
            );
        }
    }
    Ok(())
}
