use std::fmt::Debug;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::facts::HardwareFacts;

/// The board vocabularies are the emulator cores' own, so a hardware payload
/// names a board some silicon model builds and an unlisted code fails to parse.
pub use missingno_gb::cartridge::GbCartType;
pub use missingno_sg1000::cartridge::CartType as Sg1000CartType;
pub use missingno_vcs::{CartType as VcsCartType, TvStandard};

/// The platform axis of the database: one tree per platform, and a
/// platform-specific block of per-release hardware facts.
pub trait Platform {
    /// Per-release hardware facts for this platform.
    type ReleaseHardware: Serialize
        + DeserializeOwned
        + Default
        + Clone
        + PartialEq
        + Debug
        + HardwareFacts;
    /// Tree directory name at the database root.
    const DIR: &'static str;
}

/// The platform axis, stated once: a consumer macro receives every platform
/// type. Adding a system here reaches every list in the workspace.
#[macro_export]
macro_rules! with_platforms {
    ($consumer:ident) => {
        $consumer! { GameBoy, GameBoyColor, Sg1000, Vcs }
    };
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct GameBoy;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct GameBoyColor;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Vcs;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Sg1000;

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

impl Platform for Sg1000 {
    type ReleaseHardware = Sg1000Hardware;
    const DIR: &'static str = "sg1000";
}

/// Tree directory names at the database root, in platform order.
pub fn platform_dirs() -> &'static [&'static str] {
    macro_rules! dirs {
        ($($P:ident),* $(,)?) => { &[$(<$P as Platform>::DIR),*] };
    }
    with_platforms!(dirs)
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct GbHardware {
    #[serde(default, skip_serializing_if = "Enhancement::is_unknown")]
    pub sgb: Enhancement,
    #[serde(default, skip_serializing_if = "Enhancement::is_unknown")]
    pub cgb: Enhancement,
    /// Cartridge mapper, e.g. `Mbc1`, `Mbc3TimerRamBattery`; `None` = as the header says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapper: Option<GbCartType>,
}

/// Whether a Game Boy release detects and uses an enhancing console.
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
    /// Cartridge mapper, e.g. `Mbc5RumbleRamBattery`; `None` = as the header says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapper: Option<GbCartType>,
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct Sg1000Hardware {
    /// The standard of the machine this software was written against;
    /// `None` = unstated, never a default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tv_format: Option<TvStandard>,
    /// Cartridge board, e.g. `OthelloRam`, `DahjeeA`; `None` = a plain ROM.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cart_type: Option<Sg1000CartType>,
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct VcsHardware {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tv_format: Option<TvStandard>,
    /// Cartridge board, e.g. `Atari8K`, `Atari16KSuperchip`, `Plain4K`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cart_type: Option<VcsCartType>,
    /// Controllers the release needs; empty = the platform default (joystick).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub controllers: Vec<Controller>,
}

/// A VCS controller.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Controller {
    Joystick,
    Paddle,
    Driving,
    Keypad,
    Trackball,
    BoosterGrip,
    /// Coleco's Kid Vid Voice Module: an audio-cassette peripheral that plays
    /// story tapes synced to the game. A handful of Coleco titles require it.
    KidVid,
    /// Atari's MindLink: a headband read as forehead-muscle movement. Only a
    /// couple of (mostly unreleased) titles use it.
    MindLink,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_dirs_are_the_whole_axis() {
        assert_eq!(platform_dirs(), ["gb", "gbc", "sg1000", "vcs"]);
    }
}
