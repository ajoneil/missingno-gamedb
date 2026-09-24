//! Fold the No-Intro SG-1000 DAT into the sg1000 tree.
//!
//! The seeded entries state board and publisher facts no DAT knows, so a game
//! some DAT dump already reaches keeps every field it has and gains only the
//! releases the DAT adds.

use std::path::Path;

use missingno_gamedb::{Language, Region, Sg1000, Sg1000Hardware, TvStandard};

use crate::{
    import_dat::{self, DatProfile},
    import_nointro::Vocabulary,
    report::Report,
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

/// Name tags that state SC-3000/SF-7000 computer software, whatever the dump's
/// extension says.
const COMPUTER_TAGS: [&str; 3] = ["(SC-3000)", "(SC3000)", "(SF-7000)"];

impl DatProfile for Sg1000 {
    const HEADER_NAMES: &'static str = "SG-1000";

    /// The SG-1000 DAT tags languages, and names logo and board variants the
    /// Game Boy DATs never mention.
    fn vocabulary() -> Vocabulary {
        Vocabulary {
            languages: &[("Ja", Language::Japanese), ("En", Language::English)],
            known_labels: &[
                "Othello Multivision",
                "English Logo",
                "Chinese Logo",
                "Korean Logo",
                "No Logo",
            ],
        }
    }

    /// A `.sc` dump is an SC-3000/SF-7000 computer program, not a console
    /// cart — and a computer tag on the name overrules the extension.
    fn skip(name: &str, rom_names: &[&str]) -> Option<&'static str> {
        (COMPUTER_TAGS.iter().any(|tag| name.contains(tag))
            || !rom_names
                .iter()
                .any(|n| n.to_ascii_lowercase().ends_with(".sg")))
        .then_some("SC-3000/SF-7000 program entries skipped")
    }

    fn hardware(regions: &[Region]) -> Sg1000Hardware {
        Sg1000Hardware {
            tv_format: tv_format(regions),
            cart_type: None,
        }
    }
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

pub fn run(db_root: &Path, dat_path: &Path, report: &mut Report) -> Result<Stats, String> {
    let stats = import_dat::import::<Sg1000>(db_root, dat_path, report)?;
    Ok(Stats {
        dat_entries: stats.dat_entries,
        computer_entries: stats.profile_skipped,
        bios_entries: stats.bios_entries,
        new_games: stats.new_games,
        releases_added: stats.releases_added,
        families_skipped: stats.families_skipped,
    })
}

#[cfg(test)]
mod tests {
    use missingno_gamedb::{Game, Region, Sg1000CartType};

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
