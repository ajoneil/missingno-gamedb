use std::{
    fs, io,
    path::{Path, PathBuf},
};

use crate::{
    game::Game,
    ids::Slug,
    platform::{GameBoy, GameBoyColor, Platform, Vcs},
};

/// A loaded game with the slug it is filed under.
#[derive(Clone, Debug)]
pub struct Entry<P: Platform> {
    pub slug: Slug,
    pub game: Game<P>,
}

/// One platform's tree of games.
#[derive(Clone, Debug)]
pub struct Tree<P: Platform> {
    pub games: Vec<Entry<P>>,
}

/// A file that could not be loaded; loading continues past it.
#[derive(Clone, Debug)]
pub struct LoadIssue {
    pub path: PathBuf,
    pub message: String,
}

/// `(directory name, manifest path)` for every game in a platform tree,
/// sorted by directory name. A missing tree directory is an empty list.
pub(crate) fn manifest_paths(db_root: &Path, dir: &str) -> io::Result<Vec<(String, PathBuf)>> {
    let tree_dir = db_root.join(dir);
    if !tree_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(&tree_dir)? {
        let entry = entry?;
        let manifest = entry.path().join("manifest.ron");
        if manifest.is_file() {
            paths.push((entry.file_name().to_string_lossy().into_owned(), manifest));
        }
    }
    paths.sort();
    Ok(paths)
}

impl<P: Platform> Tree<P> {
    pub fn load(db_root: &Path) -> io::Result<(Self, Vec<LoadIssue>)> {
        let mut games = Vec::new();
        let mut issues = Vec::new();
        for (name, path) in manifest_paths(db_root, P::DIR)? {
            let slug = match name.parse::<Slug>() {
                Ok(slug) => slug,
                Err(message) => {
                    issues.push(LoadIssue { path, message });
                    continue;
                }
            };
            let text = match fs::read_to_string(&path) {
                Ok(text) => text,
                Err(e) => {
                    issues.push(LoadIssue {
                        path,
                        message: e.to_string(),
                    });
                    continue;
                }
            };
            match Game::<P>::from_ron(&text) {
                Ok(game) => games.push(Entry { slug, game }),
                Err(e) => issues.push(LoadIssue {
                    path,
                    message: e.to_string(),
                }),
            }
        }
        Ok((Self { games }, issues))
    }
}

/// All three platform trees.
#[derive(Clone, Debug)]
pub struct Database {
    pub gb: Tree<GameBoy>,
    pub gbc: Tree<GameBoyColor>,
    pub vcs: Tree<Vcs>,
}

impl Database {
    pub fn load(db_root: &Path) -> io::Result<(Self, Vec<LoadIssue>)> {
        let (gb, mut issues) = Tree::load(db_root)?;
        let (gbc, gbc_issues) = Tree::load(db_root)?;
        let (vcs, vcs_issues) = Tree::load(db_root)?;
        issues.extend(gbc_issues);
        issues.extend(vcs_issues);
        Ok((Self { gb, gbc, vcs }, issues))
    }
}
