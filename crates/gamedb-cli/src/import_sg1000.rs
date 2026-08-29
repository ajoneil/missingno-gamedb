//! Fold the No-Intro SG-1000 DAT into the sg1000 tree.
//!
//! The seeded entries state board and publisher facts no DAT knows, so a game
//! some DAT dump already reaches keeps every field it has and gains only the
//! releases the DAT adds.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::Path,
};

use missingno_gamedb::{
    Artifact, Game, GameKind, Language, Region, Release, Sg1000, Sg1000Hardware, Sha1, Tree,
    TvStandard,
};

use crate::{
    import_nointro::{
        ParsedName, Vocabulary, assign_slugs, canonical_title, merge_preproduction_families,
        parse_name,
    },
    report::Report,
    tree,
};

#[derive(Default)]
pub struct Stats {
    pub dat_entries: usize,
    pub computer_entries: usize,
    pub bios_entries: usize,
    pub new_games: usize,
    pub releases_added: usize,
    pub families_skipped: usize,
}

/// The SG-1000 DAT tags languages, and names logo and board variants the Game
/// Boy DATs never mention.
const VOCABULARY: Vocabulary = Vocabulary {
    languages: &[("Ja", Language::Japanese), ("En", Language::English)],
    known_labels: &[
        "Othello Multivision",
        "English Logo",
        "Chinese Logo",
        "Korean Logo",
        "No Logo",
    ],
};

/// Name tags that state SC-3000/SF-7000 computer software, whatever the dump's
/// extension says.
const COMPUTER_TAGS: [&str; 3] = ["(SC-3000)", "(SC3000)", "(SF-7000)"];

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

/// The standard the release's markets sold: the platform's NTSC markets make
/// software NTSC-authored, a PAL-market-only set is PAL-authored.
fn tv_format(regions: &[Region]) -> Option<TvStandard> {
    const NTSC_MARKETS: [Region; 3] = [Region::Japan, Region::Taiwan, Region::Korea];
    if regions.is_empty() {
        return None;
    }
    Some(if regions.iter().any(|r| NTSC_MARKETS.contains(r)) {
        TvStandard::Ntsc
    } else {
        TvStandard::Pal
    })
}

fn release(game_title: &str, parsed: &ParsedName, artifacts: &[Artifact]) -> Release<Sg1000> {
    Release {
        title: (parsed.title != game_title).then(|| parsed.title.clone()),
        label: parsed.label.clone(),
        regions: parsed.regions.clone(),
        languages: parsed.languages.clone(),
        date: parsed.date.clone(),
        publisher: None,
        status: parsed.status,
        hardware: Sg1000Hardware {
            tv_format: tv_format(&parsed.regions),
            cart_type: None,
        },
        artifacts: artifacts.to_vec(),
    }
}

pub fn run(db_root: &Path, dat_path: &Path, report: &mut Report) -> Result<Stats, String> {
    let mut stats = Stats::default();

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
    if !header_name.contains("SG-1000") {
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
        // A `.sc` dump is an SC-3000/SF-7000 computer program, not a console
        // cart — and a computer tag on the name overrules the extension.
        if COMPUTER_TAGS.iter().any(|tag| name.contains(tag))
            || !roms.iter().any(|rom| {
                rom.attribute("name")
                    .is_some_and(|n| n.to_ascii_lowercase().ends_with(".sg"))
            })
        {
            stats.computer_entries += 1;
            report.add(
                "SC-3000/SF-7000 program entries skipped",
                format!("{name:?}"),
            );
            continue;
        }
        stats.dat_entries += 1;
        let parsed = parse_name(name, &VOCABULARY, report);
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
    let (tree, issues) = Tree::<Sg1000>::load(db_root).map_err(|e| e.to_string())?;
    if let Some(first) = issues.first() {
        return Err(format!("{}: {}", first.path.display(), first.message));
    }
    let mut existing: Vec<(String, Game<Sg1000>)> = tree
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
    let mut fresh: Vec<(String, Game<Sg1000>)> = Vec::new();
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
                        .map(|&i| format!("sg1000/{}", existing[i].0))
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
                    format!("sg1000/{slug}: {name:?}"),
                );
                game.releases.push(release(&game.title, parsed, artifacts));
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
            Game::<Sg1000> {
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
                    .map(|(p, artifacts, _)| release(&title, p, artifacts))
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

#[cfg(test)]
mod tests {
    use missingno_gamedb::{Region, Sg1000CartType};

    use super::*;

    const OTHELLO_SHA1: &str = "d0cd594ddb321f707ddba8a044fa3e9b906e720a";

    const SEEDED: &str = r#"(
    title: "Othello",
    releases: [
        (
            regions: [
                Japan,
            ],
            date: Some("1985"),
            publisher: Some("Sega / Tsukuda Original"),
            hardware: (
                cart_type: Some(OthelloRam(
                    rom: Some(32768),
                )),
            ),
            artifacts: [
                (
                    sha1: "d0cd594ddb321f707ddba8a044fa3e9b906e720a",
                ),
            ],
        ),
    ],
)
"#;

    const DAT: &str = r#"<?xml version="1.0"?>
<datafile>
  <header><name>Sega - SG-1000 - SC-3000</name></header>
  <game name="Othello (Japan)" id="10">
    <rom name="Othello (Japan).sg" size="32768" sha1="d0cd594ddb321f707ddba8a044fa3e9b906e720a"/>
  </game>
  <game name="Othello (Japan) (Othello Multivision) (Unl)" id="11" cloneofid="10">
    <rom name="Othello (Japan) (Othello Multivision).sg" size="32768" sha1="1111111111111111111111111111111111111111"/>
  </game>
  <game name="Sky Jaguar (Japan)" id="20">
    <rom name="Sky Jaguar (Japan).sg" size="32768" sha1="2222222222222222222222222222222222222222"/>
  </game>
  <game name="Tian Kong Zhan Shi (Taiwan)" id="21" cloneofid="20">
    <rom name="Tian Kong Zhan Shi (Taiwan).sg" size="32768" sha1="3333333333333333333333333333333333333333"/>
  </game>
  <game name="Champion Golf (Japan) (Ja)" id="30">
    <rom name="Champion Golf (Japan).sg" size="16384" sha1="4444444444444444444444444444444444444444"/>
  </game>
  <game name="[BIOS] SG-1000 (Japan)" id="40">
    <rom name="bios.sg" size="8192" sha1="5555555555555555555555555555555555555555"/>
  </game>
  <game name="BASIC Level III (Japan)" id="50">
    <rom name="BASIC Level III (Japan).sc" size="32768" sha1="6666666666666666666666666666666666666666"/>
  </game>
  <game name="LinkWord (Japan) (Proto) (SC3000) (Program)" id="60">
    <rom name="LinkWord (Japan).sg" size="16384" sha1="7777777777777777777777777777777777777777"/>
  </game>
  <game name="Pal Homework (Europe, Australia, New Zealand)" id="70">
    <rom name="Pal Homework (Europe, Australia, New Zealand).sg" size="16384" sha1="8888888888888888888888888888888888888888"/>
  </game>
</datafile>"#;

    #[test]
    fn folds_into_the_seeded_tree_and_repeats_cleanly() {
        let root = tempfile::tempdir().unwrap();
        let othello_dir = root.path().join("sg1000/othello");
        std::fs::create_dir_all(&othello_dir).unwrap();
        std::fs::write(othello_dir.join("manifest.ron"), SEEDED).unwrap();
        let dat = root.path().join("sg1000.dat");
        std::fs::write(&dat, DAT).unwrap();

        let mut report = Report::default();
        let stats = run(root.path(), &dat, &mut report).unwrap();
        assert_eq!(stats.dat_entries, 6);
        assert_eq!(stats.bios_entries, 1);
        assert_eq!(stats.computer_entries, 2);
        assert_eq!(stats.new_games, 3);
        assert_eq!(stats.releases_added, 1);
        assert_eq!(stats.families_skipped, 0);
        assert!(!report.render("t").contains("Unknown name qualifiers"));

        let othello_path = othello_dir.join("manifest.ron");
        let othello = Game::<Sg1000>::from_ron(&std::fs::read_to_string(&othello_path).unwrap())
            .expect("the fold target still parses");
        assert_eq!(othello.releases.len(), 2);
        let seeded = &othello.releases[0];
        assert_eq!(seeded.publisher.as_deref(), Some("Sega / Tsukuda Original"));
        assert_eq!(
            seeded.hardware.cart_type,
            Some(Sg1000CartType::OthelloRam { rom: Some(32768) })
        );
        assert_eq!(seeded.artifacts[0].sha1.as_str(), OTHELLO_SHA1);
        let folded = &othello.releases[1];
        assert_eq!(folded.title, None);
        assert_eq!(folded.label.as_deref(), Some("Othello Multivision, Unl"));
        assert_eq!(folded.publisher, None);
        assert_eq!(folded.hardware.tv_format, Some(TvStandard::Ntsc));
        assert_eq!(folded.hardware.cart_type, None);

        let sky = std::fs::read_to_string(root.path().join("sg1000/sky-jaguar/manifest.ron"))
            .expect("a fresh family becomes one game");
        let sky = Game::<Sg1000>::from_ron(&sky).unwrap();
        assert_eq!(sky.title, "Sky Jaguar");
        assert_eq!(sky.releases.len(), 2);
        assert_eq!(sky.releases[1].title.as_deref(), Some("Tian Kong Zhan Shi"));
        assert_eq!(sky.releases[1].regions, vec![Region::Taiwan]);

        let golf =
            std::fs::read_to_string(root.path().join("sg1000/champion-golf/manifest.ron")).unwrap();
        let golf = Game::<Sg1000>::from_ron(&golf).unwrap();
        assert_eq!(golf.releases[0].languages, vec![Language::Japanese]);
        assert_eq!(golf.releases[0].label, None);
        assert_eq!(golf.releases[0].hardware.tv_format, Some(TvStandard::Ntsc));

        let pal =
            std::fs::read_to_string(root.path().join("sg1000/pal-homework/manifest.ron")).unwrap();
        let pal = Game::<Sg1000>::from_ron(&pal).unwrap();
        assert_eq!(pal.releases[0].hardware.tv_format, Some(TvStandard::Pal));

        assert!(!root.path().join("sg1000/basic-level-iii").exists());
        assert!(!root.path().join("sg1000/linkword").exists());
        assert!(!root.path().join("sg1000/sg-1000").exists());
        assert!(missingno_gamedb::validate(root.path()).unwrap().is_empty());

        let written = std::fs::read_to_string(&othello_path).unwrap();
        let mut report = Report::default();
        let stats = run(root.path(), &dat, &mut report).unwrap();
        assert_eq!(stats.dat_entries, 6);
        assert_eq!(stats.new_games, 0);
        assert_eq!(stats.releases_added, 0);
        assert_eq!(stats.families_skipped, 0);
        assert_eq!(std::fs::read_to_string(&othello_path).unwrap(), written);
    }
}
