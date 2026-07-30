use std::{fs, path::Path};

use missingno_gamedb::{Flag, FlagFile, FlagKind};

use crate::report::Report;

/// Sections that record completed actions rather than open questions.
const SKIP_SECTIONS: [&str; 2] = [
    "BIOS entries skipped",
    "Pre-release families merged into their retail game",
];

fn kind_for(section: &str) -> FlagKind {
    match section {
        "Near-miss titles left unmerged" => FlagKind::NearMissTitles,
        "Same-title families left separate (review candidates)" => {
            FlagKind::ReviewCandidateFamilies
        }
        s if s.starts_with("Leftover old entries") => FlagKind::Leftover,
        "Unknown name qualifiers" => FlagKind::UnknownQualifier,
        "Conflicting game fields left empty" => FlagKind::ConflictingField,
        s if s.starts_with("Old hashes absent") => FlagKind::RetiredHash,
        _ => FlagKind::Custom,
    }
}

/// Every "tree/slug" reference in an item line.
fn subjects(item: &str) -> Vec<String> {
    let mut found = Vec::new();
    for tree in ["gb", "gbc", "vcs"] {
        let mut rest = item;
        while let Some(at) = rest.find(&format!("{tree}/")) {
            let tail = &rest[at..];
            let end = tail
                .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '/'))
                .unwrap_or(tail.len());
            let candidate = &tail[..end];
            if candidate.len() > tree.len() + 1 && !found.contains(&candidate.to_owned()) {
                found.push(candidate.to_owned());
            }
            rest = &rest[at + tree.len() + 1..];
        }
    }
    found
}

pub struct Stats {
    pub imported: usize,
    pub skipped_sections: usize,
    pub duplicates: usize,
}

pub fn run(
    db_repo: &Path,
    reports: &[std::path::PathBuf],
    _report: &mut Report,
) -> Result<Stats, String> {
    let mut file = FlagFile::load(db_repo).map_err(|e| e.to_string())?;
    let mut stats = Stats {
        imported: 0,
        skipped_sections: 0,
        duplicates: 0,
    };
    let mut next_id = file.next_id();

    for path in reports {
        let text = fs::read_to_string(path).map_err(|e| format!("{path:?}: {e}"))?;
        let mut section = String::new();
        let mut skipping = false;
        for line in text.lines() {
            if let Some(heading) = line.strip_prefix("## ") {
                section = heading
                    .rsplit_once(" (")
                    .map(|(name, _)| name)
                    .unwrap_or(heading)
                    .to_owned();
                skipping = SKIP_SECTIONS.contains(&section.as_str());
                if skipping {
                    stats.skipped_sections += 1;
                }
                continue;
            }
            let Some(item) = line.strip_prefix("- ") else {
                continue;
            };
            if skipping || section.is_empty() {
                continue;
            }
            let kind = kind_for(&section);
            let note = format!("{section}: {item}");
            if file.flags.iter().any(|f| f.note == note) {
                stats.duplicates += 1;
                continue;
            }
            file.flags.push(Flag {
                id: next_id,
                kind,
                subject: subjects(item),
                note,
            });
            next_id += 1;
            stats.imported += 1;
        }
    }

    file.save(db_repo).map_err(|e| e.to_string())?;
    Ok(stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sections_and_subjects() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("gb")).unwrap();
        let report = dir.path().join("r.md");
        std::fs::write(
            &report,
            "# t\n\n## Near-miss titles left unmerged (1)\n\n- \"A\" vs \"A (x)\"\n\n\
             ## Leftover old entries (not in any DAT) — mechanically converted (1)\n\n- gb/hugo\n\n\
             ## BIOS entries skipped (1)\n\n- \"[BIOS] X\"\n",
        )
        .unwrap();
        let mut r = Report::default();
        let stats = run(dir.path(), &[report.clone()], &mut r).unwrap();
        assert_eq!(stats.imported, 2);

        let flags = FlagFile::load(dir.path()).unwrap();
        assert_eq!(flags.flags.len(), 2);
        assert_eq!(flags.flags[0].kind, FlagKind::NearMissTitles);
        assert_eq!(flags.flags[1].kind, FlagKind::Leftover);
        assert_eq!(flags.flags[1].subject, vec!["gb/hugo".to_owned()]);

        // Re-import is a no-op.
        let stats = run(dir.path(), &[report], &mut r).unwrap();
        assert_eq!(stats.imported, 0);
        assert_eq!(stats.duplicates, 2);
    }
}
