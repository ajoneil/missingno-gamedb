//! The declaration seam for per-release hardware: a platform's hardware struct
//! states the facts it carries, and a consumer iterates them instead of
//! carrying a match arm per platform. A fact is a key, the kind of value it
//! takes, and the curation guidance a reader needs to state it.

use crate::platform::{
    Controller, Enhancement, GbCartType, GbHardware, GbcHardware, Sg1000CartType, Sg1000Hardware,
    TvStandard, VcsCartType, VcsHardware,
};

/// One catalogue-level hardware fact a platform declares.
pub struct FactDescriptor {
    pub key: &'static str,
    pub label: &'static str,
    pub kind: FactKind,
    /// Curation guidance; feeds generated tool schemas.
    pub doc: &'static str,
}

pub enum FactKind {
    TvStandard,
    /// A board/mapper code from the platform's own vocabulary.
    Board {
        names: fn() -> Vec<&'static str>,
    },
    Controllers,
    Enhancement,
}

impl FactKind {
    /// What a value of this kind is called where one of the wrong kind arrives.
    fn noun(&self) -> &'static str {
        match self {
            FactKind::TvStandard => "TV standard",
            FactKind::Board { .. } => "board code",
            FactKind::Controllers => "controller list",
            FactKind::Enhancement => "enhancement",
        }
    }
}

/// A fact's value, carrying the same optionality the hardware struct's field
/// does: a key a platform states always reads back, stated or not.
#[derive(Clone, PartialEq, Debug)]
pub enum FactValue {
    TvStandard(Option<TvStandard>),
    /// A code from the board vocabulary; `None` clears back to unstated.
    Board(Option<String>),
    /// Empty = the platform default.
    Controllers(Vec<Controller>),
    /// `Unknown` = absent.
    Enhancement(Enhancement),
}

pub trait HardwareFacts {
    fn descriptors() -> &'static [FactDescriptor];
    fn get(&self, key: &str) -> Option<FactValue>;
    /// `Err` on a key this platform doesn't state, a value of the wrong kind,
    /// or a board code outside the platform's vocabulary.
    fn set(&mut self, key: &str, value: FactValue) -> Result<(), String>;
}

fn board_fact<B: missingno_vcs::BoardVocabulary>(board: Option<B>) -> FactValue {
    FactValue::Board(board.map(|board| board.name().to_owned()))
}

fn unknown_key(key: &str, descriptors: &'static [FactDescriptor]) -> String {
    let keys: Vec<_> = descriptors.iter().map(|fact| fact.key).collect();
    format!(
        "unknown hardware fact {key:?}; this platform states {}",
        keys.join(", ")
    )
}

fn wrong_kind(key: &str, descriptors: &'static [FactDescriptor]) -> String {
    let expected = descriptors
        .iter()
        .find(|fact| fact.key == key)
        .map_or("another kind of", |fact| fact.kind.noun());
    format!("hardware fact {key:?} takes a {expected} value")
}

fn tv_standard(
    key: &str,
    value: FactValue,
    descriptors: &'static [FactDescriptor],
) -> Result<Option<TvStandard>, String> {
    match value {
        FactValue::TvStandard(standard) => Ok(standard),
        _ => Err(wrong_kind(key, descriptors)),
    }
}

fn board<B: missingno_vcs::BoardVocabulary>(
    key: &str,
    value: FactValue,
    descriptors: &'static [FactDescriptor],
) -> Result<Option<B>, String> {
    let FactValue::Board(code) = value else {
        return Err(wrong_kind(key, descriptors));
    };
    let Some(code) = code else {
        return Ok(None);
    };
    B::from_name(&code).map(Some).ok_or_else(|| {
        format!(
            "unknown board name {code:?}; expected one of: {}",
            B::names().join(", ")
        )
    })
}

fn controllers(
    key: &str,
    value: FactValue,
    descriptors: &'static [FactDescriptor],
) -> Result<Vec<Controller>, String> {
    match value {
        FactValue::Controllers(controllers) => Ok(controllers),
        _ => Err(wrong_kind(key, descriptors)),
    }
}

fn enhancement(
    key: &str,
    value: FactValue,
    descriptors: &'static [FactDescriptor],
) -> Result<Enhancement, String> {
    match value {
        FactValue::Enhancement(enhancement) => Ok(enhancement),
        _ => Err(wrong_kind(key, descriptors)),
    }
}

const GB_MAPPER_DOC: &str = "Cartridge mapper for this release. It overrides the header byte, which unlicensed \
     carts lie about; unstated = as the header says.";

impl HardwareFacts for GbHardware {
    fn descriptors() -> &'static [FactDescriptor] {
        &[
            FactDescriptor {
                key: "sgb",
                label: "Super Game Boy",
                kind: FactKind::Enhancement,
                doc: "Whether the release detects and uses the Super Game Boy. Unknown is \
                      honest absence of data: it backfills from an external source, never \
                      by assumption.",
            },
            FactDescriptor {
                key: "cgb",
                label: "Game Boy Color",
                kind: FactKind::Enhancement,
                doc: "Whether the release detects and uses a Game Boy Color. Unknown is \
                      honest absence of data: it backfills from an external source, never \
                      by assumption.",
            },
            FactDescriptor {
                key: "mapper",
                label: "Mapper",
                kind: FactKind::Board {
                    names: <GbCartType as missingno_vcs::BoardVocabulary>::names,
                },
                doc: GB_MAPPER_DOC,
            },
        ]
    }

    fn get(&self, key: &str) -> Option<FactValue> {
        match key {
            "sgb" => Some(FactValue::Enhancement(self.sgb)),
            "cgb" => Some(FactValue::Enhancement(self.cgb)),
            "mapper" => Some(board_fact(self.mapper)),
            _ => None,
        }
    }

    fn set(&mut self, key: &str, value: FactValue) -> Result<(), String> {
        let descriptors = Self::descriptors();
        match key {
            "sgb" => self.sgb = enhancement(key, value, descriptors)?,
            "cgb" => self.cgb = enhancement(key, value, descriptors)?,
            "mapper" => self.mapper = board(key, value, descriptors)?,
            _ => return Err(unknown_key(key, descriptors)),
        }
        Ok(())
    }
}

impl HardwareFacts for GbcHardware {
    fn descriptors() -> &'static [FactDescriptor] {
        &[FactDescriptor {
            key: "mapper",
            label: "Mapper",
            kind: FactKind::Board {
                names: <GbCartType as missingno_vcs::BoardVocabulary>::names,
            },
            doc: GB_MAPPER_DOC,
        }]
    }

    fn get(&self, key: &str) -> Option<FactValue> {
        match key {
            "mapper" => Some(board_fact(self.mapper)),
            _ => None,
        }
    }

    fn set(&mut self, key: &str, value: FactValue) -> Result<(), String> {
        let descriptors = Self::descriptors();
        match key {
            "mapper" => self.mapper = board(key, value, descriptors)?,
            _ => return Err(unknown_key(key, descriptors)),
        }
        Ok(())
    }
}

impl HardwareFacts for Sg1000Hardware {
    fn descriptors() -> &'static [FactDescriptor] {
        &[
            FactDescriptor {
                key: "tv_format",
                label: "TV format",
                kind: FactKind::TvStandard,
                doc: "The standard of the machine the software was written against — the \
                      presentation its home market saw. The console fixes the standard, not \
                      the cartridge, so this is a market fact, recorded explicitly on every \
                      release; unstated is never a default.",
            },
            FactDescriptor {
                key: "cart_type",
                label: "Board",
                kind: FactKind::Board {
                    names: <Sg1000CartType as missingno_vcs::BoardVocabulary>::names,
                },
                doc: "Cartridge board code, e.g. \"OTHELLO\", \"DAHJEE-A\". An SG-1000 dump \
                      carries no header and no length that tells a RAM-bearing board from a \
                      plain one, so the database is the only thing that can name one; \
                      unstated = a plain ROM.",
            },
        ]
    }

    fn get(&self, key: &str) -> Option<FactValue> {
        match key {
            "tv_format" => Some(FactValue::TvStandard(self.tv_format)),
            "cart_type" => Some(board_fact(self.cart_type)),
            _ => None,
        }
    }

    fn set(&mut self, key: &str, value: FactValue) -> Result<(), String> {
        let descriptors = Self::descriptors();
        match key {
            "tv_format" => self.tv_format = tv_standard(key, value, descriptors)?,
            "cart_type" => self.cart_type = board(key, value, descriptors)?,
            _ => return Err(unknown_key(key, descriptors)),
        }
        Ok(())
    }
}

impl HardwareFacts for VcsHardware {
    fn descriptors() -> &'static [FactDescriptor] {
        &[
            FactDescriptor {
                key: "tv_format",
                label: "TV format",
                kind: FactKind::TvStandard,
                doc: "The standard this release was built for. PalM is Brazil's PAL-M: PAL \
                      colour on System M's 525-line/59.94 Hz raster, so it runs at NTSC \
                      timing, not PAL's — never file a Brazilian release as Pal.",
            },
            FactDescriptor {
                key: "cart_type",
                label: "Board",
                kind: FactKind::Board {
                    names: <VcsCartType as missingno_vcs::BoardVocabulary>::names,
                },
                doc: "Cartridge board code, e.g. \"F8\", \"F6SC\", \"4K\" — stated per \
                      release where the board differs or an import got it wrong; unstated = \
                      as the dump's length reads.",
            },
            FactDescriptor {
                key: "controllers",
                label: "Controllers",
                kind: FactKind::Controllers,
                doc: "Controllers the release needs, staged only when it deviates from the \
                      platform default (joystick) or when sibling releases of one game \
                      differ and the contrast is the fact — a joystick conversion beside the \
                      paddle original. Empty = the default.",
            },
        ]
    }

    fn get(&self, key: &str) -> Option<FactValue> {
        match key {
            "tv_format" => Some(FactValue::TvStandard(self.tv_format)),
            "cart_type" => Some(board_fact(self.cart_type)),
            "controllers" => Some(FactValue::Controllers(self.controllers.clone())),
            _ => None,
        }
    }

    fn set(&mut self, key: &str, value: FactValue) -> Result<(), String> {
        let descriptors = Self::descriptors();
        match key {
            "tv_format" => self.tv_format = tv_standard(key, value, descriptors)?,
            "cart_type" => self.cart_type = board(key, value, descriptors)?,
            "controllers" => self.controllers = controllers(key, value, descriptors)?,
            _ => return Err(unknown_key(key, descriptors)),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{platform::Platform, with_platforms};

    fn keys<H: HardwareFacts>() -> Vec<&'static str> {
        H::descriptors().iter().map(|fact| fact.key).collect()
    }

    fn every_key_reads_back<H: HardwareFacts + Default>() {
        let hardware = H::default();
        let mut seen = Vec::new();
        for key in keys::<H>() {
            assert!(!seen.contains(&key), "duplicate fact key {key:?}");
            seen.push(key);
            assert!(hardware.get(key).is_some(), "{key:?} does not read back");
        }
        assert_eq!(hardware.get("no_such_fact"), None);
    }

    #[test]
    fn declared_keys_read_back() {
        every_key_reads_back::<GbHardware>();
        every_key_reads_back::<GbcHardware>();
        every_key_reads_back::<Sg1000Hardware>();
        every_key_reads_back::<VcsHardware>();
    }

    #[test]
    fn every_platform_declares_facts() {
        macro_rules! declared {
            ($($P:ident),* $(,)?) => {$(
                assert!(
                    !<<$P as Platform>::ReleaseHardware as HardwareFacts>::descriptors()
                        .is_empty(),
                    "{} declares no hardware facts",
                    <$P as Platform>::DIR
                );
            )*};
        }
        use crate::platform::{GameBoy, GameBoyColor, Sg1000, Vcs};
        with_platforms!(declared);
    }

    #[test]
    fn values_roundtrip() {
        let mut vcs = VcsHardware::default();
        vcs.set("tv_format", FactValue::TvStandard(Some(TvStandard::Pal)))
            .unwrap();
        vcs.set(
            "controllers",
            FactValue::Controllers(vec![Controller::Paddle]),
        )
        .unwrap();
        vcs.set("cart_type", FactValue::Board(Some("Atari8K".to_owned())))
            .unwrap();
        assert_eq!(
            vcs.get("tv_format"),
            Some(FactValue::TvStandard(Some(TvStandard::Pal)))
        );
        assert_eq!(
            vcs.get("controllers"),
            Some(FactValue::Controllers(vec![Controller::Paddle]))
        );
        assert_eq!(
            vcs.get("cart_type"),
            Some(FactValue::Board(Some("Atari8K".to_owned())))
        );

        let mut sg1000 = Sg1000Hardware::default();
        sg1000
            .set("cart_type", FactValue::Board(Some("CastleRam".to_owned())))
            .unwrap();
        assert_eq!(sg1000.cart_type, Some(Sg1000CartType::CastleRam));
        assert_eq!(
            sg1000.get("cart_type"),
            Some(FactValue::Board(Some("CastleRam".to_owned())))
        );

        let mut gb = GbHardware::default();
        gb.set("sgb", FactValue::Enhancement(Enhancement::Enhanced))
            .unwrap();
        assert_eq!(
            gb.get("sgb"),
            Some(FactValue::Enhancement(Enhancement::Enhanced))
        );
        assert_eq!(
            gb.get("cgb"),
            Some(FactValue::Enhancement(Enhancement::Unknown))
        );
    }

    #[test]
    fn a_board_clears_back_to_unstated() {
        let mut gb = GbHardware {
            mapper: Some(GbCartType::Mbc1),
            ..GbHardware::default()
        };
        gb.set("mapper", FactValue::Board(None)).unwrap();
        assert_eq!(gb.mapper, None);
        assert_eq!(gb.get("mapper"), Some(FactValue::Board(None)));
    }

    #[test]
    fn an_unknown_key_names_the_platforms_facts() {
        let error = GbcHardware::default()
            .set("tv_format", FactValue::TvStandard(None))
            .unwrap_err();
        assert!(error.contains("\"tv_format\""), "{error}");
        assert!(error.contains("mapper"), "{error}");
    }

    #[test]
    fn an_unknown_board_name_names_the_name() {
        let error = VcsHardware::default()
            .set("cart_type", FactValue::Board(Some("F9".to_owned())))
            .unwrap_err();
        assert!(error.contains("\"F9\""), "{error}");
        assert!(error.contains("Atari8K"), "{error}");
    }

    #[test]
    fn a_value_of_the_wrong_kind_names_the_kind() {
        let error = VcsHardware::default()
            .set("tv_format", FactValue::Board(Some("Atari8K".to_owned())))
            .unwrap_err();
        assert!(error.contains("TV standard"), "{error}");
    }
}
