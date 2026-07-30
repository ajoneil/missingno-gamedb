//! One-shot cleanup: status/kind/licence qualifiers that survived inside game
//! titles move into their structured fields.

use std::path::Path;

use missingno_gamedb::{
    Flag, FlagFile, FlagKind, Game, GameBoy, GameBoyColor, GameKind, Platform, ReleaseStatus, Tree,
    Vcs, normalized_title,
};

use crate::{report::Report, tree};

#[derive(Default)]
pub struct Stats {
    pub cleaned: usize,
    pub statuses: usize,
    pub demo_flags: usize,
}

/// The trailing " (…)" chunk of a title, when present.
fn split_trailing(title: &str) -> Option<(&str, &str)> {
    let stripped = title.strip_suffix(')')?;
    let at = stripped.rfind(" (")?;
    Some((&title[..at], &stripped[at + 2..]))
}

enum Qual {
    Status(ReleaseStatus),
    Demo,
    PublicDomain,
}

fn classify(chunk: &str) -> Option<Qual> {
    match chunk {
        "Prototype" | "Proto" => Some(Qual::Status(ReleaseStatus::Prototype)),
        "WIP" => Some(Qual::Status(ReleaseStatus::WorkInProgress)),
        "Demo" => Some(Qual::Demo),
        "PD" => Some(Qual::PublicDomain),
        c if c.starts_with("Beta") && c[4..].chars().all(|ch| ch == ' ' || ch.is_ascii_digit()) => {
            Some(Qual::Status(ReleaseStatus::Beta))
        }
        c if c.starts_with("Prototype ") || c.starts_with("Proto ") => {
            Some(Qual::Status(ReleaseStatus::Prototype))
        }
        _ => None,
    }
}

/// `repo_root` holds curation/flags.ron; `data_root` holds the platform trees.
pub fn run(repo_root: &Path, data_root: &Path, report: &mut Report) -> Result<Stats, String> {
    let mut stats = Stats::default();
    let mut flags = FlagFile::load(repo_root).map_err(|e| e.to_string())?;
    fix_tree::<GameBoy>(data_root, report, &mut stats, &mut flags)?;
    fix_tree::<GameBoyColor>(data_root, report, &mut stats, &mut flags)?;
    fix_tree::<Vcs>(data_root, report, &mut stats, &mut flags)?;
    flags.save(repo_root).map_err(|e| e.to_string())?;
    Ok(stats)
}

fn fix_tree<P: Platform>(
    db_root: &Path,
    report: &mut Report,
    stats: &mut Stats,
    flags: &mut FlagFile,
) -> Result<(), String> {
    let (tree, issues) = Tree::<P>::load(db_root).map_err(|e| e.to_string())?;
    if let Some(first) = issues.first() {
        return Err(format!("{}: {}", first.path.display(), first.message));
    }
    let all_titles: Vec<String> = tree
        .games
        .iter()
        .map(|e| normalized_title(&e.game.title))
        .collect();

    for entry in tree.games {
        let slug = entry.slug.as_str().to_owned();
        let mut game: Game<P> = entry.game;
        let mut changed = false;

        while let Some((base, chunk)) = split_trailing(&game.title) {
            let Some(qual) = classify(chunk) else { break };
            let base = base.trim_end().to_owned();
            match qual {
                Qual::Status(status) => {
                    for release in &mut game.releases {
                        if release.status == ReleaseStatus::Released {
                            release.status = status;
                            stats.statuses += 1;
                        }
                    }
                }
                Qual::Demo => {
                    game.kind = GameKind::Demo;
                    flags.flags.push(Flag {
                        id: flags.next_id(),
                        kind: FlagKind::Custom,
                        subject: vec![format!("{}/{slug}", P::DIR)],
                        note: format!(
                            "\"{}\" was titled \"(Demo)\" — kind set to Demo; verify it is \
                             not a demoscene production",
                            base
                        ),
                    });
                    stats.demo_flags += 1;
                }
                Qual::PublicDomain => {}
            }
            game.title = base;
            changed = true;
        }

        if changed {
            stats.cleaned += 1;
            let norm = normalized_title(&game.title);
            let twins = all_titles.iter().filter(|t| **t == norm).count();
            if twins > 1 {
                report.add(
                    "Cleaned title now collides (merge candidates)",
                    format!("{}/{slug}: {:?}", P::DIR, game.title),
                );
            }
            tree::write_game(db_root, &slug, &game).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
