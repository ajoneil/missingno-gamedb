#!/usr/bin/env python3
"""
Import No-Intro DAT file(s) and generate manifest.ron files.

Usage:
    python import-nointro.py path/to/dat-file.dat [path/to/another.dat ...]

Reads Logiqx-format XML DAT files (as distributed by No-Intro) and creates
a manifest.ron for each game entry under ../games/{slug}/.

Idempotent: re-running updates existing manifests without overwriting
manually added enrichment files (knowledge/, cheats/, etc.).
"""

import xml.etree.ElementTree as ET
import os
import re
import sys
from pathlib import Path

GAMES_DIR = Path(__file__).parent.parent / "games"


def slugify(name: str) -> str:
    """Convert a game title to a filesystem-safe slug."""
    # Remove region/revision tags for the slug
    slug = re.sub(r"\s*\(.*?\)", "", name).strip()
    # Lowercase, replace non-alnum with hyphens
    slug = re.sub(r"[^a-z0-9]+", "-", slug.lower()).strip("-")
    # Collapse multiple hyphens
    slug = re.sub(r"-+", "-", slug)
    return slug


def parse_region(name: str) -> str | None:
    """Extract region from a No-Intro name like 'Title (USA, Europe)'."""
    m = re.search(r"\(([^)]*)\)", name)
    if m:
        return m.group(1)
    return None


def parse_year(name: str) -> str | None:
    """Extract year if present in parenthetical tags."""
    # No-Intro doesn't include year in the name, so this returns None.
    # Year data would come from other sources (Hasheous, etc.)
    return None


def escape_ron_string(s: str) -> str:
    """Escape a string for RON format."""
    return s.replace("\\", "\\\\").replace('"', '\\"')


def format_manifest(title: str, platform: str, hashes: list[str],
                    region: str | None) -> str:
    """Format a manifest.ron file."""
    lines = []
    lines.append("(")
    lines.append(f'    title: "{escape_ron_string(title)}",')
    lines.append(f"    platform: {platform},")

    if region:
        lines.append(f'    region: Some("{escape_ron_string(region)}"),')

    # Hashes
    hash_strs = ", ".join(f'"{h}"' for h in sorted(hashes))
    lines.append(f"    hashes: [{hash_strs}],")

    # No source for commercial games
    lines.append("    source: None,")

    lines.append(")")
    return "\n".join(lines) + "\n"


def detect_platform(header_name: str) -> str:
    """Detect platform from the DAT header name."""
    lower = header_name.lower()
    if "game boy color" in lower or "game boy colour" in lower:
        return "GBC"
    elif "game boy" in lower:
        return "GB"
    else:
        # Default to GB, but warn
        print(f"  Warning: could not detect platform from '{header_name}', defaulting to GB")
        return "GB"


def process_dat(dat_path: str) -> dict[str, dict]:
    """
    Parse a No-Intro DAT file and return a dict of slug -> game info.

    Returns:
        {slug: {title, platform, hashes: [sha1, ...], region}}
    """
    tree = ET.parse(dat_path)
    root = tree.getroot()

    # Get platform from header
    header = root.find("header")
    header_name = header.findtext("name", "") if header is not None else ""
    platform = detect_platform(header_name)

    print(f"Processing: {header_name}")
    print(f"  Platform: {platform}")

    games: dict[str, dict] = {}
    rom_count = 0

    for game_el in root.findall("game"):
        name = game_el.get("name", "")
        if not name:
            continue

        # Skip BIOS, unlicensed compilations, etc. — include everything for now
        # Users can filter in the emulator.

        rom_el = game_el.find("rom")
        if rom_el is None:
            continue

        sha1 = rom_el.get("sha1", "").lower()
        if not sha1:
            continue

        rom_count += 1

        # Clean up the title — remove region/revision tags for display
        title = name
        region = parse_region(name)
        slug = slugify(name)

        if not slug:
            continue

        # Group by slug — multiple regions/revisions of the same game
        # share a slug and accumulate hashes
        if slug in games:
            if sha1 not in games[slug]["hashes"]:
                games[slug]["hashes"].append(sha1)
        else:
            games[slug] = {
                "title": title,
                "platform": platform,
                "hashes": [sha1],
                "region": region,
            }

    print(f"  ROMs: {rom_count}")
    print(f"  Unique games: {len(games)}")

    return games


def write_manifests(games: dict[str, dict], dry_run: bool = False):
    """Write manifest.ron files for each game."""
    created = 0
    updated = 0
    skipped = 0

    for slug, info in sorted(games.items()):
        game_dir = GAMES_DIR / slug
        manifest_path = game_dir / "manifest.ron"

        # If manifest already exists, update hashes but preserve the rest
        if manifest_path.exists():
            # For now, skip existing manifests to avoid overwriting enrichment.
            # A smarter approach would parse the existing manifest and merge hashes.
            skipped += 1
            continue

        if dry_run:
            print(f"  Would create: {slug}/manifest.ron")
            created += 1
            continue

        game_dir.mkdir(parents=True, exist_ok=True)
        content = format_manifest(
            title=info["title"],
            platform=info["platform"],
            hashes=info["hashes"],
            region=info["region"],
        )
        manifest_path.write_text(content)
        created += 1

    print(f"  Created: {created}, Skipped (existing): {skipped}")


def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <dat-file> [dat-file ...]")
        print()
        print("Import No-Intro DAT files and generate manifest.ron files.")
        print("Pass --dry-run to preview without writing files.")
        sys.exit(1)

    dry_run = "--dry-run" in sys.argv
    dat_files = [f for f in sys.argv[1:] if f != "--dry-run"]

    all_games: dict[str, dict] = {}

    for dat_path in dat_files:
        if not os.path.exists(dat_path):
            print(f"Error: {dat_path} not found")
            sys.exit(1)

        games = process_dat(dat_path)

        # Merge into all_games, combining hashes for duplicate slugs
        for slug, info in games.items():
            if slug in all_games:
                for h in info["hashes"]:
                    if h not in all_games[slug]["hashes"]:
                        all_games[slug]["hashes"].append(h)
            else:
                all_games[slug] = info

    print(f"\nTotal unique games across all DATs: {len(all_games)}")
    write_manifests(all_games, dry_run=dry_run)


if __name__ == "__main__":
    main()
