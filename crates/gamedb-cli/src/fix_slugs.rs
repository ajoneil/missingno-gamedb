//! One-shot cleanup: hardware qualifiers that survived inside slugs are
//! dropped, since every one of them duplicates a structured release field.
//!
//! A suffix is only stripped when the entry's own hardware fields corroborate
//! it — "pal" goes when some release is PAL, "f6sc" when some release is on an
//! F6SC board. An uncorroborated suffix is a word in the game's name.

use std::{
    collections::{BTreeSet, HashMap},
    path::Path,
};

use missingno_gamedb::{
    FlagFile, Game, GameBoy, GameBoyColor, Platform, Tree, Vcs,
};

use crate::{report::Report, tree};

#[derive(Default)]
pub struct Stats {
    pub renamed: usize,
    pub collisions: usize,
    pub subjects: usize,
}

/// Hardware facts an entry states about itself, as slug-shaped tokens.
trait SlugTokens: Platform + Sized {
    fn tokens(game: &Game<Self>) -> BTreeSet<String>;
}

impl SlugTokens for Vcs {
    fn tokens(game: &Game<Self>) -> BTreeSet<String> {
        let mut tokens = BTreeSet::new();
        for release in &game.releases {
            if let Some(format) = release.hardware.tv_format {
                tokens.insert(format!("{format:?}").to_lowercase());
            }
            if let Some(cart) = &release.hardware.cart_type {
                tokens.insert(cart.to_lowercase());
            }
        }
        tokens
    }
}

impl SlugTokens for GameBoy {
    fn tokens(game: &Game<Self>) -> BTreeSet<String> {
        mapper_tokens(game.releases.iter().map(|r| r.hardware.mapper.as_deref()))
    }
}

impl SlugTokens for GameBoyColor {
    fn tokens(game: &Game<Self>) -> BTreeSet<String> {
        mapper_tokens(game.releases.iter().map(|r| r.hardware.mapper.as_deref()))
    }
}

fn mapper_tokens<'a>(mappers: impl Iterator<Item = Option<&'a str>>) -> BTreeSet<String> {
    mappers
        .flatten()
        .map(|mapper| mapper.to_lowercase())
        .collect()
}

/// Strip every trailing `-token` the entry's hardware vouches for.
fn desired_slug(slug: &str, tokens: &BTreeSet<String>) -> String {
    let mut slug = slug;
    loop {
        let trimmed = tokens.iter().find_map(|token| {
            slug.strip_suffix(token)
                .and_then(|head| head.strip_suffix('-'))
                .filter(|head| !head.is_empty())
        });
        match trimmed {
            Some(head) => slug = head,
            None => return slug.to_owned(),
        }
    }
}

/// `repo_root` holds curation/flags.ron; `data_root` holds the platform trees.
pub fn run(
    repo_root: &Path,
    data_root: &Path,
    report: &mut Report,
    dry_run: bool,
) -> Result<Stats, String> {
    let mut stats = Stats::default();
    let mut flags = FlagFile::load(repo_root).map_err(|e| e.to_string())?;
    fix_tree::<GameBoy>(data_root, report, &mut stats, &mut flags, dry_run)?;
    fix_tree::<GameBoyColor>(data_root, report, &mut stats, &mut flags, dry_run)?;
    fix_tree::<Vcs>(data_root, report, &mut stats, &mut flags, dry_run)?;
    if !dry_run {
        flags.save(repo_root).map_err(|e| e.to_string())?;
    }
    Ok(stats)
}

fn fix_tree<P: SlugTokens>(
    db_root: &Path,
    report: &mut Report,
    stats: &mut Stats,
    flags: &mut FlagFile,
    dry_run: bool,
) -> Result<(), String> {
    let (tree, issues) = Tree::<P>::load(db_root).map_err(|e| e.to_string())?;
    if let Some(first) = issues.first() {
        return Err(format!("{}: {}", first.path.display(), first.message));
    }

    let games: Vec<(String, Game<P>)> = tree
        .games
        .into_iter()
        .map(|entry| (entry.slug.as_str().to_owned(), entry.game))
        .collect();
    let before = tree::sha1_multiset(&games);

    let wanted: Vec<String> = games
        .iter()
        .map(|(slug, game)| desired_slug(slug, &P::tokens(game)))
        .collect();

    // A slug an unmoved entry still occupies is not available to a mover.
    let held: BTreeSet<&str> = games
        .iter()
        .zip(&wanted)
        .filter(|((slug, _), want)| *slug == **want)
        .map(|((slug, _), _)| slug.as_str())
        .collect();
    let mut claims: HashMap<&str, usize> = HashMap::new();
    for want in &wanted {
        *claims.entry(want.as_str()).or_default() += 1;
    }

    let mut renames: Vec<(String, String)> = Vec::new();
    for ((slug, game), want) in games.iter().zip(&wanted) {
        if slug == want {
            continue;
        }
        if held.contains(want.as_str()) || claims[want.as_str()] > 1 {
            stats.collisions += 1;
            report.add(
                "Slug kept: bare slug is contested",
                format!("{}/{slug} → {}/{want}: {:?}", P::DIR, P::DIR, game.title),
            );
            continue;
        }
        renames.push((slug.clone(), want.clone()));
    }

    for (old, new) in &renames {
        let game = games
            .iter()
            .find(|(slug, _)| slug == old)
            .map(|(_, game)| game)
            .expect("rename source is a loaded entry");
        report.add(
            "Renamed",
            format!("{}/{old} → {}/{new}: {:?}", P::DIR, P::DIR, game.title),
        );
        if !dry_run {
            tree::write_game(db_root, new, game).map_err(|e| e.to_string())?;
            tree::remove_game_dir(db_root, P::DIR, old).map_err(|e| e.to_string())?;
        }
        stats.renamed += 1;
    }

    let moved: HashMap<String, String> = renames
        .iter()
        .map(|(old, new)| (format!("{}/{old}", P::DIR), format!("{}/{new}", P::DIR)))
        .collect();
    for flag in &mut flags.flags {
        for subject in &mut flag.subject {
            if let Some(new) = moved.get(subject.as_str()) {
                *subject = new.clone();
                stats.subjects += 1;
            }
        }
    }

    if dry_run {
        return Ok(());
    }

    let (reloaded, _) = Tree::<P>::load(db_root).map_err(|e| e.to_string())?;
    let after = tree::sha1_multiset(
        &reloaded
            .games
            .into_iter()
            .map(|entry| (entry.slug.as_str().to_owned(), entry.game))
            .collect::<Vec<_>>(),
    );
    if before != after {
        return Err(format!("{}: renames changed the dump set", P::DIR));
    }
    Ok(())
}
