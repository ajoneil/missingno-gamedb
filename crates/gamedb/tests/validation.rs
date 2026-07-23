use std::{fs, path::Path};

use missingno_gamedb::{Game, GameBoy, Platform, Severity, Vcs, validate};

fn write_canonical<P: Platform>(root: &Path, slug: &str, text: &str) {
    let game = Game::<P>::from_ron(text).expect("fixture parses");
    let dir = root.join(P::DIR).join(slug);
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("manifest.ron"),
        game.to_ron_string().expect("fixture serializes"),
    )
    .unwrap();
}

fn write_raw(root: &Path, tree: &str, slug: &str, text: &str) {
    let dir = root.join(tree).join(slug);
    fs::create_dir_all(&dir).unwrap();
    fs::write(dir.join("manifest.ron"), text).unwrap();
}

const GB_GOOD: &str = r#"(
    title: "Good Game",
    releases: [
        (artifacts: [(sha1: "0123456789abcdef0123456789abcdef01234567")]),
    ],
)"#;

#[test]
fn clean_tree_validates() {
    let root = tempfile::tempdir().unwrap();
    write_canonical::<GameBoy>(root.path(), "good-game", GB_GOOD);
    write_canonical::<Vcs>(
        root.path(),
        "combat",
        r#"(
            title: "Combat",
            releases: [
                (
                    hardware: (tv_format: Some(Ntsc), cart_type: Some("2K")),
                    artifacts: [(sha1: "fedcba9876543210fedcba9876543210fedcba98")],
                ),
            ],
        )"#,
    );
    let findings = validate(root.path()).unwrap();
    assert!(findings.is_empty(), "{findings:?}");
}

#[test]
fn format_drift_is_an_error() {
    let root = tempfile::tempdir().unwrap();
    write_raw(
        root.path(),
        "gb",
        "drifted",
        "(title: \"Good Game\", releases: [(artifacts: [(sha1: \"0123456789abcdef0123456789abcdef01234567\")])])",
    );
    let findings = validate(root.path()).unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f.severity == Severity::Error && f.message.contains("canonically")),
        "{findings:?}"
    );
}

#[test]
fn game_without_releases_is_an_error() {
    let root = tempfile::tempdir().unwrap();
    write_canonical::<GameBoy>(root.path(), "empty-game", r#"(title: "Empty")"#);
    let findings = validate(root.path()).unwrap();
    assert!(
        findings.iter().any(|f| f.message.contains("no releases")),
        "{findings:?}"
    );
}

#[test]
fn unlocatable_unverifiable_release_is_an_error() {
    let root = tempfile::tempdir().unwrap();
    write_canonical::<GameBoy>(root.path(), "ghost", r#"(title: "Ghost", releases: [()])"#);
    let findings = validate(root.path()).unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f.message.contains("no artifacts")),
        "{findings:?}"
    );
}

#[test]
fn duplicate_sha1_is_an_error() {
    let root = tempfile::tempdir().unwrap();
    write_canonical::<GameBoy>(root.path(), "first", GB_GOOD);
    write_canonical::<GameBoy>(
        root.path(),
        "second",
        r#"(
            title: "Second Game",
            releases: [
                (artifacts: [(sha1: "0123456789abcdef0123456789abcdef01234567")]),
            ],
        )"#,
    );
    let findings = validate(root.path()).unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f.severity == Severity::Error && f.message.contains("duplicate sha1")),
        "{findings:?}"
    );
}

#[test]
fn unresolved_mod_base_is_a_warning() {
    let root = tempfile::tempdir().unwrap();
    write_canonical::<GameBoy>(
        root.path(),
        "orphan-mod",
        r#"(
            title: "Orphan Mod",
            mod_of: Some((
                base_sha1: "fedcba9876543210fedcba9876543210fedcba98",
                category: Translation,
                patch: Some((format: Ips, sha1: "0123456789abcdef0123456789abcdef01234567")),
            )),
            releases: [
                (artifacts: [(sha1: "cccccccccccccccccccccccccccccccccccccccc")]),
            ],
        )"#,
    );
    let findings = validate(root.path()).unwrap();
    assert!(
        findings
            .iter()
            .any(|f| f.severity == Severity::Warning && f.message.contains("mod base sha1")),
        "{findings:?}"
    );
}

#[test]
fn invalid_slug_is_an_error() {
    let root = tempfile::tempdir().unwrap();
    write_raw(root.path(), "gb", "Bad Slug", "garbage");
    let findings = validate(root.path()).unwrap();
    assert!(
        findings.iter().any(|f| f.message.contains("invalid slug")),
        "{findings:?}"
    );
    assert!(
        findings.iter().any(|f| f.message.contains("parse error")),
        "{findings:?}"
    );
}
