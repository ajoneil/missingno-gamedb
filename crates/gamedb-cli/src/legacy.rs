use std::{fs, io, path::Path};

use missingno_gamedb::{Link, TvStandard};
use serde::Deserialize;

/// The pre-migration manifest shape; parse-only.
#[derive(Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields)]
pub struct LegacyManifest {
    pub title: String,
    #[serde(default)]
    pub date: Option<String>,
    #[serde(default)]
    pub region: Option<String>,
    #[serde(default)]
    pub developer: Option<String>,
    #[serde(default)]
    pub publisher: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub tv_format: Option<TvStandard>,
    #[serde(default)]
    pub cart_type: Option<String>,
    #[serde(default)]
    pub hashes: Vec<String>,
    #[serde(default)]
    pub source: Option<LegacySource>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub screenshots: Vec<String>,
    #[serde(default)]
    pub links: Vec<Link>,
}

#[derive(Deserialize, Clone, Debug)]
pub enum LegacySource {
    HomebrewHub { slug: String, filename: String },
    Url(String),
}

pub type LoadResult = Result<Vec<(String, LegacyManifest)>, Vec<(String, String)>>;

/// Every legacy manifest in a tree, or every parse failure — never a mixture
/// silently dropped. `skip_new_format` ignores files that already parse as the
/// new schema (post-homebrew-migration trees hold both).
pub fn load_tree<P: missingno_gamedb::Platform>(
    db_root: &Path,
    skip_new_format: bool,
) -> io::Result<LoadResult> {
    let tree_dir = db_root.join(P::DIR);
    let mut entries = Vec::new();
    let mut failures = Vec::new();
    if !tree_dir.is_dir() {
        return Ok(Ok(entries));
    }
    let mut dirs: Vec<_> = fs::read_dir(&tree_dir)?.collect::<Result<_, _>>()?;
    dirs.sort_by_key(|d| d.file_name());
    for dir in dirs {
        let manifest = dir.path().join("manifest.ron");
        if !manifest.is_file() {
            continue;
        }
        let text = fs::read_to_string(&manifest)?;
        if skip_new_format && missingno_gamedb::Game::<P>::from_ron(&text).is_ok() {
            continue;
        }
        match ron::from_str::<LegacyManifest>(&text) {
            Ok(parsed) => entries.push((dir.file_name().to_string_lossy().into_owned(), parsed)),
            Err(e) => failures.push((manifest.display().to_string(), e.to_string())),
        }
    }
    Ok(if failures.is_empty() {
        Ok(entries)
    } else {
        Err(failures)
    })
}
