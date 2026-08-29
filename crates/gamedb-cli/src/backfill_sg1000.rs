//! Backfill the two facts curation kept missing on SG-1000 entries: the title
//! a release shipped under in its own script, and the ROM measured on its
//! board. Both are read from sources outside the tree — MAME's software list
//! keys a native title by dump hash, and a local ROM collection is what makes
//! a size a measurement rather than a copied number.

use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use missingno_gamedb::{
    AttributeValue, BoardValue, Defect, FactValue, HardwareFacts, Platform, Sg1000, Sg1000CartType,
    Sg1000Hardware, Tree,
};

use crate::{report::Report, tree};

#[derive(Default)]
pub struct Stats {
    pub titled: usize,
    pub sized: usize,
    pub skipped_romanised: usize,
    pub skipped_no_dump: usize,
}

/// `alt_title` holds `pinyin ~ native` for the Taiwanese entries and the native
/// form alone elsewhere; the native side is the half after the tilde.
fn native_title(alt: &str) -> Option<&str> {
    let native = alt.rsplit('~').next()?.trim();
    (!native.is_empty()).then_some(native)
}

/// A romanisation is not native script, so it is not a release title. Latin
/// text with no CJK in it is MAME recording the reading, not the name.
fn is_romanisation(title: &str) -> bool {
    !title.chars().any(|c| {
        matches!(c as u32,
            0x3040..=0x30FF     // kana
            | 0x3400..=0x4DBF   // CJK ext A
            | 0x4E00..=0x9FFF   // CJK unified
            | 0xAC00..=0xD7AF   // hangul
            | 0xFF66..=0xFF9F) // halfwidth kana
    })
}

/// sha1 → native title, from MAME's `hash/sg1000.xml`.
fn softlist_titles(path: &Path) -> Result<HashMap<String, String>, String> {
    let xml = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut by_hash = HashMap::new();
    for block in xml.split("<software ").skip(1) {
        let end = block.find("</software>").unwrap_or(block.len());
        let block = &block[..end];
        let Some(alt) = field(block, "alt_title\" value=\"") else {
            continue;
        };
        let Some(native) = native_title(alt) else {
            continue;
        };
        for sha1 in block.match_indices("sha1=\"").filter_map(|(at, tag)| {
            let rest = &block[at + tag.len()..];
            rest.find('"').map(|end| &rest[..end])
        }) {
            by_hash.insert(sha1.to_ascii_lowercase(), native.to_owned());
        }
    }
    Ok(by_hash)
}

fn field<'a>(block: &'a str, tag: &str) -> Option<&'a str> {
    let at = block.find(tag)? + tag.len();
    let rest = &block[at..];
    rest.find('"').map(|end| &rest[..end])
}

/// The dumps we hold, as `sha1sum` prints them: a hash and the file it is of.
/// Passed in rather than hashed here so the set a run acted on is a file you
/// can read back — and the file is what the ROM's size is measured off.
fn local_dumps(path: &Path) -> Result<HashMap<String, PathBuf>, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let mut dumps = HashMap::new();
    for line in text.lines() {
        let Some((hash, file)) = line.split_once(char::is_whitespace) else {
            continue;
        };
        // `sha1sum` marks a binary-mode read with a `*` before the name.
        let file = file.trim_start().trim_start_matches('*');
        if hash.len() == 40 && !file.is_empty() {
            dumps.insert(hash.to_ascii_lowercase(), PathBuf::from(file));
        }
    }
    Ok(dumps)
}

/// The ROM measured on the release's board, where anyone has measured it.
fn board_rom(hardware: &Sg1000Hardware) -> Option<u32> {
    hardware.cart_type.and_then(Sg1000CartType::rom)
}

/// State the measured ROM on the release's board — the plain ROM `Flat` where
/// the release names no board.
fn state_rom(hardware: &mut Sg1000Hardware, bytes: u32) -> Result<(), String> {
    let board = match hardware.get("cart_type") {
        Some(FactValue::Board(Some(board))) => board,
        _ => BoardValue::new("Flat"),
    };
    hardware.set(
        "cart_type",
        FactValue::Board(Some(board.with("rom", AttributeValue::Bytes(bytes)))),
    )
}

pub fn run(
    data_root: &Path,
    softlist: &Path,
    held_list: &Path,
    report: &mut Report,
    dry_run: bool,
) -> Result<Stats, String> {
    let titles = softlist_titles(softlist)?;
    let held = local_dumps(held_list)?;
    let mut stats = Stats::default();

    let (tree, issues) = Tree::<Sg1000>::load(data_root).map_err(|e| e.to_string())?;
    if let Some(first) = issues.first() {
        return Err(format!("{}: {}", first.path.display(), first.message));
    }

    for entry in tree.games {
        let slug = entry.slug.as_str().to_owned();
        let mut game = entry.game;
        if !game.curated {
            continue;
        }
        let mut changed = false;

        for (index, release) in game.releases.iter_mut().enumerate() {
            let key = format!("{}/{slug} release {index}", Sg1000::DIR);
            let hashes: Vec<String> = release
                .artifacts
                .iter()
                .map(|a| a.sha1.as_str().to_ascii_lowercase())
                .collect();

            if release.title.is_none()
                && let Some(native) = hashes.iter().find_map(|h| titles.get(h))
            {
                if is_romanisation(native) {
                    report.add(
                        "Softlist gives a romanisation, not native script",
                        format!("{key}: {native:?} — left empty"),
                    );
                    stats.skipped_romanised += 1;
                } else {
                    release.title = Some(native.clone());
                    stats.titled += 1;
                    changed = true;
                }
            }

            // A memory map is longer than the silicon it was read from, so its
            // size is a measurement of the mirroring, not the image's length.
            let mapped = release
                .artifacts
                .iter()
                .any(|a| a.defect == Some(Defect::MemoryMap));
            if board_rom(&release.hardware).is_none() {
                if mapped {
                    report.add("Memory map: the ROM is measured by hand", key.clone());
                    continue;
                }
                match release
                    .artifacts
                    .iter()
                    .find_map(|a| held.get(&a.sha1.as_str().to_ascii_lowercase()))
                {
                    Some(dump) => {
                        let size = fs::metadata(dump)
                            .map_err(|e| format!("{key}: {}: {e}", dump.display()))?
                            .len();
                        state_rom(&mut release.hardware, size as u32)
                            .map_err(|e| format!("{key}: {e}"))?;
                        stats.sized += 1;
                        changed = true;
                    }
                    None => {
                        report.add("No local dump, so no measured ROM size", key.clone());
                        stats.skipped_no_dump += 1;
                    }
                }
            }
        }

        if changed && !dry_run {
            tree::write_game(data_root, &slug, &game).map_err(|e| e.to_string())?;
        }
    }
    Ok(stats)
}
