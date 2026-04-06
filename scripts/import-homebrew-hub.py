#!/usr/bin/env python3
"""
Import gbdev Homebrew Hub database and generate manifest.ron files.

Usage:
    python import-homebrew-hub.py path/to/database/entries/

Reads the gbdev database repo (https://github.com/gbdev/database) and
creates a manifest.ron for each GB game entry under ../games/{slug}/.

The database repo should be cloned locally first:
    git clone https://github.com/gbdev/database.git

Then point this script at the entries/ directory:
    python import-homebrew-hub.py path/to/database/entries/

Idempotent: re-running updates existing manifests without overwriting
manually added enrichment files (knowledge/, cheats/, etc.).
"""

import json
import os
import sys
from pathlib import Path

GAMES_DIR = Path(__file__).parent.parent / "games"


def escape_ron_string(s: str) -> str:
    """Escape a string for RON format."""
    return s.replace("\\", "\\\\").replace('"', '\\"')


def as_string(val) -> str | None:
    """Coerce a value to a string, handling lists by joining."""
    if val is None:
        return None
    if isinstance(val, list):
        return ", ".join(str(v) for v in val) if val else None
    return str(val)


def format_ron_list(items: list[str], indent: str = "    ") -> str:
    """Format a list of strings as a RON array."""
    if not items:
        return "[]"
    if len(items) == 1:
        return f'["{escape_ron_string(items[0])}"]'
    inner = ", ".join(f'"{escape_ron_string(i)}"' for i in items)
    return f"[{inner}]"


def format_manifest(entry: dict) -> str | None:
    """Format a manifest.ron file from a gbdev game.json entry."""
    title = entry.get("title")
    platform = entry.get("platform")
    slug = entry.get("slug")

    if not title or not slug:
        return None

    # Only GB games
    if platform != "GB":
        return None

    # Need at least one playable file
    files = entry.get("files", [])
    playable = [f for f in files if f.get("playable")]
    if not playable:
        return None

    # Pick the default or first playable file
    rom_file = next((f for f in playable if f.get("default")), playable[0])
    filename = rom_file.get("filename", "")

    # Strip subdirectory from filename (some entries have "files/foo.gb")
    filename = os.path.basename(filename)

    lines = []
    lines.append("(")
    lines.append(f'    title: "{escape_ron_string(title)}",')
    lines.append(f"    platform: {platform},")

    developer = as_string(entry.get("developer"))
    if developer:
        lines.append(f'    developer: Some("{escape_ron_string(developer)}"),')

    description = as_string(entry.get("description"))
    if description:
        desc = description.replace("\n", " ").strip()
        lines.append(f'    description: Some("{escape_ron_string(desc)}"),')

    lines.append("    hashes: [],")

    # Source: Homebrew Hub
    lines.append(f'    source: Some(HomebrewHub(slug: "{escape_ron_string(slug)}", filename: "{escape_ron_string(filename)}")),')

    license_str = as_string(entry.get("license"))
    if license_str:
        lines.append(f'    license: Some("{escape_ron_string(license_str)}"),')

    tags = entry.get("tags", [])
    if isinstance(tags, list) and tags:
        lines.append(f"    tags: {format_ron_list(tags)},")

    # Collect links
    links = []
    game_website = entry.get("gameWebsite") or entry.get("website")
    if isinstance(game_website, str) and game_website:
        links.append(("Website", game_website, "Wiki"))
    repo = entry.get("repository")
    if isinstance(repo, str) and repo:
        links.append(("Source Code", repo, "Source"))

    if links:
        lines.append("    links: [")
        for name, url, link_type in links:
            lines.append(f'        (name: "{escape_ron_string(name)}", url: "{escape_ron_string(url)}", link_type: {link_type}),')
        lines.append("    ],")

    lines.append(")")
    return "\n".join(lines) + "\n"


def process_entries_dir(entries_dir: str) -> dict[str, str]:
    """
    Read all game.json files from the gbdev database entries directory.

    Returns:
        {slug: manifest_ron_content}
    """
    entries_path = Path(entries_dir)
    if not entries_path.is_dir():
        print(f"Error: {entries_dir} is not a directory")
        sys.exit(1)

    manifests = {}
    skipped_platform = 0
    skipped_no_playable = 0
    skipped_parse_error = 0

    for game_dir in sorted(entries_path.iterdir()):
        game_json = game_dir / "game.json"
        if not game_json.exists():
            continue

        try:
            with open(game_json) as f:
                entry = json.load(f)
        except (json.JSONDecodeError, OSError) as e:
            print(f"  Warning: failed to read {game_json}: {e}")
            skipped_parse_error += 1
            continue

        platform = entry.get("platform")
        if platform != "GB":
            skipped_platform += 1
            continue

        manifest = format_manifest(entry)
        if manifest is None:
            skipped_no_playable += 1
            continue

        slug = entry["slug"]
        manifests[slug] = manifest

    print(f"  GB entries with playable ROMs: {len(manifests)}")
    print(f"  Skipped (other platform): {skipped_platform}")
    print(f"  Skipped (no playable file): {skipped_no_playable}")
    if skipped_parse_error:
        print(f"  Skipped (parse error): {skipped_parse_error}")

    return manifests


def write_manifests(manifests: dict[str, str], dry_run: bool = False):
    """Write manifest.ron files for each game."""
    created = 0
    updated = 0
    skipped = 0

    for slug, content in sorted(manifests.items()):
        game_dir = GAMES_DIR / slug
        manifest_path = game_dir / "manifest.ron"

        if manifest_path.exists():
            # Check if it's a homebrew manifest (has HomebrewHub source)
            # If so, update it. If it's a commercial manifest, skip.
            existing = manifest_path.read_text()
            if "HomebrewHub" in existing:
                # Update — overwrite with fresh data
                if not dry_run:
                    manifest_path.write_text(content)
                updated += 1
            else:
                # Commercial game with same slug — don't overwrite
                skipped += 1
            continue

        if dry_run:
            created += 1
            continue

        game_dir.mkdir(parents=True, exist_ok=True)
        manifest_path.write_text(content)
        created += 1

    print(f"  Created: {created}, Updated: {updated}, Skipped (commercial): {skipped}")


def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <path/to/database/entries/>")
        print()
        print("Import gbdev Homebrew Hub database and generate manifest.ron files.")
        print()
        print("Clone the database first:")
        print("    git clone https://github.com/gbdev/database.git")
        print()
        print("Then run:")
        print("    python import-homebrew-hub.py path/to/database/entries/")
        print()
        print("Pass --dry-run to preview without writing files.")
        sys.exit(1)

    dry_run = "--dry-run" in sys.argv
    entries_dir = [f for f in sys.argv[1:] if f != "--dry-run"][0]

    print(f"Reading gbdev database from: {entries_dir}")
    manifests = process_entries_dir(entries_dir)

    print(f"\nWriting manifests:")
    write_manifests(manifests, dry_run=dry_run)


if __name__ == "__main__":
    main()
