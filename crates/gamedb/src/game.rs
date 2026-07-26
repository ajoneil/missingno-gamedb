use serde::{Deserialize, Serialize};

use crate::{
    ids::{ReleaseDate, Sha1},
    platform::Platform,
    region::Region,
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
    /// Present when this game is a derived work patched onto another game's ROM
    /// (total conversions — works with their own identity).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mod_of: Option<ModOf>,
    /// Fan modifications of this game — QoL fixes, additions, translations
    /// (a translated game is the same game, as official localizations are).
    /// Each is its own thing with its own versions and curation.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub mods: Vec<Mod<P>>,
    /// Whether a human has reviewed this entry — the draft/non-draft line.
    /// Edits happen at a curator's request, so they never clear it.
    #[serde(default, skip_serializing_if = "is_default")]
    pub curated: bool,
    /// Sexually explicit content — lets a frontend gate or badge the entry.
    #[serde(default, skip_serializing_if = "is_default")]
    pub adult: bool,
    /// Editor's-choice highlights, by curator identifier ("andy"), not
    /// display name; presentation is the frontend's decision.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recommended_by: Vec<String>,
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
/// (kiosk/sample carts); `Demoscene` is a scene production in its own right;
/// `Test` is a diagnostic or calibration utility (pattern generators, test carts).
#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum GameKind {
    #[default]
    Game,
    Demo,
    Demoscene,
    Test,
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
    pub artifacts: Vec<Artifact>,
}

/// How finished a release is. `WorkInProgress` is an unfinished version the
/// author published; `Beta`/`Prototype` are pre-release builds, usually dumps
/// that were never meant to circulate. `Demo` is a limited promotional build
/// that shipped as its own product (a store-demo cartridge), not the full game.
#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReleaseStatus {
    #[default]
    Released,
    Demo,
    WorkInProgress,
    Beta,
    Prototype,
}

/// A known dump of a release.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct Artifact {
    pub sha1: Sha1,
    /// What distinguishes this dump when a release has several: "alt", "[a1]" —
    /// benign dump-level variance, not release facts. A quality *problem* goes
    /// in `defect`, not here.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    /// A quality problem with this dump, if any. Separate from `label` so the
    /// two severities are queryable: an overdump still plays, a bad dump does not.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defect: Option<Defect>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size: Option<u64>,
}

/// A quality problem with a specific dump — distinct from the benign `label`
/// that merely tells sibling dumps apart.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Defect {
    /// A padded dump: larger than the ROM, so it fingerprints as a bigger
    /// board. The emulator loads it on the release's stated board and ignores
    /// the padding; recorded so it is not mistaken for a distinct release
    /// (TOSEC `[o]`).
    Overdump,
    /// A corrupt or truncated dump that does not play correctly (TOSEC `[b]`).
    BadDump,
}

impl Defect {
    /// Short human-readable name for display.
    pub fn label(self) -> &'static str {
        match self {
            Defect::Overdump => "overdump",
            Defect::BadDump => "bad dump",
        }
    }
}

#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub name: String,
    pub url: String,
    pub link_type: LinkType,
    /// The languages this link's text is in. Empty = English (the database is
    /// English-first, so most links carry no tag). A populated list names every
    /// language present, so a trilingual manual reads `[English, German, French]`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub languages: Vec<Language>,
}

/// A human language, for tagging non-English (or multilingual) links and — in
/// future — releases whose text differs by language.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Language {
    English,
    French,
    German,
    Spanish,
    Italian,
    Portuguese,
    Dutch,
    Japanese,
    Swedish,
}

impl Language {
    /// Short human-readable name for display.
    pub fn label(self) -> &'static str {
        match self {
            Language::English => "English",
            Language::French => "French",
            Language::German => "German",
            Language::Spanish => "Spanish",
            Language::Italian => "Italian",
            Language::Portuguese => "Portuguese",
            Language::Dutch => "Dutch",
            Language::Japanese => "Japanese",
            Language::Swedish => "Swedish",
        }
    }
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
    /// A page to buy the game — a store product page for a paid aftermarket or
    /// homebrew cart, as opposed to a free `DownloadPage`.
    Store,
    /// Where to get the ROM: a page to obtain it from (a forum thread or
    /// "download here" page, followed by a human), and a direct, fetchable ROM
    /// file URL. Split so a freeware game can carry whichever it has — some
    /// hosts (AtariAge) offer only a page, others also a direct file.
    DownloadPage,
    Download,
}

/// The derivation block of a mod/romhack: which artifact it patches and how.
#[derive(Serialize, Deserialize, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct ModOf {
    /// The base ROM the derived work modifies.
    pub base_sha1: Sha1,
    pub category: ModCategory,
    /// Absent when the work circulates only as a pre-patched ROM.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch: Option<Patch>,
}

/// A fan modification attached to the game it modifies.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(deny_unknown_fields, bound = "")]
pub struct Mod<P: Platform> {
    pub name: String,
    pub category: ModCategory,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<Link>,
    /// Independently of the game: curating a game does not vouch for its
    /// mods, and a mod can earn its own recommendation.
    #[serde(default, skip_serializing_if = "is_default")]
    pub curated: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recommended_by: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub releases: Vec<ModRelease<P>>,
}

/// One version of a mod.
#[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
#[serde(deny_unknown_fields, bound = "")]
pub struct ModRelease<P: Platform> {
    /// Version or variant name: "v1.2", "easy mode".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub date: Option<ReleaseDate>,
    /// The base artifact this version applies to, when known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_sha1: Option<Sha1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub patch: Option<Patch>,
    /// Where the build differs from the game's own hardware — the controller a
    /// conversion swaps to, the TV standard a regional build targets.
    #[serde(default, skip_serializing_if = "is_default")]
    pub hardware: P::ReleaseHardware,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub artifacts: Vec<Artifact>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum ModCategory {
    Translation,
    QualityOfLife,
    ContentChange,
    /// Makes the same game run on hardware or a region it otherwise wouldn't
    /// (NTSC/PAL conversion, a bankswitch re-encoding) — no gameplay change.
    Compatibility,
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
