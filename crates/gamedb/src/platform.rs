use std::fmt::Debug;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

/// The board vocabularies are the emulator cores' own, so a hardware payload
/// names a board some silicon model builds and an unlisted code fails to parse.
pub use missingno_gb::cartridge::GbCartType;
pub use missingno_sg1000::cartridge::CartType as Sg1000CartType;
pub use missingno_vcs::CartType as VcsCartType;

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

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct GbHardware {
    #[serde(default, skip_serializing_if = "Enhancement::is_unknown")]
    pub sgb: Enhancement,
    #[serde(default, skip_serializing_if = "Enhancement::is_unknown")]
    pub cgb: Enhancement,
    /// Cartridge mapper, e.g. "MBC1", "MBC3+TIMER+BATTERY"; `None` = as the header says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mapper: Option<GbCartType>,
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
    pub mapper: Option<GbCartType>,
}

/// An SG-1000 dump carries no header and no length that tells a RAM-bearing
/// board from a plain one, so the board is a database fact or nothing.
#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct Sg1000Hardware {
    /// Cartridge board code, e.g. "OTHELLO", "DAHJEE-A"; `None` = a plain ROM.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cart_type: Option<Sg1000CartType>,
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct VcsHardware {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tv_format: Option<TvFormat>,
    /// Cartridge board code, e.g. "F8", "F6SC", "4K".
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cart_type: Option<VcsCartType>,
    /// Controllers the release needs, staged only when it deviates from the
    /// platform default (VCS: joystick) or when sibling releases of one game
    /// differ and the contrast is the fact (a joystick conversion beside the
    /// paddle original). Empty = the default; every platform added later
    /// picks one default and follows the same rule.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub controllers: Vec<Controller>,
}

/// A VCS controller. An empty controller list means the joystick, which the
/// great majority of games use; the list is spelled out only when a game needs
/// something else or supports several.
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

#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum TvFormat {
    Ntsc,
    Pal,
    /// Standard PAL colour at a 60 Hz/525-line raster: correct colours on a PAL
    /// set but NTSC-speed timing. Common as a second build of homebrew and
    /// demoscene productions alongside the NTSC one; distinct from PAL-M.
    Pal60,
    /// Brazil's PAL-M: PAL colour encoding on System M's 525-line, 59.94 Hz
    /// raster, so it runs at NTSC timing rather than PAL's.
    PalM,
    Secam,
}
