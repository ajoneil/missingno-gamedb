use std::fmt::Debug;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// The platform axis of the database: one tree per platform, and a
/// platform-specific block of per-release hardware facts.
pub trait Platform {
    /// Per-release hardware facts for this platform.
    type ReleaseHardware: Serialize + DeserializeOwned + Default + Clone + PartialEq + Debug;
    /// Tree directory name at the database root.
    const DIR: &'static str;
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct GameBoy;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct GameBoyColor;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Vcs;

impl Platform for GameBoy {
    type ReleaseHardware = GbHardware;
    const DIR: &'static str = "gb";
}

impl Platform for GameBoyColor {
    type ReleaseHardware = GbcHardware;
    const DIR: &'static str = "gbc";
}

impl Platform for Vcs {
    type ReleaseHardware = VcsHardware;
    const DIR: &'static str = "vcs";
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct GbHardware {
    #[serde(default, skip_serializing_if = "Enhancement::is_unknown")]
    pub sgb: Enhancement,
    #[serde(default, skip_serializing_if = "Enhancement::is_unknown")]
    pub cgb: Enhancement,
    /// Cartridge mapper, e.g. "MBC1", "MBC3+TIMER+BATTERY"; `None` = as the header says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapper: Option<String>,
}

/// Whether a Game Boy release detects and uses an enhancing console.
/// `Unknown` is honest absence of data: flags backfill from external sources,
/// never by assumption.
#[derive(Serialize, Deserialize, Default, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Enhancement {
    #[default]
    Unknown,
    NotEnhanced,
    Enhanced,
}

impl Enhancement {
    pub fn is_unknown(&self) -> bool {
        *self == Self::Unknown
    }
}

/// CGB games are CGB-required by definition.
#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct GbcHardware {
    /// Cartridge mapper, e.g. "MBC5+RUMBLE+RAM+BATTERY"; `None` = as the header says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapper: Option<String>,
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct VcsHardware {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tv_format: Option<TvFormat>,
    /// Cartridge board code, e.g. "F8", "F6SC", "4K".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cart_type: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TvFormat {
    Ntsc,
    Pal,
    Secam,
}
