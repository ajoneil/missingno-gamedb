use std::path::Path;

use missingno_gamedb::{
    Artifact, Game, GameBoy, GameBoyColor, Link, LinkType, Platform, Release, Sha1,
};

use crate::{
    legacy::{self, LegacySource},
    report::Report,
    text::parse_region_list,
    tree,
};

const GBDEV_ENTRIES: &str = "https://raw.githubusercontent.com/gbdev/database/master/entries";

#[derive(Default)]
pub struct Stats {
    pub migrated: usize,
}

pub fn run(db_root: &Path, report: &mut Report) -> Result<Stats, String> {
    let mut stats = Stats::default();
    run_tree::<GameBoy>(db_root, report, &mut stats)?;
    run_tree::<GameBoyColor>(db_root, report, &mut stats)?;
    Ok(stats)
}

fn run_tree<P: Platform>(
    db_root: &Path,
    report: &mut Report,
    stats: &mut Stats,
) -> Result<(), String> {
    let entries = match legacy::load_tree::<P>(db_root, true).map_err(|e| e.to_string())? {
        Ok(entries) => entries,
        Err(failures) => {
            return Err(format!(
                "{} legacy manifests failed to parse; first: {}: {}",
                failures.len(),
                failures[0].0,
                failures[0].1
            ));
        }
    };

    for (slug, m) in entries {
        let Some(source) = m.source.clone() else {
            continue; // commercial — the nointro importer's job
        };

        let mut covers = Vec::new();
        let mut screenshots = Vec::new();
        match &source {
            LegacySource::HomebrewHub { slug: hub_slug, .. } => {
                let url = |file: &str| format!("{GBDEV_ENTRIES}/{hub_slug}/{file}");
                let cover_file = if m.screenshots.iter().any(|s| s == "cover.png") {
                    Some("cover.png".to_owned())
                } else {
                    m.screenshots.first().cloned()
                };
                if let Some(file) = &cover_file {
                    covers.push(url(file));
                }
                screenshots = m
                    .screenshots
                    .iter()
                    .filter(|s| *s != "cover.png")
                    .map(|s| url(s))
                    .collect();
            }
            LegacySource::Url(_) => {
                if !m.screenshots.is_empty() {
                    report.add(
                        "Screenshot filenames dropped (no URL derivable)",
                        format!("{}/{slug}", P::DIR),
                    );
                }
            }
        }

        let regions = match &m.region {
            None => Vec::new(),
            Some(text) => match parse_region_list(text) {
                Ok(regions) => regions,
                Err(unknown) => {
                    report.add(
                        "Unmappable region text dropped",
                        format!("{}/{slug}: {unknown:?}", P::DIR),
                    );
                    Vec::new()
                }
            },
        };
        let date = m.date.as_ref().and_then(|d| match d.parse() {
            Ok(date) => Some(date),
            Err(e) => {
                report.add(
                    "Unparseable dates dropped",
                    format!("{}/{slug}: {e}", P::DIR),
                );
                None
            }
        });
        let mut artifacts = Vec::new();
        for hash in &m.hashes {
            match hash.parse::<Sha1>() {
                Ok(sha1) => artifacts.push(Artifact {
                    sha1,
                    label: None,
                    size: None,
                }),
                Err(e) => report.add("Invalid hashes dropped", format!("{}/{slug}: {e}", P::DIR)),
            }
        }

        let (itch_links, mut links): (Vec<_>, Vec<_>) = m
            .links
            .iter()
            .cloned()
            .partition(|link| link.url.contains(".itch.io/"));
        // Where to obtain the ROM lives as game-level links: a direct file
        // URL as Download, obtain-from pages as DownloadPage.
        match source {
            LegacySource::HomebrewHub { slug, filename } => {
                links.push(Link {
                    name: "Homebrew Hub ROM".to_owned(),
                    url: format!("{GBDEV_ENTRIES}/{slug}/{filename}"),
                    link_type: LinkType::Download,
                });
                links.push(Link {
                    name: "Homebrew Hub".to_owned(),
                    url: format!("https://hh.gbdev.io/games/{slug}"),
                    link_type: LinkType::DownloadPage,
                });
            }
            LegacySource::Url(url) => links.push(Link {
                name: "Download".to_owned(),
                url,
                link_type: LinkType::Download,
            }),
        }
        links.extend(itch_links.into_iter().map(|link| Link {
            name: "itch.io".to_owned(),
            url: link.url,
            link_type: LinkType::DownloadPage,
        }));

        let game = Game::<P> {
            title: m.title.clone(),
            kind: Default::default(),
            developer: m.developer.clone(),
            description: m.description.clone(),
            license: m.license.clone(),
            tags: m.tags.clone(),
            links,
            covers,
            screenshots,
            mod_of: None,
            mods: Vec::new(),
            curated: false,
            adult: false,
            recommended_by: Vec::new(),
            releases: vec![Release {
                title: None,
                label: None,
                regions,
                date,
                publisher: m.publisher.clone(),
                status: Default::default(),
                hardware: Default::default(),
                artifacts,
            }],
        };
        tree::write_game(db_root, &slug, &game).map_err(|e| e.to_string())?;
        stats.migrated += 1;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn homebrew_entry_gains_urls_and_links() {
        let root = tempfile::tempdir().unwrap();
        let dir = root.path().join("gb/144p-test-suite");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("manifest.ron"),
            r#"(title: "144p Test Suite", date: Some("2018-04-17"), developer: Some("Damian Yerrick"), hashes: [], source: Some(HomebrewHub(slug: "144p-test-suite", filename: "gb240p.gb")), tags: ["Open Source"], screenshots: ["cover.png", "a.png"], links: [(name: "Website", url: "https://pinobatch.itch.io/240p-test-suite", link_type: Wiki), (name: "Source Code", url: "https://github.com/pinobatch/240p-test-mini", link_type: Source)])"#,
        )
        .unwrap();
        std::fs::create_dir_all(root.path().join("gbc")).unwrap();

        let mut report = Report::default();
        let stats = run(root.path(), &mut report).unwrap();
        assert_eq!(stats.migrated, 1);

        let text =
            std::fs::read_to_string(root.path().join("gb/144p-test-suite/manifest.ron")).unwrap();
        let game = Game::<GameBoy>::from_ron(&text).unwrap();
        assert_eq!(
            game.covers,
            vec![format!("{GBDEV_ENTRIES}/144p-test-suite/cover.png")]
        );
        assert_eq!(
            game.screenshots,
            vec![format!("{GBDEV_ENTRIES}/144p-test-suite/a.png")]
        );
        let urls: Vec<&str> = game.links.iter().map(|l| l.url.as_str()).collect();
        assert!(urls.contains(&"https://raw.githubusercontent.com/gbdev/database/master/entries/144p-test-suite/gb240p.gb"));
        assert!(urls.contains(&"https://hh.gbdev.io/games/144p-test-suite"));
        assert!(
            urls.contains(&"https://pinobatch.itch.io/240p-test-suite"),
            "itch page kept as a DownloadPage link"
        );
        assert!(missingno_gamedb::validate(root.path()).unwrap().is_empty());
    }
}
