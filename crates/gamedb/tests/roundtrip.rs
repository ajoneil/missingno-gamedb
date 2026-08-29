use missingno_gamedb::{
    Enhancement, Game, GameBoy, GameBoyColor, Sg1000, Sg1000CartType, TvStandard, Vcs, VcsCartType,
};

const GB_HOMEBREW: &str = r#"(
    title: "144p Test Suite",
    developer: Some("Damian Yerrick"),
    tags: ["Open Source"],
    links: [
        (name: "Source Code", url: "https://github.com/pinobatch/240p-test-mini", link_type: Source),
    ],
    covers: ["https://example.org/covers/gb240p.png", "https://example.org/covers/gb240p-alt.png"],
    mods: [
        (
            name: "Grid fix",
            category: QualityOfLife,
            author: Some("someone"),
            releases: [
                (
                    label: Some("v1.1"),
                    base_sha1: Some("0123456789abcdef0123456789abcdef01234567"),
                    artifacts: [(sha1: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")],
                ),
            ],
        ),
    ],
    screenshots: ["https://example.org/shots/gb240p-linearity.png"],
    releases: [
        (
            date: Some("2018-04-17"),
            hardware: (sgb: Enhanced, cgb: Enhanced),
            artifacts: [
                (sha1: "0123456789abcdef0123456789abcdef01234567"),
            ],
        ),
    ],
)
"#;

const GBC_MINIMAL: &str = r#"(
    title: "1942",
    releases: [
        (
            regions: [Usa, Europe],
            artifacts: [(sha1: "d960e951b18d07e79d046313df49c18313664224")],
        ),
    ],
)
"#;

const VCS_VARIANTS: &str = r#"(
    title: "Pitfall II - Lost Caverns",
    releases: [
        (
            regions: [Usa],
            publisher: Some("Activision"),
            hardware: (tv_format: Some(Ntsc), cart_type: Some(Atari8K)),
            artifacts: [(sha1: "920cfbd517764ad3fa6a7425c031bd72dc7d927c")],
        ),
        (
            regions: [Europe],
            publisher: Some("Activision"),
            hardware: (tv_format: Some(Pal), cart_type: Some(Atari8K)),
            artifacts: [(sha1: "3ee18a1be7155900c2a01a104563657254d3a9a9")],
        ),
    ],
)
"#;

const GB_MOD: &str = r#"(
    title: "Wario Land II - Repainted",
    mod_of: Some((
        base_sha1: "0123456789abcdef0123456789abcdef01234567",
        category: ContentChange,
        patch: Some((format: Bps, sha1: "89abcdef0123456789abcdef0123456789abcdef")),
    )),
    links: [
        (name: "Patch", url: "https://example.org/repainted.bps", link_type: Download),
    ],
    releases: [
        (artifacts: [(sha1: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb")]),
    ],
)
"#;

fn round_trip<P: missingno_gamedb::Platform + PartialEq + std::fmt::Debug>(
    text: &str,
) -> (Game<P>, String) {
    let game = Game::<P>::from_ron(text).expect("sample parses");
    let canonical = game.to_ron_string().expect("serializes");
    let reparsed = Game::<P>::from_ron(&canonical).expect("canonical parses");
    assert_eq!(game, reparsed, "round trip preserves value");
    let again = reparsed.to_ron_string().expect("serializes again");
    assert_eq!(canonical, again, "canonical form is a fixed point");
    (game, canonical)
}

#[test]
fn gb_homebrew_round_trips() {
    let (game, _) = round_trip::<GameBoy>(GB_HOMEBREW);
    let release = &game.releases[0];
    assert_eq!(release.hardware.sgb, Enhancement::Enhanced);
    assert_eq!(release.hardware.cgb, Enhancement::Enhanced);
    assert_eq!(game.covers.len(), 2);
    assert_eq!(game.mods.len(), 1);
    assert_eq!(game.mods[0].releases[0].label.as_deref(), Some("v1.1"));
}

#[test]
fn gbc_minimal_round_trips_and_omits_defaults() {
    let (game, canonical) = round_trip::<GameBoyColor>(GBC_MINIMAL);
    assert_eq!(game.releases[0].artifacts.len(), 1);
    for absent in ["hardware", "sources", "developer", "label", "date"] {
        assert!(
            !canonical.contains(absent),
            "default field {absent:?} should be omitted:\n{canonical}"
        );
    }
}

#[test]
fn vcs_variants_round_trip() {
    let (game, _) = round_trip::<Vcs>(VCS_VARIANTS);
    assert_eq!(game.releases.len(), 2);
    assert_eq!(game.releases[0].hardware.tv_format, Some(TvStandard::Ntsc));
    assert_eq!(game.releases[1].hardware.tv_format, Some(TvStandard::Pal));
    assert_eq!(
        game.releases[1].hardware.cart_type,
        Some(VcsCartType::Atari8K)
    );
}

const SG1000_RAM_CART: &str = r#"(
    title: "Othello",
    releases: [
        (
            regions: [Japan],
            hardware: (cart_type: Some(OthelloRam(rom: Some(32768)))),
            artifacts: [(sha1: "d0cd594ddb321f707ddba8a044fa3e9b906e720a")],
        ),
        (
            regions: [NewZealand],
            hardware: (tv_format: Some(Pal)),
            artifacts: [(sha1: "a43aef367857a681decea52377c2e7a992c2ac68")],
        ),
    ],
)
"#;

#[test]
fn sg1000_board_round_trips() {
    let (game, canonical) = round_trip::<Sg1000>(SG1000_RAM_CART);
    assert_eq!(
        game.releases[0].hardware.cart_type,
        Some(Sg1000CartType::OthelloRam { rom: Some(32768) })
    );
    assert_eq!(game.releases[0].hardware.tv_format, None);
    assert_eq!(game.releases[1].hardware.tv_format, Some(TvStandard::Pal));
    assert!(
        canonical.contains(r#"cart_type: Some(OthelloRam("#),
        "{canonical}"
    );
    assert!(canonical.contains("rom: Some(32768)"), "{canonical}");
}

#[test]
fn mod_round_trips() {
    let (game, _) = round_trip::<GameBoy>(GB_MOD);
    let mod_of = game.mod_of.expect("mod block present");
    assert_eq!(
        mod_of.base_sha1.as_str(),
        "0123456789abcdef0123456789abcdef01234567"
    );
}

#[test]
fn unknown_enhancement_is_omitted() {
    let text = r#"(
        title: "Half Known",
        releases: [
            (
                hardware: (cgb: Enhanced),
                artifacts: [(sha1: "0123456789abcdef0123456789abcdef01234567")],
            ),
        ],
    )
    "#;
    let game = Game::<GameBoy>::from_ron(text).expect("parses");
    assert_eq!(game.releases[0].hardware.sgb, Enhancement::Unknown);
    let canonical = game.to_ron_string().expect("serializes");
    assert!(canonical.contains("cgb: Enhanced"));
    assert!(
        !canonical.contains("sgb"),
        "unknown sgb omitted:\n{canonical}"
    );
}

#[test]
fn unknown_fields_are_rejected() {
    let text = r#"(
        title: "Stray",
        bogus: 1,
        releases: [],
    )"#;
    assert!(Game::<GameBoy>::from_ron(text).is_err());
}

#[test]
fn bad_sha1_is_rejected() {
    let text = r#"(
        title: "Short Hash",
        releases: [(artifacts: [(sha1: "abc123")])],
    )"#;
    assert!(Game::<GameBoy>::from_ron(text).is_err());
}
