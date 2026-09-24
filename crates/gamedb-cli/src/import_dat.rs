//! Fold a No-Intro DAT into its platform tree.
//!
//! A game some DAT dump already reaches keeps every field it has and gains
//! only the releases the DAT adds, so a curated or seeded entry survives a
//! re-import and a repeat run changes nothing.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use missingno_gamedb::{Artifact, Game, GameKind, Platform, Region, Release, Sha1, Tree};

use crate::{
    import_nointro::{
        ParsedName, Vocabulary, assign_slugs, canonical_title, merge_preproduction_families,
        parse_name,
    },
    report::Report,
    tree,
};

/// What one platform's DAT import states beyond the shared fold.
pub trait DatProfile: Platform {
    /// A substring the DAT header's name must carry.
    const HEADER_NAMES: &'static str;
    fn vocabulary() -> Vocabulary;
    /// Entries the tree does not take, as the report section naming why.
    fn skip(name: &str, rom_names: &[&str]) -> Option<&'static str>;
    fn hardware(regions: &[Region]) -> Self::ReleaseHardware;
}

#[derive(Default)]
pub struct Stats {
    pub dat_entries: usize,
    pub bios_entries: usize,
    pub profile_skipped: usize,
    pub new_games: usize,
    pub releases_added: usize,
    pub families_skipped: usize,
}

/// One DAT entry: what its name says, its dumps, and the raw name for reports.
type Member = (ParsedName, Vec<Artifact>, String);

/// The existing games some dump in this list already reaches.
fn covering<'a>(
    by_sha1: &'a BTreeMap<String, usize>,
    artifacts: &'a [Artifact],
) -> impl Iterator<Item = usize> + 'a {
    artifacts
        .iter()
        .filter_map(|a| by_sha1.get(a.sha1.as_str()).copied())
}

fn release<P: DatProfile>(
    game_title: &str,
    parsed: &ParsedName,
    artifacts: &[Artifact],
) -> Release<P> {
    Release {
        title: (parsed.title != game_title).then(|| parsed.title.clone()),
        label: parsed.label.clone(),
        regions: parsed.regions.clone(),
        languages: parsed.languages.clone(),
        date: parsed.date.clone(),
        publisher: None,
        status: parsed.status,
        hardware: P::hardware(&parsed.regions),
        artifacts: artifacts.to_vec(),
    }
}

pub fn import<P: DatProfile>(
    db_root: &Path,
    dat_path: &Path,
    report: &mut Report,
) -> Result<Stats, String> {
    let mut stats = Stats::default();
    let vocabulary = P::vocabulary();

    // ── Parse the DAT, grouped by clone family (cloneofid → parent id) ──
    let text = fs::read_to_string(dat_path).map_err(|e| format!("{dat_path:?}: {e}"))?;
    let doc = roxmltree::Document::parse(&text).map_err(|e| format!("{dat_path:?}: {e}"))?;
    let header_name = doc
        .descendants()
        .find(|n| n.has_tag_name("header"))
        .and_then(|h| h.children().find(|n| n.has_tag_name("name")))
        .and_then(|n| n.text())
        .unwrap_or_default()
        .to_owned();
    if !header_name.contains(P::HEADER_NAMES) {
        return Err(format!(
            "{dat_path:?}: unrecognized DAT header {header_name:?}"
        ));
    }

    let mut groups: BTreeMap<String, Vec<Member>> = BTreeMap::new();
    let mut family_title: BTreeMap<String, String> = BTreeMap::new();
    for game in doc.descendants().filter(|n| n.has_tag_name("game")) {
        let name = game.attribute("name").unwrap_or_default();
        if name.starts_with("[BIOS]") {
            stats.bios_entries += 1;
            report.add("BIOS entries skipped", format!("{name:?}"));
            continue;
        }
        let roms: Vec<_> = game.children().filter(|n| n.has_tag_name("rom")).collect();
        let rom_names: Vec<&str> = roms.iter().filter_map(|r| r.attribute("name")).collect();
        if let Some(section) = P::skip(name, &rom_names) {
            stats.profile_skipped += 1;
            report.add(section, format!("{name:?}"));
            continue;
        }
        stats.dat_entries += 1;
        let parsed = parse_name(name, &vocabulary, report);
        let mut artifacts = Vec::new();
        for rom in roms {
            let Some(sha1) = rom.attribute("sha1") else {
                report.add("ROMs without sha1 skipped", format!("{name:?}"));
                continue;
            };
            match sha1.parse::<Sha1>() {
                Ok(sha1) => artifacts.push(Artifact {
                    sha1,
                    label: None,
                    defect: None,
                }),
                Err(e) => report.add("Invalid DAT sha1 skipped", format!("{name:?}: {e}")),
            }
        }
        if artifacts.is_empty() {
            report.add("DAT entries with no usable rom", format!("{name:?}"));
            continue;
        }
        let id = game.attribute("id").unwrap_or(name);
        let family = game.attribute("cloneofid").unwrap_or(id).to_owned();
        if game.attribute("cloneofid").is_none() {
            family_title.insert(family.clone(), parsed.title.clone());
        }
        groups
            .entry(family)
            .or_default()
            .push((parsed, artifacts, name.to_owned()));
    }
    merge_preproduction_families(&mut groups, &family_title, |m| &m.0, report);

    // ── Load the tree the DAT folds into ────────────────────────────────
    let (tree, issues) = Tree::<P>::load(db_root).map_err(|e| e.to_string())?;
    if let Some(first) = issues.first() {
        return Err(format!("{}: {}", first.path.display(), first.message));
    }
    let mut existing: Vec<(String, Game<P>)> = tree
        .games
        .into_iter()
        .map(|entry| (entry.slug.as_str().to_owned(), entry.game))
        .collect();
    let before = tree::sha1_multiset(&existing);
    let mut by_sha1: BTreeMap<String, usize> = BTreeMap::new();
    for (i, (_, game)) in existing.iter().enumerate() {
        for release in &game.releases {
            for artifact in &release.artifacts {
                by_sha1.insert(artifact.sha1.as_str().to_owned(), i);
            }
        }
    }

    // ── Route each family: fold into the game it reaches, or file a new one ──
    let mut gained: BTreeSet<usize> = BTreeSet::new();
    let mut fresh: Vec<(String, Game<P>)> = Vec::new();
    for (family, group) in &groups {
        let title = canonical_title(family, group, &family_title, &|m: &Member| &m.0);
        let targets: BTreeSet<usize> = group
            .iter()
            .flat_map(|(_, artifacts, _)| covering(&by_sha1, artifacts))
            .collect();
        if targets.len() > 1 {
            stats.families_skipped += 1;
            report.add(
                "Families spanning multiple existing games — skipped for manual review",
                format!(
                    "{title:?} → {}",
                    targets
                        .iter()
                        .map(|&i| format!("{}/{}", P::DIR, existing[i].0))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
            continue;
        }

        if let Some(&target) = targets.iter().next() {
            for (parsed, artifacts, name) in group {
                if covering(&by_sha1, artifacts).next().is_some() {
                    continue;
                }
                let (slug, game) = &mut existing[target];
                report.add(
                    "Existing games gained releases",
                    format!("{}/{slug}: {name:?}", P::DIR),
                );
                game.releases
                    .push(release::<P>(&game.title, parsed, artifacts));
                stats.releases_added += 1;
                gained.insert(target);
            }
            continue;
        }

        let mut members: Vec<&Member> = group.iter().collect();
        members.sort_by_key(|(p, artifacts, _)| {
            (
                p.label.clone().unwrap_or_default(),
                format!("{:?}", p.regions),
                artifacts[0].sha1.as_str().to_owned(),
            )
        });
        let kind = members
            .iter()
            .map(|(p, ..)| p.kind)
            .find(|k| *k != GameKind::Game)
            .unwrap_or(GameKind::Game);
        stats.new_games += 1;
        fresh.push((
            String::new(),
            Game::<P> {
                title: title.clone(),
                kind,
                developer: None,
                description: None,
                tags: Vec::new(),
                links: Vec::new(),
                covers: Vec::new(),
                screenshots: Vec::new(),
                mod_of: None,
                mods: Vec::new(),
                curated: false,
                adult: false,
                recommended_by: Vec::new(),
                releases: members
                    .iter()
                    .map(|(p, artifacts, _)| release::<P>(&title, p, artifacts))
                    .collect(),
            },
        ));
    }
    assign_slugs(db_root, &mut fresh, &[], report)?;

    // ── Invariant: the fold never drops a dump the tree already had ─────
    let mut after = tree::sha1_multiset(&existing);
    after.extend(tree::sha1_multiset(&fresh));
    let after: BTreeSet<String> = after.into_iter().collect();
    for sha1 in &before {
        if !after.contains(sha1) {
            return Err(format!("sha1 preservation violated: {sha1} lost"));
        }
    }

    for &i in &gained {
        let (slug, game) = &existing[i];
        tree::write_game(db_root, slug, game).map_err(|e| e.to_string())?;
    }
    for (slug, game) in &fresh {
        tree::write_game(db_root, slug, game).map_err(|e| e.to_string())?;
    }
    Ok(stats)
}
