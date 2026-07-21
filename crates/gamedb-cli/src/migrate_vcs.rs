use std::{collections::BTreeMap, path::Path};

use missingno_gamedb::{Artifact, Game, Release, ReleaseStatus, Sha1, Vcs, VcsHardware};

use crate::{
    legacy::{self, LegacyManifest},
    report::Report,
    text::normalize_title,
    tree,
};

pub struct Stats {
    pub entries_before: usize,
    pub games_after: usize,
    pub releases: usize,
}

fn unify_scalar(
    members: &[(String, LegacyManifest, bool)],
    field: impl Fn(&LegacyManifest) -> Option<&String>,
    title: &str,
    name: &str,
    report: &mut Report,
) -> Option<String> {
    let mut values: Vec<&String> = members.iter().filter_map(|(_, m, _)| field(m)).collect();
    values.sort();
    values.dedup();
    match values.len() {
        0 => None,
        1 => Some(values[0].clone()),
        _ => {
            report.add(
                "Conflicting game fields left empty",
                format!("{title:?}: {name} differs across variants"),
            );
            None
        }
    }
}

pub fn run(db_root: &Path, report: &mut Report) -> Result<Stats, String> {
    let entries = match legacy::load_tree::<Vcs>(db_root, false).map_err(|e| e.to_string())? {
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
    let entries_before = entries.len();

    let mut old_sha1s: Vec<String> = Vec::new();
    let mut groups: BTreeMap<String, Vec<(String, LegacyManifest, bool)>> = BTreeMap::new();
    for (slug, manifest) in entries {
        let (title, wip) = match manifest.title.strip_suffix(" (WIP)") {
            Some(stripped) => (stripped.to_owned(), true),
            None => (manifest.title.clone(), false),
        };
        groups.entry(title).or_default().push((slug, manifest, wip));
    }

    // Near-miss report: normalized collisions between distinct exact titles.
    let mut by_normalized: BTreeMap<String, Vec<&String>> = BTreeMap::new();
    for title in groups.keys() {
        by_normalized
            .entry(normalize_title(title))
            .or_default()
            .push(title);
    }
    for titles in by_normalized.values().filter(|t| t.len() > 1) {
        report.add(
            "Near-miss titles left unmerged",
            titles
                .iter()
                .map(|t| format!("{t:?}"))
                .collect::<Vec<_>>()
                .join(" vs "),
        );
    }

    let mut games: Vec<(String, Game<Vcs>)> = Vec::new();
    let mut releases_total = 0;
    for (title, mut members) in groups {
        members.sort_by(|a, b| {
            let key = |(slug, m, wip): &(String, LegacyManifest, bool)| {
                (
                    *wip,
                    format!("{:?}", m.tv_format),
                    m.cart_type.clone().unwrap_or_default(),
                    m.date.clone().unwrap_or_default(),
                    slug.clone(),
                )
            };
            key(a).cmp(&key(b))
        });
        let slug = members
            .iter()
            .map(|(slug, _, _)| slug.clone())
            .min_by_key(|s| (s.len(), s.clone()))
            .expect("group is non-empty");

        let mut seen_hardware = BTreeMap::new();
        let mut releases = Vec::new();
        for (member_slug, m, wip) in &members {
            let hw_key = (*wip, format!("{:?}", m.tv_format), m.cart_type.clone());
            if let Some(first) = seen_hardware.insert(hw_key, member_slug.clone()) {
                report.add(
                    "Same title with identical hardware",
                    format!("{title:?}: {first} and {member_slug}"),
                );
            }
            let date = m.date.as_ref().and_then(|d| match d.parse() {
                Ok(date) => Some(date),
                Err(e) => {
                    report.add("Unparseable dates dropped", format!("{member_slug}: {e}"));
                    None
                }
            });
            let mut artifacts = Vec::new();
            for hash in &m.hashes {
                match hash.parse::<Sha1>() {
                    Ok(sha1) => artifacts.push(Artifact {
                        sha1,
                        size: None,
                        filename: None,
                    }),
                    Err(e) => report.add("Invalid hashes dropped", format!("{member_slug}: {e}")),
                }
            }
            if artifacts.is_empty() {
                report.add(
                    "Releases with no artifacts and no sources",
                    member_slug.clone(),
                );
            }
            old_sha1s.extend(artifacts.iter().map(|a| a.sha1.as_str().to_owned()));
            releases.push(Release {
                label: None,
                regions: Vec::new(),
                date,
                publisher: m.publisher.clone(),
                status: if *wip {
                    ReleaseStatus::WorkInProgress
                } else {
                    ReleaseStatus::Released
                },
                hardware: VcsHardware {
                    tv_format: m.tv_format,
                    cart_type: m.cart_type.clone(),
                },
                sources: Vec::new(),
                artifacts,
            });
        }
        releases_total += releases.len();

        let developer = unify_scalar(
            &members,
            |m| m.developer.as_ref(),
            &title,
            "developer",
            report,
        );
        let description = unify_scalar(
            &members,
            |m| m.description.as_ref(),
            &title,
            "description",
            report,
        );
        let license = unify_scalar(&members, |m| m.license.as_ref(), &title, "license", report);
        let mut tags: Vec<String> = members
            .iter()
            .flat_map(|(_, m, _)| m.tags.clone())
            .collect();
        tags.sort();
        tags.dedup();
        let mut links = Vec::new();
        for (_, m, _) in &members {
            for link in &m.links {
                if !links.contains(link) {
                    links.push(link.clone());
                }
            }
        }
        for (member_slug, m, _) in &members {
            if !m.screenshots.is_empty() {
                report.add(
                    "Screenshot filenames dropped (no URL derivable)",
                    member_slug.clone(),
                );
            }
        }

        games.push((
            slug,
            Game {
                title,
                kind: Default::default(),
                developer,
                description,
                license,
                tags,
                links,
                covers: Vec::new(),
                screenshots: Vec::new(),
                mod_of: None,
                releases,
            },
        ));
    }

    old_sha1s.sort();
    let new_sha1s = tree::sha1_multiset(&games);
    if old_sha1s != new_sha1s {
        return Err(format!(
            "sha1 preservation violated: {} before vs {} after",
            old_sha1s.len(),
            new_sha1s.len()
        ));
    }

    let old_slugs: Vec<String> = {
        let tree_dir = db_root.join("vcs");
        let mut slugs: Vec<String> = std::fs::read_dir(&tree_dir)
            .map_err(|e| e.to_string())?
            .filter_map(|d| d.ok())
            .filter(|d| d.path().join("manifest.ron").is_file())
            .map(|d| d.file_name().to_string_lossy().into_owned())
            .collect();
        slugs.sort();
        slugs
    };
    for slug in &old_slugs {
        tree::remove_game_dir(db_root, "vcs", slug).map_err(|e| e.to_string())?;
    }
    for (slug, game) in &games {
        tree::write_game(db_root, slug, game).map_err(|e| e.to_string())?;
    }

    Ok(Stats {
        entries_before,
        games_after: games.len(),
        releases: releases_total,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn write_legacy(root: &Path, slug: &str, text: &str) {
        let dir = root.join("vcs").join(slug);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("manifest.ron"), text).unwrap();
    }

    #[test]
    fn collapses_variants_and_reports_near_misses() {
        let root = tempfile::tempdir().unwrap();
        write_legacy(
            root.path(),
            "pitfall-ii",
            r#"(title: "Pitfall II", publisher: Some("Activision"), tv_format: Some(Ntsc), cart_type: Some("F8"), hashes: ["920cfbd517764ad3fa6a7425c031bd72dc7d927c"], source: None)"#,
        );
        write_legacy(
            root.path(),
            "pitfall-ii-pal",
            r#"(title: "Pitfall II", publisher: Some("Activision"), tv_format: Some(Pal), cart_type: Some("F8"), hashes: ["3ee18a1be7155900c2a01a104563657254d3a9a9"], source: None)"#,
        );
        write_legacy(
            root.path(),
            "other",
            r#"(title: "Other", tv_format: Some(Ntsc), hashes: ["0123456789abcdef0123456789abcdef01234567"], source: None)"#,
        );
        write_legacy(
            root.path(),
            "other-wip",
            r#"(title: "Other (WIP)", tv_format: Some(Pal), hashes: ["fedcba9876543210fedcba9876543210fedcba98"], source: None)"#,
        );

        write_legacy(
            root.path(),
            "star-fire",
            r#"(title: "Star Fire", tv_format: Some(Ntsc), hashes: ["1111111111111111111111111111111111111111"], source: None)"#,
        );
        write_legacy(
            root.path(),
            "starfire",
            r#"(title: "Star-Fire", tv_format: Some(Pal), hashes: ["2222222222222222222222222222222222222222"], source: None)"#,
        );

        let mut report = Report::default();
        let stats = run(root.path(), &mut report).unwrap();
        assert_eq!(stats.entries_before, 6);
        assert_eq!(stats.games_after, 4);
        assert_eq!(stats.releases, 6);

        let merged = fs::read_to_string(root.path().join("vcs/pitfall-ii/manifest.ron")).unwrap();
        let game = Game::<Vcs>::from_ron(&merged).unwrap();
        assert_eq!(game.releases.len(), 2);
        assert_eq!(game.releases[0].publisher.as_deref(), Some("Activision"));
        assert!(!root.path().join("vcs/pitfall-ii-pal").exists());

        let other = fs::read_to_string(root.path().join("vcs/other/manifest.ron")).unwrap();
        let other = Game::<Vcs>::from_ron(&other).unwrap();
        assert_eq!(other.title, "Other");
        assert_eq!(other.releases.len(), 2);
        assert_eq!(other.releases[0].status, ReleaseStatus::Released);
        assert_eq!(other.releases[1].status, ReleaseStatus::WorkInProgress);
        assert!(!root.path().join("vcs/other-wip").exists());

        assert!(report.render("t").contains("Star Fire"));
        assert!(missingno_gamedb::validate(root.path()).unwrap().is_empty());
    }
}
