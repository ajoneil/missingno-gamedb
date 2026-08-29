//! One-shot: state a release's ROM on the board that holds it, and drop the
//! dump lengths the catalogue stops carrying.
//!
//! Sizes were two flat fields. The cartridge's ROM was the release's
//! `rom_size`, which only ever made sense beside the board — so it becomes that
//! board's `rom`, measured silicon on the silicon it measures. The dump's own
//! length was the artifact's `size`, which is a property of a file rather than
//! a fact about a product, and goes.

use std::{fs, path::Path};

use missingno_gamedb::{
    AttributeKind, AttributeSpec, AttributeValue, BoardSpec, BoardValue, FactKind, FactValue, Game,
    GameBoy, GameBoyColor, HardwareFacts, Platform, Sg1000, Tree, Vcs, with_platforms,
};

use crate::{report::Report, tree};

#[derive(Default)]
pub struct Stats {
    pub trees: Vec<TreeStats>,
}

#[derive(Default)]
pub struct TreeStats {
    pub tree: &'static str,
    /// Manifests whose bare board name was restated in the attributed form.
    pub restated: usize,
    pub folded: usize,
    /// Folds that named the plain-ROM board, the release having stated none.
    pub boards_stated: usize,
    pub refused: usize,
    pub dropped: usize,
    pub rewritten: usize,
}

impl TreeStats {
    pub fn line(&self) -> String {
        format!(
            "{}: {} ROM sizes folded onto boards ({} boards newly stated, {} refused), \
             {} dump lengths dropped, {} bare boards restated, {} manifests rewritten",
            self.tree,
            self.folded,
            self.boards_stated,
            self.refused,
            self.dropped,
            self.restated,
            self.rewritten
        )
    }
}

pub fn run(data_root: &Path, report: &mut Report) -> Result<Stats, String> {
    let mut stats = Stats::default();
    macro_rules! sweep_each {
        ($($P:ident),* $(,)?) => {$( stats.trees.push(sweep::<$P>(data_root, report)?); )*};
    }
    with_platforms!(sweep_each);
    Ok(stats)
}

fn sweep<P: Platform>(data_root: &Path, report: &mut Report) -> Result<TreeStats, String> {
    let mut stats = TreeStats {
        tree: P::DIR,
        restated: restate_bare_boards::<P>(data_root)?,
        ..TreeStats::default()
    };

    let (tree, issues) = Tree::<P>::load(data_root).map_err(|e| e.to_string())?;
    if let Some(first) = issues.first() {
        return Err(format!("{}: {}", first.path.display(), first.message));
    }

    for entry in tree.games {
        let slug = entry.slug.as_str().to_owned();
        let mut game = entry.game;
        let mut changed = false;

        for (index, release) in game.releases.iter_mut().enumerate() {
            if let Some(bytes) = release.rom_size {
                match fold_rom(&mut release.hardware, P::DIR, bytes) {
                    Ok(newly_stated) => {
                        release.rom_size = None;
                        stats.folded += 1;
                        stats.boards_stated += usize::from(newly_stated);
                        changed = true;
                    }
                    Err(e) => {
                        report.add(
                            "ROM size no board could carry",
                            format!("{}/{slug} release {index}: {e}", P::DIR),
                        );
                        stats.refused += 1;
                    }
                }
            }
            for artifact in &mut release.artifacts {
                if artifact.size.take().is_some() {
                    stats.dropped += 1;
                    changed = true;
                }
            }
        }
        for game_mod in &mut game.mods {
            for release in &mut game_mod.releases {
                for artifact in &mut release.artifacts {
                    if artifact.size.take().is_some() {
                        stats.dropped += 1;
                        changed = true;
                    }
                }
            }
        }

        if changed {
            tree::write_game(data_root, &slug, &game).map_err(|e| e.to_string())?;
            stats.rewritten += 1;
        }
    }
    Ok(stats)
}

/// State the ROM on the board that holds it. A release that named no board
/// still has one — the plain ROM the tree reads an absent board as.
fn fold_rom<H: HardwareFacts>(hardware: &mut H, tree: &str, bytes: u32) -> Result<bool, String> {
    let key = board_key::<H>().ok_or_else(|| format!("the {tree} tree states no board"))?;
    let stated = match hardware.get(key) {
        Some(FactValue::Board(board)) => board,
        _ => None,
    };
    let newly_stated = stated.is_none();
    let board = match stated {
        Some(board) => board,
        None => BoardValue::new(plain_board(tree).ok_or_else(|| {
            format!("no board is stated and the {tree} tree names no plain-ROM board")
        })?),
    };
    hardware.set(
        key,
        FactValue::Board(Some(board.with("rom", AttributeValue::Bytes(bytes)))),
    )?;
    Ok(newly_stated)
}

/// What an unstated board means, where the tree reads one: an SG-1000 cart with
/// no board named is a ROM and nothing else, and its size belongs on that board.
fn plain_board(tree: &str) -> Option<&'static str> {
    (tree == Sg1000::DIR).then_some("Flat")
}

fn board_key<H: HardwareFacts>() -> Option<&'static str> {
    H::descriptors()
        .iter()
        .find(|fact| matches!(fact.kind, FactKind::Board { .. }))
        .map(|fact| fact.key)
}

fn board_catalogue<H: HardwareFacts>() -> &'static [BoardSpec] {
    H::descriptors()
        .iter()
        .find_map(|fact| match fact.kind {
            FactKind::Board { catalogue } => Some(catalogue()),
            _ => None,
        })
        .unwrap_or(&[])
}

/// A manifest written before boards carried parts spells a board as a bare
/// name, which the attributed vocabulary cannot read. What that name meant is
/// the board with nothing measured on it.
fn restate_bare_boards<P: Platform>(data_root: &Path) -> Result<usize, String> {
    let mut restated = 0;
    for path in manifests(&data_root.join(P::DIR))? {
        let text = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        if Game::<P>::from_ron(&text).is_ok() {
            continue;
        }
        let mut repaired = text;
        for spec in board_catalogue::<P::ReleaseHardware>() {
            if let Some(unmeasured) = unmeasured_form(spec) {
                repaired = repaired.replace(&format!("Some({})", spec.name), &unmeasured);
            }
        }
        let game =
            Game::<P>::from_ron(&repaired).map_err(|e| format!("{}: {e}", path.display()))?;
        let canonical = game
            .to_ron_string()
            .map_err(|e| format!("{}: {e}", path.display()))?;
        fs::write(&path, canonical).map_err(|e| format!("{}: {e}", path.display()))?;
        restated += 1;
    }
    Ok(restated)
}

/// The unmeasured form of a board whose ROM is the one part it carries — the
/// only shape a bare name can be restated as, since every other part would be
/// inventing silicon the manifest never claimed.
fn unmeasured_form(spec: &BoardSpec) -> Option<String> {
    match spec.attributes {
        [
            AttributeSpec {
                key: "rom",
                kind: AttributeKind::Bytes,
                optional: true,
                ..
            },
        ] => Some(format!("Some({}(rom: None))", spec.name)),
        _ => None,
    }
}

fn manifests(tree_dir: &Path) -> Result<Vec<std::path::PathBuf>, String> {
    if !tree_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(tree_dir).map_err(|e| format!("{}: {e}", tree_dir.display()))? {
        let entry = entry.map_err(|e| format!("{}: {e}", tree_dir.display()))?;
        let manifest = entry.path().join("manifest.ron");
        if manifest.is_file() {
            paths.push(manifest);
        }
    }
    paths.sort();
    Ok(paths)
}
