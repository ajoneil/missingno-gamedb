//! The declaration seam for per-release hardware: a platform's hardware struct
//! states the facts it carries, and a consumer iterates them instead of
//! carrying a match arm per platform. A fact is a key, the kind of value it
//! takes, and the curation guidance a reader needs to state it.

use missingno_core::cartridge::{BoardSpec, BoardValue, BoardVocabulary};

use crate::platform::{
    Controller, Feature, GbCartType, GbHardware, GbcHardware, Sg1000CartType, Sg1000Hardware,
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
    /// A board from the platform's own vocabulary, and the parts that board
    /// can carry — the catalogue is what a consumer renders and edits from.
    Board {
        catalogue: fn() -> &'static [BoardSpec],
    },
    /// Controllers from the platform's own list — a platform is offered only
    /// the ones it can be played with.
    Controllers {
        catalogue: &'static [Controller],
    },
    /// Hardware the release drives, from the platform's own list.
    Features {
        catalogue: &'static [Feature],
    },
}

impl FactKind {
    /// What a value of this kind is called where one of the wrong kind arrives.
    fn noun(&self) -> &'static str {
        match self {
            FactKind::TvStandard => "TV standard",
            FactKind::Board { .. } => "board code",
            FactKind::Controllers { .. } => "controller list",
            FactKind::Features { .. } => "feature list",
        }
    }
}

/// A fact's value, carrying the same optionality the hardware struct's field
/// does: a key a platform states always reads back, stated or not.
#[derive(Clone, PartialEq, Debug)]
pub enum FactValue {
    TvStandard(Option<TvStandard>),
    /// A board of the platform's vocabulary and the parts stated on it;
    /// `None` clears back to unstated.
    Board(Option<BoardValue>),
    /// Empty = the platform default.
    Controllers(Vec<Controller>),
    /// Empty = the release drives none of them.
    Features(Vec<Feature>),
}

pub trait HardwareFacts {
    fn descriptors() -> &'static [FactDescriptor];
    fn get(&self, key: &str) -> Option<FactValue>;
    /// `Err` on a key this platform doesn't state, a value of the wrong kind,
    /// or a board statement the platform's vocabulary refuses.
    fn set(&mut self, key: &str, value: FactValue) -> Result<(), String>;
}

fn board_fact<B: BoardVocabulary>(board: Option<&B>) -> FactValue {
    FactValue::Board(board.map(BoardVocabulary::to_value))
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

/// The board a statement names. An unknown board, a part it does not carry and
/// a value outside what that part takes are all refused in the vocabulary's own
/// words, so this seam adds nothing to them.
fn board<B: BoardVocabulary>(
    key: &str,
    value: FactValue,
    descriptors: &'static [FactDescriptor],
) -> Result<Option<B>, String> {
    let FactValue::Board(stated) = value else {
        return Err(wrong_kind(key, descriptors));
    };
    stated.as_ref().map(B::from_value).transpose()
}

fn controllers(
    key: &str,
    value: FactValue,
    catalogue: &'static [Controller],
    descriptors: &'static [FactDescriptor],
) -> Result<Vec<Controller>, String> {
    let FactValue::Controllers(stated) = value else {
        return Err(wrong_kind(key, descriptors));
    };
    offered(key, stated, catalogue)
}

fn features(
    key: &str,
    value: FactValue,
    catalogue: &'static [Feature],
    descriptors: &'static [FactDescriptor],
) -> Result<Vec<Feature>, String> {
    let FactValue::Features(stated) = value else {
        return Err(wrong_kind(key, descriptors));
    };
    offered(key, stated, catalogue)
}

/// A platform states which terms it offers, so one it does not is refused
/// rather than filed as a fact nothing on this hardware could carry.
fn offered<T: Copy + PartialEq + std::fmt::Debug>(
    key: &str,
    stated: Vec<T>,
    catalogue: &'static [T],
) -> Result<Vec<T>, String> {
    match stated.iter().find(|term| !catalogue.contains(term)) {
        Some(outside) => Err(format!(
            "hardware fact {key:?} does not offer {outside:?} on this platform"
        )),
        None => Ok(stated),
    }
}

/// The consoles a Game Boy cartridge can detect and light up.
const GB_FEATURES: &[Feature] = &[Feature::SuperGameBoyEnhanced, Feature::GameBoyColorEnhanced];

/// Every controller a VCS was played with.
const VCS_CONTROLLERS: &[Controller] = &[
    Controller::Joystick,
    Controller::Paddle,
    Controller::Driving,
    Controller::Keypad,
    Controller::Trackball,
    Controller::BoosterGrip,
    Controller::KidVid,
    Controller::MindLink,
];

const GB_BOARD_DOC: &str = "Cartridge board for this release: the mapper and the parts populated beside it — the ROM \
     and RAM chips' sizes, a battery, a clock, a rumble motor. A stated board is a whole \
     statement, replacing the header, which unlicensed carts lie about; unstated = as the \
     header says.";

impl HardwareFacts for GbHardware {
    fn descriptors() -> &'static [FactDescriptor] {
        &[
            FactDescriptor {
                key: "features",
                label: "Enhances",
                kind: FactKind::Features {
                    catalogue: GB_FEATURES,
                },
                doc: "The richer consoles this release detects and lights up — a Super Game \
                      Boy border and palette, Game Boy Color palettes. The cartridge header \
                      states both, so a dump answers this; empty is a plain Game Boy \
                      release.",
            },
            FactDescriptor {
                key: "cart_type",
                label: "Board",
                kind: FactKind::Board {
                    catalogue: <GbCartType as BoardVocabulary>::catalogue,
                },
                doc: GB_BOARD_DOC,
            },
        ]
    }

    fn get(&self, key: &str) -> Option<FactValue> {
        match key {
            "features" => Some(FactValue::Features(self.features.clone())),
            "cart_type" => Some(board_fact(self.cart_type.as_ref())),
            _ => None,
        }
    }

    fn set(&mut self, key: &str, value: FactValue) -> Result<(), String> {
        let descriptors = Self::descriptors();
        match key {
            "features" => self.features = features(key, value, GB_FEATURES, descriptors)?,
            "cart_type" => self.cart_type = board(key, value, descriptors)?,
            _ => return Err(unknown_key(key, descriptors)),
        }
        Ok(())
    }
}

impl HardwareFacts for GbcHardware {
    fn descriptors() -> &'static [FactDescriptor] {
        &[FactDescriptor {
            key: "cart_type",
            label: "Board",
            kind: FactKind::Board {
                catalogue: <GbCartType as BoardVocabulary>::catalogue,
            },
            doc: GB_BOARD_DOC,
        }]
    }

    fn get(&self, key: &str) -> Option<FactValue> {
        match key {
            "cart_type" => Some(board_fact(self.cart_type.as_ref())),
            _ => None,
        }
    }

    fn set(&mut self, key: &str, value: FactValue) -> Result<(), String> {
        let descriptors = Self::descriptors();
        match key {
            "cart_type" => self.cart_type = board(key, value, descriptors)?,
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
                    catalogue: <Sg1000CartType as BoardVocabulary>::catalogue,
                },
                doc: "Cartridge board, e.g. \"OthelloRam\", \"DahjeeA\", with the ROM chip \
                      measured on it. An SG-1000 dump carries no header and no length that \
                      tells a RAM-bearing board from a plain one, so the database is the only \
                      thing that can name one; unstated = a plain ROM. The ROM is measured \
                      silicon, not the image's length: a memory-map dump runs larger than the \
                      chip it was read from, so state what the mirroring implies. Unstated \
                      where nobody has measured it — a dump we do not hold stays unmeasured, \
                      however confidently a catalogue names a size.",
            },
        ]
    }

    fn get(&self, key: &str) -> Option<FactValue> {
        match key {
            "tv_format" => Some(FactValue::TvStandard(self.tv_format)),
            "cart_type" => Some(board_fact(self.cart_type.as_ref())),
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
                    catalogue: <VcsCartType as BoardVocabulary>::catalogue,
                },
                doc: "Cartridge board, e.g. \"Atari8K\", \"Atari16KSuperchip\", \"Plain4K\" — \
                      stated per release where the board differs or an import got it wrong; \
                      unstated = as the dump's length reads. Only the Tigervision family takes \
                      a ROM size beside it, that board running 8 KB to 32 KB; every other board \
                      fixes its size by wiring, where restating it says nothing. Where it is \
                      stated it is measured silicon, unstated where nobody has measured it.",
            },
            FactDescriptor {
                key: "controllers",
                label: "Controllers",
                kind: FactKind::Controllers {
                    catalogue: VCS_CONTROLLERS,
                },
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
            "cart_type" => Some(board_fact(self.cart_type.as_ref())),
            "controllers" => Some(FactValue::Controllers(self.controllers.clone())),
            _ => None,
        }
    }

    fn set(&mut self, key: &str, value: FactValue) -> Result<(), String> {
        let descriptors = Self::descriptors();
        match key {
            "tv_format" => self.tv_format = tv_standard(key, value, descriptors)?,
            "cart_type" => self.cart_type = board(key, value, descriptors)?,
            "controllers" => {
                self.controllers = controllers(key, value, VCS_CONTROLLERS, descriptors)?
            }
            _ => return Err(unknown_key(key, descriptors)),
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use missingno_core::cartridge::AttributeValue;

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

    fn set_board<H: HardwareFacts>(hardware: &mut H, board: BoardValue) -> Result<(), String> {
        hardware.set("cart_type", FactValue::Board(Some(board)))
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
        set_board(&mut vcs, BoardValue::new("Atari8K")).unwrap();
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
            Some(FactValue::Board(Some(BoardValue::new("Atari8K"))))
        );

        let mut sg1000 = Sg1000Hardware::default();
        set_board(&mut sg1000, BoardValue::new("CastleRam")).unwrap();
        assert_eq!(
            sg1000.cart_type,
            Some(Sg1000CartType::CastleRam { rom: None })
        );

        let mut gb = GbHardware::default();
        gb.set(
            "features",
            FactValue::Features(vec![Feature::SuperGameBoyEnhanced]),
        )
        .unwrap();
        assert_eq!(
            gb.get("features"),
            Some(FactValue::Features(vec![Feature::SuperGameBoyEnhanced]))
        );
    }

    #[test]
    fn a_platform_refuses_a_term_it_does_not_offer() {
        let mut vcs = VcsHardware::default();
        assert!(
            vcs.set("features", FactValue::Features(vec![])).is_err(),
            "the VCS states no features key"
        );

        let mut gb = GbHardware::default();
        assert!(
            gb.set("features", FactValue::Controllers(vec![Controller::Paddle]))
                .is_err(),
            "a controller list is not a feature list"
        );
    }

    #[test]
    fn a_board_reads_back_the_parts_it_was_stated_with() {
        let measured = BoardValue::new("DahjeeB").with("rom", AttributeValue::Bytes(49152));
        let mut sg1000 = Sg1000Hardware::default();
        set_board(&mut sg1000, measured.clone()).unwrap();
        assert_eq!(
            sg1000.cart_type,
            Some(Sg1000CartType::DahjeeB { rom: Some(49152) })
        );
        assert_eq!(
            sg1000.get("cart_type"),
            Some(FactValue::Board(Some(measured)))
        );

        let populated = BoardValue::new("Mbc5")
            .with_choice("rom", "1M")
            .with_choice("ram", "32K")
            .with_toggle("battery", true)
            .with_toggle("rumble", false);
        let mut gb = GbHardware::default();
        set_board(&mut gb, populated.clone()).unwrap();
        assert_eq!(gb.get("cart_type"), Some(FactValue::Board(Some(populated))));
    }

    #[test]
    fn a_board_clears_back_to_unstated() {
        let mut gb = GbHardware::default();
        set_board(
            &mut gb,
            BoardValue::new("Mbc1")
                .with_choice("rom", "512K")
                .with_toggle("battery", true),
        )
        .unwrap();
        gb.set("cart_type", FactValue::Board(None)).unwrap();
        assert_eq!(gb.cart_type, None);
        assert_eq!(gb.get("cart_type"), Some(FactValue::Board(None)));
    }

    #[test]
    fn an_unknown_key_names_the_platforms_facts() {
        let error = GbcHardware::default()
            .set("tv_format", FactValue::TvStandard(None))
            .unwrap_err();
        assert!(error.contains("\"tv_format\""), "{error}");
        assert!(error.contains("cart_type"), "{error}");
    }

    #[test]
    fn a_board_the_vocabulary_refuses_is_refused_in_its_words() {
        let unknown = set_board(&mut VcsHardware::default(), BoardValue::new("F9")).unwrap_err();
        assert!(unknown.contains("unknown Atari VCS board"), "{unknown}");
        assert!(unknown.contains("F9"), "{unknown}");

        let wired = set_board(
            &mut VcsHardware::default(),
            BoardValue::new("Plain4K").with("rom", AttributeValue::Bytes(4096)),
        )
        .unwrap_err();
        assert!(wired.contains("carries no \"rom\" attribute"), "{wired}");

        let unpopulated =
            set_board(&mut GbHardware::default(), BoardValue::new("Mbc5")).unwrap_err();
        assert!(
            unpopulated.contains("needs a \"rom\" attribute"),
            "{unpopulated}"
        );
    }

    #[test]
    fn a_value_of_the_wrong_kind_names_the_kind() {
        let error = VcsHardware::default()
            .set(
                "tv_format",
                FactValue::Board(Some(BoardValue::new("Atari8K"))),
            )
            .unwrap_err();
        assert!(error.contains("TV standard"), "{error}");
    }
}
