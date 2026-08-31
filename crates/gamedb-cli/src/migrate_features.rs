//! One-shot: split the Game Boy `features` list into `enhancements` (console
//! variants the software exploits) and `peripherals` (devices it is played
//! with), and rename the VCS `controllers` key to `peripherals`.

use std::{
    fs,
    path::{Path, PathBuf},
};

use missingno_gamedb::{Game, GameBoy, GameBoyColor, Platform, Vcs};

fn manifest_paths(db_root: &Path, dir: &str) -> Result<Vec<PathBuf>, String> {
    let tree_dir = db_root.join(dir);
    let mut paths = Vec::new();
    for entry in fs::read_dir(&tree_dir).map_err(|e| format!("{}: {e}", tree_dir.display()))? {
        let manifest = entry
            .map_err(|e| e.to_string())?
            .path()
            .join("manifest.ron");
        if manifest.is_file() {
            paths.push(manifest);
        }
    }
    paths.sort();
    Ok(paths)
}

#[derive(Default)]
pub struct Stats {
    pub gb_files: usize,
    pub gbc_files: usize,
    pub vcs_files: usize,
    pub super_game_boy: usize,
    pub game_boy_color: usize,
    pub link_cable: usize,
    pub printer: usize,
    pub cleared_lists: usize,
    pub controller_lists: usize,
}

/// Where a `features` entry lands under the split.
enum Split {
    Enhancement(&'static str),
    Peripheral(&'static str),
}

fn split_term(term: &str) -> Option<Split> {
    Some(match term {
        "SuperGameBoyEnhanced" => Split::Enhancement("SuperGameBoy"),
        "GameBoyColorEnhanced" => Split::Enhancement("GameBoyColor"),
        "GameLink" => Split::Peripheral("LinkCable"),
        "Printer" => Split::Peripheral("Printer"),
        _ => return None,
    })
}

/// A term this does not recognise stops the migration, so there is nothing to
/// report but the counts.
pub fn run(db_root: &Path) -> Result<Stats, String> {
    let mut stats = Stats::default();
    split_tree::<GameBoy>(db_root, &mut stats)?;
    split_tree::<GameBoyColor>(db_root, &mut stats)?;
    rename_tree::<Vcs>(db_root, &mut stats)?;
    Ok(stats)
}

fn split_tree<P: Platform>(db_root: &Path, stats: &mut Stats) -> Result<(), String> {
    for path in manifest_paths(db_root, P::DIR)? {
        let display = path.display().to_string();
        let text = fs::read_to_string(&path).map_err(|e| format!("{display}: {e}"))?;
        let Some(rewritten) =
            split_features(&text, stats).map_err(|e| format!("{display}: {e}"))?
        else {
            continue;
        };
        Game::<P>::from_ron(&rewritten)
            .map_err(|e| format!("{display}: rewritten manifest does not parse: {e}"))?;
        fs::write(&path, rewritten).map_err(|e| format!("{display}: {e}"))?;
        match P::DIR {
            "gbc" => stats.gbc_files += 1,
            _ => stats.gb_files += 1,
        }
    }
    Ok(())
}

/// `None` where the manifest states no `features` list, so untouched files stay
/// untouched.
fn split_features(text: &str, stats: &mut Stats) -> Result<Option<String>, String> {
    let mut out = String::with_capacity(text.len());
    let mut lines = text.lines().peekable();
    let mut split_any = false;

    while let Some(line) = lines.next() {
        let indent = &line[..line.len() - line.trim_start().len()];
        let trimmed = line.trim_start();
        if trimmed != "features: []," && trimmed != "features: [" {
            out.push_str(line);
            out.push('\n');
            continue;
        }
        split_any = true;

        let mut terms = Vec::new();
        if trimmed == "features: [" {
            loop {
                let entry = lines.next().ok_or("unterminated features list")?;
                let entry = entry.trim();
                if entry == "]," {
                    break;
                }
                terms.push(entry.trim_end_matches(',').to_owned());
            }
        } else {
            stats.cleared_lists += 1;
        }

        let mut enhancements = Vec::new();
        let mut peripherals = Vec::new();
        for term in &terms {
            match split_term(term).ok_or(format!("unknown feature {term:?}"))? {
                Split::Enhancement(name) => {
                    match name {
                        "SuperGameBoy" => stats.super_game_boy += 1,
                        _ => stats.game_boy_color += 1,
                    }
                    enhancements.push(name);
                }
                Split::Peripheral(name) => {
                    match name {
                        "LinkCable" => stats.link_cable += 1,
                        _ => stats.printer += 1,
                    }
                    peripherals.push(name);
                }
            }
        }

        write_list(&mut out, indent, "enhancements", &enhancements);
        // A release that drove no peripheral never stated one, so the split
        // leaves the key unstated rather than claiming it drives none.
        if !peripherals.is_empty() {
            write_list(&mut out, indent, "peripherals", &peripherals);
        }
    }

    Ok(split_any.then_some(out))
}

fn write_list(out: &mut String, indent: &str, key: &str, terms: &[&str]) {
    if terms.is_empty() {
        out.push_str(&format!("{indent}{key}: [],\n"));
        return;
    }
    out.push_str(&format!("{indent}{key}: [\n"));
    for term in terms {
        out.push_str(&format!("{indent}    {term},\n"));
    }
    out.push_str(&format!("{indent}],\n"));
}

fn rename_tree<P: Platform>(db_root: &Path, stats: &mut Stats) -> Result<(), String> {
    for path in manifest_paths(db_root, P::DIR)? {
        let display = path.display().to_string();
        let text = fs::read_to_string(&path).map_err(|e| format!("{display}: {e}"))?;
        let mut renamed = 0;
        let mut out = String::with_capacity(text.len());
        for line in text.lines() {
            let indent = &line[..line.len() - line.trim_start().len()];
            match line.trim_start() {
                rest @ ("controllers: [" | "controllers: [],") => {
                    renamed += 1;
                    out.push_str(indent);
                    out.push_str(&rest.replace("controllers:", "peripherals:"));
                }
                _ => out.push_str(line),
            }
            out.push('\n');
        }
        if renamed == 0 {
            continue;
        }
        Game::<P>::from_ron(&out)
            .map_err(|e| format!("{display}: rewritten manifest does not parse: {e}"))?;
        fs::write(&path, out).map_err(|e| format!("{display}: {e}"))?;
        stats.vcs_files += 1;
        stats.controller_lists += renamed;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use missingno_gamedb::{Enhancement, Peripheral};

    fn write_manifest(root: &Path, tree: &str, slug: &str, text: &str) {
        let dir = root.join(tree).join(slug);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("manifest.ron"), text).unwrap();
    }

    fn manifest(root: &Path, tree: &str, slug: &str) -> String {
        fs::read_to_string(root.join(tree).join(slug).join("manifest.ron")).unwrap()
    }

    const SPLIT: &str = r#"(
    title: "Split",
    releases: [
        (
            hardware: (
                features: [
                    SuperGameBoyEnhanced,
                    GameLink,
                ],
                cart_type: Some(Mbc1(
                    rom: Kb512,
                    ram: None,
                    battery: false,
                )),
            ),
            artifacts: [
                (
                    sha1: "0123456789abcdef0123456789abcdef01234567",
                ),
            ],
        ),
        (
            hardware: (
                features: [],
            ),
            artifacts: [
                (
                    sha1: "89abcdef0123456789abcdef0123456789abcdef",
                ),
            ],
        ),
    ],
)
"#;

    const CONTROLLED: &str = r#"(
    title: "Controlled",
    releases: [
        (
            hardware: (
                tv_format: Some(Ntsc),
                controllers: [
                    Paddle,
                ],
            ),
            artifacts: [
                (
                    sha1: "920cfbd517764ad3fa6a7425c031bd72dc7d927c",
                ),
            ],
        ),
    ],
)
"#;

    #[test]
    fn features_split_by_kind_and_controllers_are_renamed() {
        let root = tempfile::tempdir().unwrap();
        write_manifest(root.path(), "gb", "split", SPLIT);
        write_manifest(root.path(), "vcs", "controlled", CONTROLLED);
        for tree in ["gbc", "sg1000"] {
            fs::create_dir_all(root.path().join(tree)).unwrap();
        }

        let stats = run(root.path()).unwrap();
        assert_eq!(
            (stats.gb_files, stats.gbc_files, stats.vcs_files),
            (1, 0, 1)
        );
        assert_eq!((stats.super_game_boy, stats.link_cable), (1, 1));
        assert_eq!((stats.cleared_lists, stats.controller_lists), (1, 1));

        let split = manifest(root.path(), "gb", "split");
        let game = Game::<GameBoy>::from_ron(&split).unwrap();
        assert_eq!(
            game.releases[0].hardware.enhancements,
            Some(vec![Enhancement::SuperGameBoy])
        );
        assert_eq!(
            game.releases[0].hardware.peripherals,
            Some(vec![Peripheral::LinkCable])
        );
        assert_eq!(game.releases[1].hardware.enhancements, Some(Vec::new()));
        assert_eq!(
            game.releases[1].hardware.peripherals, None,
            "a stated empty list is not a claim that it drives no peripheral"
        );
        assert_eq!(
            game.to_ron_string().unwrap(),
            split,
            "canonically formatted"
        );

        let controlled = manifest(root.path(), "vcs", "controlled");
        let game = Game::<Vcs>::from_ron(&controlled).unwrap();
        assert_eq!(
            game.releases[0].hardware.peripherals,
            Some(vec![Peripheral::Paddle])
        );
        assert_eq!(
            game.to_ron_string().unwrap(),
            controlled,
            "canonically formatted"
        );

        assert!(missingno_gamedb::validate(root.path()).unwrap().is_empty());
        let repeat = run(root.path()).unwrap();
        assert_eq!((repeat.gb_files, repeat.vcs_files), (0, 0));
    }
}
