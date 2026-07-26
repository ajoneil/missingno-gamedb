use std::{collections::HashMap, fmt, fs, io, path::Path};

use crate::{
    game::Game,
    ids::{Sha1, Slug},
    load::manifest_paths,
    platform::{GameBoy, GameBoyColor, Platform, Vcs},
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Severity {
    Warning,
    Error,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Warning => "warning",
            Self::Error => "error",
        })
    }
}

#[derive(Clone, Debug)]
pub struct Finding {
    pub path: String,
    pub severity: Severity,
    pub message: String,
}

impl Finding {
    fn error(path: &str, message: String) -> Self {
        Self {
            path: path.to_owned(),
            severity: Severity::Error,
            message,
        }
    }

    fn warning(path: &str, message: String) -> Self {
        Self {
            path: path.to_owned(),
            severity: Severity::Warning,
            message,
        }
    }
}

/// Validate every platform tree under the database root.
pub fn validate(db_root: &Path) -> io::Result<Vec<Finding>> {
    let mut findings = validate_tree::<GameBoy>(db_root)?;
    findings.extend(validate_tree::<GameBoyColor>(db_root)?);
    findings.extend(validate_tree::<Vcs>(db_root)?);
    Ok(findings)
}

fn validate_tree<P: Platform>(db_root: &Path) -> io::Result<Vec<Finding>> {
    let mut findings = Vec::new();
    let mut games = Vec::new();

    for (name, path) in manifest_paths(db_root, P::DIR)? {
        let display = path.display().to_string();
        if let Err(message) = name.parse::<Slug>() {
            findings.push(Finding::error(&display, message));
        }
        let text = fs::read_to_string(&path)?;
        let game = match Game::<P>::from_ron(&text) {
            Ok(game) => game,
            Err(e) => {
                findings.push(Finding::error(&display, format!("parse error: {e}")));
                continue;
            }
        };
        match game.to_ron_string() {
            Ok(canonical) if canonical != text => findings.push(Finding::error(
                &display,
                "not canonically formatted (run `gamedb fmt`)".to_owned(),
            )),
            Ok(_) => {}
            Err(e) => findings.push(Finding::error(&display, format!("serialize error: {e}"))),
        }
        games.push((display, game));
    }

    let mut seen_sha1: HashMap<Sha1, String> = HashMap::new();
    for (display, game) in &games {
        if game.releases.is_empty() {
            findings.push(Finding::error(display, "game has no releases".to_owned()));
        }
        // A game is locatable when a link says where to obtain it — free or
        // for sale; a release without artifacts is only an error when the game
        // is unlocatable too.
        let locatable = game.links.iter().any(|l| {
            matches!(
                l.link_type,
                crate::LinkType::Download | crate::LinkType::DownloadPage | crate::LinkType::Store
            )
        });
        for release in &game.releases {
            if release.artifacts.is_empty() && !locatable {
                findings.push(Finding::error(
                    display,
                    "release has no artifacts and the game has no download link".to_owned(),
                ));
            }
            for artifact in &release.artifacts {
                if let Some(first) = seen_sha1.insert(artifact.sha1.clone(), display.clone()) {
                    findings.push(Finding::error(
                        display,
                        format!("duplicate sha1 {} (also in {first})", artifact.sha1),
                    ));
                }
            }
        }
        for game_mod in &game.mods {
            for release in &game_mod.releases {
                for artifact in &release.artifacts {
                    if let Some(first) = seen_sha1.insert(artifact.sha1.clone(), display.clone()) {
                        findings.push(Finding::error(
                            display,
                            format!("duplicate sha1 {} (also in {first})", artifact.sha1),
                        ));
                    }
                }
            }
        }
    }

    for (display, game) in &games {
        for game_mod in &game.mods {
            for release in &game_mod.releases {
                if let Some(base) = &release.base_sha1
                    && !seen_sha1.contains_key(base)
                {
                    findings.push(Finding::warning(
                        display,
                        format!(
                            "mod {:?} base sha1 {base} not found in the {} tree",
                            game_mod.name,
                            P::DIR
                        ),
                    ));
                }
            }
        }
        if let Some(mod_of) = &game.mod_of
            && !seen_sha1.contains_key(&mod_of.base_sha1)
        {
            findings.push(Finding::warning(
                display,
                format!(
                    "mod base sha1 {} not found in the {} tree",
                    mod_of.base_sha1,
                    P::DIR
                ),
            ));
        }
    }

    Ok(findings)
}

#[derive(Default, Debug)]
pub struct FormatReport {
    pub rewritten: Vec<String>,
    pub errors: Vec<Finding>,
}

/// Rewrite every manifest in canonical formatting. Files that fail to parse
/// are reported and left untouched.
pub fn format_all(db_root: &Path) -> io::Result<FormatReport> {
    let mut report = FormatReport::default();
    format_tree::<GameBoy>(db_root, &mut report)?;
    format_tree::<GameBoyColor>(db_root, &mut report)?;
    format_tree::<Vcs>(db_root, &mut report)?;
    Ok(report)
}

fn format_tree<P: Platform>(db_root: &Path, report: &mut FormatReport) -> io::Result<()> {
    for (_name, path) in manifest_paths(db_root, P::DIR)? {
        let display = path.display().to_string();
        let text = fs::read_to_string(&path)?;
        match Game::<P>::from_ron(&text) {
            Ok(game) => match game.to_ron_string() {
                Ok(canonical) if canonical != text => {
                    fs::write(&path, canonical)?;
                    report.rewritten.push(display);
                }
                Ok(_) => {}
                Err(e) => report
                    .errors
                    .push(Finding::error(&display, format!("serialize error: {e}"))),
            },
            Err(e) => report
                .errors
                .push(Finding::error(&display, format!("parse error: {e}"))),
        }
    }
    Ok(())
}
