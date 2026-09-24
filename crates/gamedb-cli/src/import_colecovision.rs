//! Fold the No-Intro ColecoVision DAT into the colecovision tree.

use std::path::Path;

use missingno_gamedb::{ColecoVision, ColecoVisionHardware, Language, Region, TvStandard};

use crate::{
    import_dat::{self, DatProfile, Stats},
    import_nointro::Vocabulary,
    report::Report,
};

impl DatProfile for ColecoVision {
    const HEADER_NAMES: &'static str = "ColecoVision";

    fn vocabulary() -> Vocabulary {
        Vocabulary {
            languages: &[("En", Language::English), ("Fr-CA", Language::French)],
            known_labels: &[
                "v1.1",
                "BUG Fixed",
                "Program",
                "Standard Controller",
                "Super Action Controller",
            ],
        }
    }

    fn skip(_name: &str, _rom_names: &[&str]) -> Option<&'static str> {
        None
    }

    fn hardware(regions: &[Region]) -> ColecoVisionHardware {
        ColecoVisionHardware {
            tv_format: tv_format(regions),
        }
    }
}

/// The standard the release's markets sold. Brazil's machines presented PAL-M,
/// so a Brazil-only release is PAL-M-authored.
fn tv_format(regions: &[Region]) -> Option<TvStandard> {
    const NTSC_MARKETS: [Region; 5] = [
        Region::Usa,
        Region::Canada,
        Region::Japan,
        Region::Taiwan,
        Region::Korea,
    ];
    match regions {
        [] => None,
        r if r.iter().any(|r| NTSC_MARKETS.contains(r)) => Some(TvStandard::Ntsc),
        r if r.iter().all(|r| *r == Region::Brazil) => Some(TvStandard::PalM),
        _ => Some(TvStandard::Pal),
    }
}

pub fn run(db_root: &Path, dat_path: &Path, report: &mut Report) -> Result<Stats, String> {
    import_dat::import::<ColecoVision>(db_root, dat_path, report)
}

#[cfg(test)]
mod tests {
    use missingno_gamedb::Game;

    use super::*;

    const DAT: &str = r#"<?xml version="1.0"?>
<datafile>
  <header><name>Coleco - ColecoVision</name></header>
  <game name="[BIOS] ColecoVision (USA, Europe)" id="0">
    <rom name="ColecoVision.col" size="8192" sha1="1111111111111111111111111111111111111111"/>
  </game>
  <game name="Zenji (USA)" id="10">
    <rom name="Zenji (USA).col" size="16384" sha1="2222222222222222222222222222222222222222"/>
  </game>
  <game name="Zenji (USA) (Beta)" id="11" cloneofid="10">
    <rom name="Zenji (USA) (Beta).col" size="16384" sha1="3333333333333333333333333333333333333333"/>
  </game>
  <game name="Tank Wars (Europe)" id="20">
    <rom name="Tank Wars (Europe).col" size="16384" sha1="4444444444444444444444444444444444444444"/>
  </game>
  <game name="Castelo (Brazil) (En) (Unl)" id="30">
    <rom name="Castelo (Brazil) (En) (Unl).col" size="8192" sha1="5555555555555555555555555555555555555555"/>
  </game>
  <game name="Energy Quiz (Canada) (En,Fr-CA) (1983-06-06) (Proto)" id="40">
    <rom name="Energy Quiz.col" size="8192" sha1="6666666666666666666666666666666666666666"/>
  </game>
</datafile>"#;

    fn load(root: &Path, slug: &str) -> Game<ColecoVision> {
        let path = root.join("colecovision").join(slug).join("manifest.ron");
        Game::from_ron(&std::fs::read_to_string(path).unwrap()).unwrap()
    }

    #[test]
    fn markets_state_the_standard_and_a_repeat_changes_nothing() {
        let root = tempfile::tempdir().unwrap();
        let dat = root.path().join("coleco.dat");
        std::fs::write(&dat, DAT).unwrap();

        let mut report = Report::default();
        let stats = run(root.path(), &dat, &mut report).unwrap();
        assert_eq!(stats.bios_entries, 1);
        assert_eq!(stats.dat_entries, 5);
        assert_eq!(stats.new_games, 4);
        assert!(!report.render("t").contains("Unknown name qualifiers"));

        let zenji = load(root.path(), "zenji");
        assert_eq!(zenji.releases.len(), 2);
        assert_eq!(zenji.releases[0].hardware.tv_format, Some(TvStandard::Ntsc));
        assert_eq!(
            load(root.path(), "tank-wars").releases[0]
                .hardware
                .tv_format,
            Some(TvStandard::Pal)
        );
        assert_eq!(
            load(root.path(), "castelo").releases[0].hardware.tv_format,
            Some(TvStandard::PalM)
        );
        let quiz = load(root.path(), "energy-quiz").releases.remove(0);
        assert_eq!(quiz.hardware.tv_format, Some(TvStandard::Ntsc));
        assert_eq!(quiz.languages, vec![Language::English, Language::French]);
        assert!(missingno_gamedb::validate(root.path()).unwrap().is_empty());

        let written =
            std::fs::read_to_string(root.path().join("colecovision/zenji/manifest.ron")).unwrap();
        let stats = run(root.path(), &dat, &mut Report::default()).unwrap();
        assert_eq!(stats.new_games, 0);
        assert_eq!(stats.releases_added, 0);
        assert_eq!(
            std::fs::read_to_string(root.path().join("colecovision/zenji/manifest.ron")).unwrap(),
            written
        );
    }
}
