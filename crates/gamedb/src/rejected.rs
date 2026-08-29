use std::{fs, io, path::Path};

use serde::{Deserialize, Serialize};

use crate::Sha1;

/// A dump the database deliberately does not catalogue. Where a flag is work
/// that ends when someone does it, this is a standing decision, so the scan
/// consults it and never offers the dump as a new record again.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct Rejection {
    /// What the entry was called when it was turned away.
    pub title: String,
    pub reason: String,
    pub dumps: Vec<Sha1>,
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct RejectedFile {
    pub rejected: Vec<Rejection>,
}

impl RejectedFile {
    pub fn path(repo_root: &Path) -> std::path::PathBuf {
        repo_root.join("curation").join("rejected.ron")
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

    pub fn holds(&self, sha1: &str) -> bool {
        self.rejected
            .iter()
            .any(|r| r.dumps.iter().any(|d| d.as_str() == sha1))
    }

    /// The rejection a dump belongs to, for saying why a scan passed it over.
    pub fn reason_for(&self, sha1: &str) -> Option<&Rejection> {
        self.rejected
            .iter()
            .find(|r| r.dumps.iter().any(|d| d.as_str() == sha1))
    }
}
