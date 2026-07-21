use serde::{Deserialize, Serialize};

use crate::{
    ids::{Date, ReleaseDate, Sha1},
    platform::Platform,
    region::Region,
    source::Source,
};

pub(crate) fn is_default<T: Default + PartialEq>(value: &T) -> bool {
    *value == T::default()
}

/// One game: the work itself, plus its releases.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(deny_unknown_fields, bound = "")]
pub struct Game<P: Platform> {
    pub title: String,
    #[serde(default, skip_serializing_if = "is_default")]
    pub kind: GameKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub developer: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
    /// Remote cover image URLs, preference-ordered.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub covers: Vec<String>,
    /// Remote screenshot URLs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub screenshots: Vec<String>,
    /// Present when this game is a derived work patched onto another game's ROM.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mod_of: Option<ModOf>,
    /// Date a human last confirmed this entry; automation clears it on change.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub curated: Option<Date>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub releases: Vec<Release<P>>,
}

impl<P: Platform> Game<P> {
    pub fn from_ron(text: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(text)
    }

    /// Canonical manifest text: one fixed formatting for every writer, so git
    /// diffs stay minimal.
    pub fn to_ron_string(&self) -> Result<String, ron::Error> {
        let mut text = ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::new())?;
        text.push('\n');
        Ok(text)
    }
}

/// What kind of work this entry is. `Demo` is a playable preview of a game
/// (kiosk/sample carts); `Demoscene` is a scene production in its own right.
#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameKind {
    #[default]
    Game,
    Demo,
    Demoscene,
}

/// A concrete published form of a game: region, revision, hardware variant.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(deny_unknown_fields, bound = "")]
pub struct Release<P: Platform> {
    /// The title this release was published under, when it differs from the
    /// game's canonical title (localized names: "Pokemon - Blaue Edition").
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    /// Distinguishing name where regions/hardware aren't enough: "Rev A", "Player's Choice".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub regions: Vec<Region>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<ReleaseDate>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub status: ReleaseStatus,
    #[serde(default, skip_serializing_if = "is_default")]
    pub hardware: P::ReleaseHardware,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub sources: Vec<Source>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<Artifact>,
}

/// How finished a release is. `WorkInProgress` is an unfinished version the
/// author published; `Beta`/`Prototype` are pre-release builds, usually dumps
/// that were never meant to circulate.
#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReleaseStatus {
    #[default]
    Released,
    WorkInProgress,
    Beta,
    Prototype,
}

/// A known dump of a release.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub sha1: Sha1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub name: String,
    pub url: String,
    pub link_type: LinkType,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum LinkType {
    Wiki,
    Manual,
    Source,
    Speedrun,
    UnusedContent,
    TechnicalReference,
    Guide,
    Community,
}

/// The derivation block of a mod/romhack: which artifact it patches and how.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct ModOf {
    /// The base ROM the patch applies to.
    pub base_sha1: Sha1,
    pub category: ModCategory,
    pub patch: Patch,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModCategory {
    Translation,
    QualityOfLife,
    ContentChange,
    TotalConversion,
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct Patch {
    pub format: PatchFormat,
    pub sha1: Sha1,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum PatchFormat {
    Ips,
    Bps,
}
