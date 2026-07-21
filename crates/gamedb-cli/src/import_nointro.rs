use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::{Path, PathBuf},
};

use missingno_gamedb::{
    Artifact, Enhancement, Game, GameBoy, GameBoyColor, GameKind, GbHardware, Platform, Release,
    ReleaseDate, Sha1,
};

use crate::{
    legacy::{self, LegacyManifest},
    report::Report,
    text::{parse_region, parse_region_list, slugify},
    tree,
};

#[derive(Default)]
pub struct Stats {
    pub dat_entries: usize,
    pub gb_games: usize,
    pub gbc_games: usize,
    pub moved_to_gb: usize,
    pub merged: usize,
    pub leftovers: usize,
}

#[derive(PartialEq, Clone, Copy)]
enum DatSystem {
    GameBoy,
    GameBoyColor,
}

struct ParsedName {
    title: String,
    regions: Vec<missingno_gamedb::Region>,
    label: Option<String>,
    date: Option<ReleaseDate>,
    kind: GameKind,
    sgb: bool,
    cgb: bool,
    gb_compatible: bool,
}

/// Split a No-Intro game name into title, region set, and qualifier tags.
fn parse_name(name: &str, report: &mut Report) -> ParsedName {
    let mut chunks = Vec::new();
    let mut title = String::new();
    let mut current = String::new();
    let mut depth = 0usize;
    for c in name.chars() {
        match c {
            '(' => {
                depth += 1;
                if depth == 1 {
                    continue;
                }
            }
            ')' => {
                depth -= 1;
                if depth == 0 {
                    chunks.push(std::mem::take(&mut current));
                    continue;
                }
            }
            _ => {}
        }
        if depth == 0 {
            title.push(c);
        } else {
            current.push(c);
        }
    }
    let title = title.trim().to_owned();

    let mut parsed = ParsedName {
        title,
        regions: Vec::new(),
        label: None,
        date: None,
        kind: GameKind::Game,
        sgb: false,
        cgb: false,
        gb_compatible: false,
    };

    let mut label_parts: Vec<String> = Vec::new();
    for (i, chunk) in chunks.iter().enumerate() {
        // The first parenthetical is the region set when every word maps.
        if i == 0 {
            let words: Vec<&str> = chunk.split(',').map(str::trim).collect();
            if words.iter().all(|w| parse_region(w).is_some()) {
                parsed.regions = words
                    .iter()
                    .filter_map(|w| parse_region(w).flatten())
                    .collect();
                continue;
            }
            report.add("Names without a leading region tag", format!("{name:?}"));
        }
        match chunk.as_str() {
            "SGB Enhanced" => parsed.sgb = true,
            "CGB Enhanced" => parsed.cgb = true,
            "CGB+SGB Enhanced" => {
                parsed.sgb = true;
                parsed.cgb = true;
            }
            "GB Compatible" => parsed.gb_compatible = true,
            "Demo" | "Sample" | "Kiosk" => parsed.kind = GameKind::Demo,
            other => {
                if let Ok(date) = other.parse::<ReleaseDate>() {
                    parsed.date = Some(date);
                    continue;
                }
                let known_label = [
                    "Rev", "Beta", "Proto", "Demo", "Sample", "Kiosk", "Promo", "Unl", "Pirate",
                    "Alt", "NP",
                ]
                .iter()
                .any(|prefix| other == *prefix || other.starts_with(&format!("{prefix} ")));
                if known_label {
                    if other.starts_with("Demo")
                        || other.starts_with("Sample")
                        || other.starts_with("Kiosk")
                    {
                        parsed.kind = GameKind::Demo;
                    }
                } else {
                    report.add("Unknown name qualifiers", format!("{other:?} in {name:?}"));
                }
                label_parts.push(other.to_owned());
            }
        }
    }
    if !label_parts.is_empty() {
        parsed.label = Some(label_parts.join(", "));
    }
    parsed
}

pub fn run(db_root: &Path, dats: &[PathBuf], report: &mut Report) -> Result<Stats, String> {
    let mut stats = Stats::default();

    // ── Parse DATs, route each entry to its destination tree ─────────
    let mut gb_groups: BTreeMap<String, Vec<(ParsedName, Vec<Artifact>, bool)>> = BTreeMap::new();
    let mut gbc_groups: BTreeMap<String, Vec<(ParsedName, Vec<Artifact>)>> = BTreeMap::new();

    for dat_path in dats {
        let text = fs::read_to_string(dat_path).map_err(|e| format!("{dat_path:?}: {e}"))?;
        let doc = roxmltree::Document::parse(&text).map_err(|e| format!("{dat_path:?}: {e}"))?;
        let header_name = doc
            .descendants()
            .find(|n| n.has_tag_name("header"))
            .and_then(|h| h.children().find(|n| n.has_tag_name("name")))
            .and_then(|n| n.text())
            .unwrap_or_default()
            .to_owned();
        let system = if header_name.contains("Game Boy Color") {
            DatSystem::GameBoyColor
        } else if header_name.contains("Game Boy") {
            DatSystem::GameBoy
        } else {
            return Err(format!(
                "{dat_path:?}: unrecognized DAT header {header_name:?}"
            ));
        };

        for game in doc.descendants().filter(|n| n.has_tag_name("game")) {
            let name = game.attribute("name").unwrap_or_default();
            if name.starts_with("[BIOS]") {
                report.add("BIOS entries skipped", format!("{name:?}"));
                continue;
            }
            stats.dat_entries += 1;
            let parsed = parse_name(name, report);
            let mut artifacts = Vec::new();
            for rom in game.children().filter(|n| n.has_tag_name("rom")) {
                let Some(sha1) = rom.attribute("sha1") else {
                    report.add("ROMs without sha1 skipped", format!("{name:?}"));
                    continue;
                };
                match sha1.parse::<Sha1>() {
                    Ok(sha1) => artifacts.push(Artifact {
                        sha1,
                        size: rom.attribute("size").and_then(|s| s.parse().ok()),
                        filename: rom.attribute("name").map(str::to_owned),
                    }),
                    Err(e) => report.add("Invalid DAT sha1 skipped", format!("{name:?}: {e}")),
                }
            }
            if artifacts.is_empty() {
                report.add("DAT entries with no usable rom", format!("{name:?}"));
                continue;
            }
            match system {
                DatSystem::GameBoy => {
                    gb_groups
                        .entry(parsed.title.clone())
                        .or_default()
                        .push((parsed, artifacts, false));
                }
                DatSystem::GameBoyColor if parsed.gb_compatible => {
                    stats.moved_to_gb += 1;
                    gb_groups
                        .entry(parsed.title.clone())
                        .or_default()
                        .push((parsed, artifacts, true));
                }
                DatSystem::GameBoyColor => {
                    gbc_groups
                        .entry(parsed.title.clone())
                        .or_default()
                        .push((parsed, artifacts));
                }
            }
        }
    }

    // ── Load old commercial entries for merge + leftovers ────────────
    let mut old_commercial: Vec<(String, String, LegacyManifest)> = Vec::new(); // (tree, slug, m)
    for (tree_name, entries) in [
        ("gb", legacy::load_tree::<GameBoy>(db_root, true)),
        ("gbc", legacy::load_tree::<GameBoyColor>(db_root, true)),
    ] {
        match entries.map_err(|e| e.to_string())? {
            Ok(entries) => {
                for (slug, m) in entries {
                    if m.source.is_none() {
                        old_commercial.push((tree_name.to_owned(), slug, m));
                    }
                }
            }
            Err(failures) => {
                return Err(format!(
                    "{} legacy manifests failed to parse; first: {}: {}",
                    failures.len(),
                    failures[0].0,
                    failures[0].1
                ));
            }
        }
    }
    let mut old_by_sha1: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, (_, _, m)) in old_commercial.iter().enumerate() {
        for hash in &m.hashes {
            old_by_sha1
                .entry(hash.to_ascii_lowercase())
                .or_default()
                .push(i);
        }
    }
    let mut old_claimed: BTreeSet<usize> = BTreeSet::new();

    // ── Build destination games ──────────────────────────────────────
    let build = |title: &str,
                 entries: &mut Vec<(ParsedName, Vec<Artifact>, GbHardware)>,
                 old_commercial: &Vec<(String, String, LegacyManifest)>,
                 old_by_sha1: &BTreeMap<String, Vec<usize>>,
                 old_claimed: &mut BTreeSet<usize>,
                 stats: &mut Stats,
                 report: &mut Report| {
        entries.sort_by_key(|(p, artifacts, _)| {
            (
                p.label.clone().unwrap_or_default(),
                format!("{:?}", p.regions),
                artifacts[0].sha1.as_str().to_owned(),
            )
        });
        let kind = entries
            .iter()
            .map(|(p, ..)| p.kind)
            .find(|k| *k != GameKind::Game)
            .unwrap_or(GameKind::Game);
        let mut matches: Vec<usize> = entries
            .iter()
            .flat_map(|(_, artifacts, _)| artifacts)
            .filter_map(|a| old_by_sha1.get(a.sha1.as_str()))
            .flatten()
            .copied()
            .collect();
        matches.sort();
        matches.dedup();
        if matches.len() > 1 {
            report.add(
                "Multiple old entries merged into one game",
                format!(
                    "{title:?} ← {}",
                    matches
                        .iter()
                        .map(|&i| format!("{}/{}", old_commercial[i].0, old_commercial[i].1))
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
            );
        }
        let merged = matches.first().map(|&i| &old_commercial[i].2);
        if merged.is_some() {
            stats.merged += 1;
        }
        for &i in &matches {
            old_claimed.insert(i);
        }
        let releases: Vec<Release<GameBoy>> = entries
            .iter()
            .map(|(p, artifacts, hardware)| Release {
                label: p.label.clone(),
                regions: p.regions.clone(),
                date: p.date.clone(),
                publisher: merged.and_then(|m| m.publisher.clone()),
                hardware: hardware.clone(),
                sources: Vec::new(),
                artifacts: artifacts.clone(),
            })
            .collect();
        Game::<GameBoy> {
            title: title.to_owned(),
            kind,
            developer: merged.and_then(|m| m.developer.clone()),
            description: merged.and_then(|m| m.description.clone()),
            license: merged.and_then(|m| m.license.clone()),
            tags: merged.map(|m| m.tags.clone()).unwrap_or_default(),
            links: merged.map(|m| m.links.clone()).unwrap_or_default(),
            covers: Vec::new(),
            screenshots: Vec::new(),
            mod_of: None,
            releases,
        }
    };

    let mut gb_games: Vec<(String, Game<GameBoy>)> = Vec::new();
    for (title, group) in &gb_groups {
        let mut entries: Vec<(ParsedName, Vec<Artifact>, GbHardware)> = Vec::new();
        for (parsed, artifacts, from_gbc_dat) in group {
            let hardware = GbHardware {
                sgb: if parsed.sgb {
                    Enhancement::Enhanced
                } else {
                    Enhancement::Unknown
                },
                cgb: if parsed.cgb || *from_gbc_dat {
                    Enhancement::Enhanced
                } else {
                    Enhancement::NotEnhanced
                },
            };
            entries.push((
                ParsedName {
                    title: parsed.title.clone(),
                    regions: parsed.regions.clone(),
                    label: parsed.label.clone(),
                    date: parsed.date.clone(),
                    kind: parsed.kind,
                    sgb: parsed.sgb,
                    cgb: parsed.cgb,
                    gb_compatible: parsed.gb_compatible,
                },
                artifacts.clone(),
                hardware,
            ));
        }
        let game = build(
            title,
            &mut entries,
            &old_commercial,
            &old_by_sha1,
            &mut old_claimed,
            &mut stats,
            report,
        );
        gb_games.push((String::new(), game));
    }

    let mut gbc_games: Vec<(String, Game<GameBoyColor>)> = Vec::new();
    for (title, group) in &gbc_groups {
        let mut entries: Vec<(ParsedName, Vec<Artifact>, GbHardware)> = group
            .iter()
            .map(|(p, artifacts)| {
                (
                    ParsedName {
                        title: p.title.clone(),
                        regions: p.regions.clone(),
                        label: p.label.clone(),
                        date: p.date.clone(),
                        kind: p.kind,
                        sgb: p.sgb,
                        cgb: p.cgb,
                        gb_compatible: p.gb_compatible,
                    },
                    artifacts.clone(),
                    GbHardware::default(),
                )
            })
            .collect();
        let game = build(
            title,
            &mut entries,
            &old_commercial,
            &old_by_sha1,
            &mut old_claimed,
            &mut stats,
            report,
        );
        gbc_games.push((
            String::new(),
            Game::<GameBoyColor> {
                title: game.title,
                kind: game.kind,
                developer: game.developer,
                description: game.description,
                license: game.license,
                tags: game.tags,
                links: game.links,
                covers: game.covers,
                screenshots: game.screenshots,
                mod_of: None,
                releases: game
                    .releases
                    .into_iter()
                    .map(|r| Release {
                        label: r.label,
                        regions: r.regions,
                        date: r.date,
                        publisher: r.publisher,
                        hardware: Default::default(),
                        sources: r.sources,
                        artifacts: r.artifacts,
                    })
                    .collect(),
            },
        ));
    }

    // ── Leftovers: old commercial entries no DAT entry claimed ───────
    let mut gb_leftovers: Vec<(String, Game<GameBoy>)> = Vec::new();
    let mut gbc_leftovers: Vec<(String, Game<GameBoyColor>)> = Vec::new();
    for (i, (tree_name, slug, m)) in old_commercial.iter().enumerate() {
        if old_claimed.contains(&i) {
            continue;
        }
        stats.leftovers += 1;
        report.add(
            "Leftover old entries (not in any DAT) — mechanically converted",
            format!("{tree_name}/{slug}"),
        );
        let regions = match &m.region {
            None => Vec::new(),
            Some(text) => parse_region_list(text).unwrap_or_else(|unknown| {
                report.add(
                    "Unmappable region text dropped",
                    format!("{tree_name}/{slug}: {unknown:?}"),
                );
                Vec::new()
            }),
        };
        let artifacts: Vec<Artifact> = m
            .hashes
            .iter()
            .filter_map(|h| h.parse::<Sha1>().ok())
            .map(|sha1| Artifact {
                sha1,
                size: None,
                filename: None,
            })
            .collect();
        macro_rules! leftover {
            ($ptype:ty) => {
                Game::<$ptype> {
                    title: m.title.clone(),
                    kind: Default::default(),
                    developer: m.developer.clone(),
                    description: m.description.clone(),
                    license: m.license.clone(),
                    tags: m.tags.clone(),
                    links: m.links.clone(),
                    covers: Vec::new(),
                    screenshots: Vec::new(),
                    mod_of: None,
                    releases: vec![Release {
                        label: None,
                        regions: regions.clone(),
                        date: None,
                        publisher: m.publisher.clone(),
                        hardware: Default::default(),
                        sources: Vec::new(),
                        artifacts: artifacts.clone(),
                    }],
                }
            };
        }
        if tree_name == "gb" {
            gb_leftovers.push((slug.clone(), leftover!(GameBoy)));
        } else {
            gbc_leftovers.push((slug.clone(), leftover!(GameBoyColor)));
        }
    }

    // ── Assign slugs (existing homebrew dirs + leftovers are reserved) ─
    fn assign_slugs<P: Platform>(
        db_root: &Path,
        games: &mut [(String, Game<P>)],
        reserved: &[String],
        report: &mut Report,
    ) -> Result<(), String> {
        let mut taken: BTreeSet<String> = reserved.iter().cloned().collect();
        let tree_dir = db_root.join(P::DIR);
        if tree_dir.is_dir() {
            for dir in fs::read_dir(&tree_dir)
                .map_err(|e| e.to_string())?
                .flatten()
            {
                let manifest = dir.path().join("manifest.ron");
                if manifest.is_file() {
                    let text = fs::read_to_string(&manifest).map_err(|e| e.to_string())?;
                    if Game::<P>::from_ron(&text).is_ok() {
                        taken.insert(dir.file_name().to_string_lossy().into_owned());
                    }
                }
            }
        }
        for (slug, game) in games.iter_mut() {
            let base = slugify(&game.title);
            let mut candidate = base.clone();
            let mut n = 1;
            while taken.contains(&candidate) {
                n += 1;
                candidate = format!("{base}-{n}");
            }
            if n > 1 {
                report.add(
                    "Slug collisions suffixed",
                    format!("{}/{candidate} for {:?}", P::DIR, game.title),
                );
            }
            taken.insert(candidate.clone());
            *slug = candidate;
        }
        Ok(())
    }
    let gb_reserved: Vec<String> = gb_leftovers.iter().map(|(s, _)| s.clone()).collect();
    let gbc_reserved: Vec<String> = gbc_leftovers.iter().map(|(s, _)| s.clone()).collect();
    assign_slugs(db_root, &mut gb_games, &gb_reserved, report)?;
    assign_slugs(db_root, &mut gbc_games, &gbc_reserved, report)?;
    gb_games.extend(gb_leftovers);
    gbc_games.extend(gbc_leftovers);

    // ── Invariant: every old commercial sha1 survives ────────────────
    let mut new_sha1s: BTreeSet<String> = BTreeSet::new();
    for sha1 in tree::sha1_multiset(&gb_games) {
        new_sha1s.insert(sha1);
    }
    for sha1 in tree::sha1_multiset(&gbc_games) {
        new_sha1s.insert(sha1);
    }
    for (i, (tree_name, slug, m)) in old_commercial.iter().enumerate() {
        for hash in &m.hashes {
            let hash = hash.to_ascii_lowercase();
            if hash.parse::<Sha1>().is_ok() && !new_sha1s.contains(&hash) {
                // A claimed entry can carry hashes the current DATs retired;
                // dropping them is allowed only out loud, via the report.
                if old_claimed.contains(&i) {
                    report.add(
                        "Old hashes absent from current DATs — dropped",
                        format!("{tree_name}/{slug}: {hash}"),
                    );
                } else {
                    return Err(format!(
                        "sha1 preservation violated: {hash} from {tree_name}/{slug} lost"
                    ));
                }
            }
        }
    }

    // ── Write: remove old commercial dirs, write the new games ───────
    for (tree_name, slug, _) in &old_commercial {
        tree::remove_game_dir(db_root, tree_name, slug).map_err(|e| e.to_string())?;
    }
    for (slug, game) in &gb_games {
        tree::write_game(db_root, slug, game).map_err(|e| e.to_string())?;
    }
    for (slug, game) in &gbc_games {
        tree::write_game(db_root, slug, game).map_err(|e| e.to_string())?;
    }
    stats.gb_games = gb_games.len();
    stats.gbc_games = gbc_games.len();
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    const GB_DAT: &str = r#"<?xml version="1.0"?>
<datafile>
  <header><name>Nintendo - Game Boy</name></header>
  <game name="Zelda (USA) (SGB Enhanced)">
    <rom name="Zelda (USA).gb" size="524288" sha1="0123456789abcdef0123456789abcdef01234567"/>
  </game>
  <game name="Zelda (Europe) (Rev A) (SGB Enhanced)">
    <rom name="Zelda (Europe).gb" size="524288" sha1="fedcba9876543210fedcba9876543210fedcba98"/>
  </game>
  <game name="[BIOS] Boot ROM (World)">
    <rom name="boot.gb" size="256" sha1="4ed31ec6b0b175bb109c0eb5fd3d193da823339f"/>
  </game>
</datafile>"#;

    const GBC_DAT: &str = r#"<?xml version="1.0"?>
<datafile>
  <header><name>Nintendo - Game Boy Color</name></header>
  <game name="Dual Game (USA) (GB Compatible)">
    <rom name="Dual Game (USA).gbc" size="1048576" sha1="89abcdef0123456789abcdef0123456789abcdef"/>
  </game>
  <game name="Colors Only (USA) (Demo)">
    <rom name="Colors Only (USA).gbc" size="1048576" sha1="1111111189abcdef0123456789abcdef01234567"/>
  </game>
</datafile>"#;

    #[test]
    fn imports_routes_and_merges() {
        let root = tempfile::tempdir().unwrap();
        let gb_dir = root.path().join("gb/zelda-old");
        std::fs::create_dir_all(&gb_dir).unwrap();
        std::fs::write(
            gb_dir.join("manifest.ron"),
            r#"(title: "Zelda (USA)", region: Some("USA"), description: Some("A classic."), hashes: ["0123456789abcdef0123456789abcdef01234567"], source: None)"#,
        )
        .unwrap();
        std::fs::create_dir_all(root.path().join("gbc")).unwrap();
        let gb_dat = root.path().join("gb.dat");
        let gbc_dat = root.path().join("gbc.dat");
        std::fs::write(&gb_dat, GB_DAT).unwrap();
        std::fs::write(&gbc_dat, GBC_DAT).unwrap();

        let mut report = Report::default();
        let stats = run(root.path(), &[gb_dat, gbc_dat], &mut report).unwrap();
        assert_eq!(stats.moved_to_gb, 1);
        assert_eq!(stats.merged, 1);
        assert_eq!(stats.leftovers, 0);

        let zelda = std::fs::read_to_string(root.path().join("gb/zelda/manifest.ron")).unwrap();
        let zelda = Game::<GameBoy>::from_ron(&zelda).unwrap();
        assert_eq!(zelda.title, "Zelda");
        assert_eq!(zelda.description.as_deref(), Some("A classic."));
        assert_eq!(zelda.releases.len(), 2);
        assert!(
            zelda
                .releases
                .iter()
                .any(|r| r.label.as_deref() == Some("Rev A"))
        );
        assert!(
            zelda
                .releases
                .iter()
                .all(|r| r.hardware.sgb == Enhancement::Enhanced
                    && r.hardware.cgb == Enhancement::NotEnhanced)
        );
        assert!(!root.path().join("gb/zelda-old").exists());

        let dual = std::fs::read_to_string(root.path().join("gb/dual-game/manifest.ron")).unwrap();
        let dual = Game::<GameBoy>::from_ron(&dual).unwrap();
        assert_eq!(dual.releases[0].hardware.cgb, Enhancement::Enhanced);

        let demo =
            std::fs::read_to_string(root.path().join("gbc/colors-only/manifest.ron")).unwrap();
        let demo = Game::<GameBoyColor>::from_ron(&demo).unwrap();
        assert_eq!(demo.kind, GameKind::Demo);

        assert!(missingno_gamedb::validate(root.path()).unwrap().is_empty());
    }
}
