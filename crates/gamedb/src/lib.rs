//! Schema, loader, and validator for the missingno game database.
//!
//! The database is a tree of RON manifests, one per game, grouped per
//! platform: `{gb,gbc,vcs}/{slug}/manifest.ron`. A game holds one or more
//! releases (region / revision / hardware variants), each carrying the
//! artifacts (ROM dumps) and sources (where to obtain it) for that variant.

pub mod flags;
pub mod game;
pub mod ids;
pub mod load;
pub mod platform;
pub mod region;
pub mod source;
pub mod text;
pub mod validate;

pub use flags::{Flag, FlagFile, FlagKind};
pub use game::{
    Artifact, Curation, Game, GameKind, Link, LinkType, Mod, ModCategory, ModOf, ModRelease, Patch,
    PatchFormat, Release, ReleaseStatus, Verification, VerificationMethod,
};
pub use ids::{Date, ReleaseDate, Sha1, Slug};
pub use load::{Database, Entry, LoadIssue, Tree};
pub use platform::{
    Controller, Enhancement, GameBoy, GameBoyColor, GbHardware, GbcHardware, Platform, TvFormat,
    Vcs, VcsHardware,
};
pub use region::Region;
pub use source::Source;
pub use text::normalized_title;
pub use validate::{Finding, FormatReport, Severity, format_all, validate};
