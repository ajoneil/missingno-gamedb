use std::fmt::Debug;

use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::facts::HardwareFacts;

/// A board and the parts populated on it cross the facts seam untyped, so a
/// consumer states one without naming any console's enum.
pub use missingno_core::cartridge::{
    AttributeKind, AttributeSpec, AttributeValue, BoardSpec, BoardValue,
};
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
    /// Console variants this release exploits; `None` = unstated, which a
    /// booted header may fill, and an empty list states a plain Game Boy
    /// release.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "stated_list::serialize",
        deserialize_with = "stated_list::deserialize"
    )]
    pub enhancements: Option<Vec<Enhancement>>,
    /// Devices this release drives beside the console; `None` = unstated, and
    /// an empty list states it drives none.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "stated_list::serialize",
        deserialize_with = "stated_list::deserialize"
    )]
    pub peripherals: Option<Vec<Peripheral>>,
    /// Cartridge board — the mapper and the parts populated beside it, e.g.
    /// `Mbc1(rom: Kb512, ram: Some(Kb8), battery: true)`; `None` = as the
    /// header says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cart_type: Option<GbCartType>,
}

/// CGB games are CGB-required by definition.
#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct GbcHardware {
    /// Devices this release drives beside the console; `None` = unstated, and
    /// an empty list states it drives none.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "stated_list::serialize",
        deserialize_with = "stated_list::deserialize"
    )]
    pub peripherals: Option<Vec<Peripheral>>,
    /// Cartridge board — the mapper and the parts populated beside it, e.g.
    /// `Mbc5(rom: Mb1, ram: Some(Kb32), battery: true, rumble: true)`;
    /// `None` = as the header says.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cart_type: Option<GbCartType>,
}

/// A stated list is written as the bare list, so manifests from before the
/// field could be cleared read back with their meaning unchanged.
mod stated_list {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<T: Serialize, S: Serializer>(
        stated: &Option<Vec<T>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        stated.as_deref().unwrap_or(&[]).serialize(serializer)
    }

    pub fn deserialize<'de, T: Deserialize<'de>, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Vec<T>>, D::Error> {
        Vec::deserialize(deserializer).map(Some)
    }
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct Sg1000Hardware {
    /// The standard of the machine this software was written against;
    /// `None` = unstated, never a default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tv_format: Option<TvStandard>,
    /// Cartridge board and the ROM measured on it, e.g.
    /// `DahjeeA(rom: Some(49152))`; `None` = a plain ROM, nothing measured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cart_type: Option<Sg1000CartType>,
}

#[derive(Serialize, Deserialize, Default, Clone, PartialEq, Eq, Debug)]
#[serde(deny_unknown_fields)]
pub struct VcsHardware {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tv_format: Option<TvStandard>,
    /// Cartridge board, e.g. `Atari8K`, `Atari16KSuperchip`, `Plain4K`. The
    /// Tigervision family states the ROM measured on it; every other board
    /// fixes its size by wiring.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cart_type: Option<VcsCartType>,
    /// Peripherals the release needs; `None` = unstated, so the platform
    /// default (joystick) stands.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        serialize_with = "stated_list::serialize",
        deserialize_with = "stated_list::deserialize"
    )]
    pub peripherals: Option<Vec<Peripheral>>,
}

/// A console variant a release targets, drawing on what that variant adds over
/// the base machine. Each platform states which of these it offers.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Enhancement {
    /// Detects a Super Game Boy and drives its border, palette and sound.
    SuperGameBoy,
    /// Detects a Game Boy Color and draws in its palettes.
    GameBoyColor,
}

/// A device a release is played with beside the console. Each platform states
/// which of these it offers.
#[derive(Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
pub enum Peripheral {
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
    /// A second console over the Game Link cable, the name the box prints.
    LinkCable,
    /// The Game Boy Printer, a thermal printer on the link port.
    Printer,
    /// The Barcode Boy, a link-port barcode card reader.
    BarcodeBoy,
    /// The DMG-07 Four Player Adapter, a link-port hub for four consoles.
    FourPlayerAdapter,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn platform_dirs_are_the_whole_axis() {
        assert_eq!(platform_dirs(), ["gb", "gbc", "sg1000", "vcs"]);
    }
}
