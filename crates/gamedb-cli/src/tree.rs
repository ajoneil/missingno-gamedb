use std::{fs, io, path::Path};

use missingno_gamedb::{Game, Platform};

/// Remove a game directory wholesale (contents are recoverable from git —
/// callers run behind the clean-tree guard).
pub fn remove_game_dir(db_root: &Path, tree: &str, slug: &str) -> io::Result<()> {
    let dir = db_root.join(tree).join(slug);
    if dir.is_dir() {
        fs::remove_dir_all(dir)?;
    }
    Ok(())
}

pub fn write_game<P: Platform>(db_root: &Path, slug: &str, game: &Game<P>) -> io::Result<()> {
    let dir = db_root.join(P::DIR).join(slug);
    fs::create_dir_all(&dir)?;
    let text = game
        .to_ron_string()
        .map_err(|e| io::Error::other(format!("{slug}: {e}")))?;
    fs::write(dir.join("manifest.ron"), text)
}

/// Sorted sha1 multiset of a set of games, for the preservation invariant.
pub fn sha1_multiset<P: Platform>(games: &[(String, Game<P>)]) -> Vec<String> {
    let mut sha1s: Vec<String> = games
        .iter()
        .flat_map(|(_, game)| &game.releases)
        .flat_map(|release| &release.artifacts)
        .map(|artifact| artifact.sha1.as_str().to_owned())
        .collect();
    sha1s.sort();
    sha1s
}
