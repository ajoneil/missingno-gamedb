use std::{fs, io, path::Path};

use serde::{Deserialize, Serialize};

use crate::ids::Date;

/// An importer's unresolved question about the data, tracked to a decision.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct Flag {
    pub id: u32,
    pub kind: FlagKind,
    /// The entries this concerns, as "tree/slug" refs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub subject: Vec<String>,
    pub note: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolved: Option<Date>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum FlagKind {
    NearMissTitles,
    ReviewCandidateFamilies,
    Leftover,
    UnknownQualifier,
    ConflictingField,
    RetiredHash,
    Custom,
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct FlagFile {
    pub flags: Vec<Flag>,
}

impl FlagFile {
    pub fn path(repo_root: &Path) -> std::path::PathBuf {
        repo_root.join("curation").join("flags.ron")
    }

    pub fn load(repo_root: &Path) -> io::Result<Self> {
        let path = Self::path(repo_root);
        if !path.is_file() {
            return Ok(Self::default());
        }
        let text = fs::read_to_string(&path)?;
        ron::from_str(&text).map_err(|e| io::Error::other(format!("{}: {e}", path.display())))
    }

    pub fn save(&self, repo_root: &Path) -> io::Result<()> {
        let dir = repo_root.join("curation");
        fs::create_dir_all(&dir)?;
        let mut text = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::new())
            .map_err(io::Error::other)?;
        text.push('\n');
        fs::write(Self::path(repo_root), text)
    }

    pub fn next_id(&self) -> u32 {
        self.flags.iter().map(|f| f.id + 1).max().unwrap_or(1)
    }

    pub fn open(&self) -> impl Iterator<Item = &Flag> {
        self.flags.iter().filter(|f| f.resolved.is_none())
    }
}
