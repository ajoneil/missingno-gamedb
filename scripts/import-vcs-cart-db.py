#!/usr/bin/env python3
"""
Import the vcs_cart_db (Atari VCS / 2600 cartridge database) and generate
manifest.ron files under ../vcs/{slug}/.

Usage:
    python import-vcs-cart-db.py path/to/db.json

Groups roms by slugify(title), accumulating every rom's sha1 into one
manifest. Idempotent: re-running merges new hashes into existing manifests
without overwriting other fields.
"""

import json
import re
import sys
from pathlib import Path

VCS_DIR = Path(__file__).parent.parent / "vcs"


def slugify(name: str) -> str:
    s = re.sub(r"\s*\(.*?\)", "", name).strip()
    s = re.sub(r"[^a-z0-9]+", "-", s.lower()).strip("-")
    return re.sub(r"-+", "-", s)


def escape_ron_string(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


TV_FORMAT_MAP = {
    "NTSC": "Ntsc",
    "NTSC50": "Ntsc",
    "PAL": "Pal",
    "PAL60": "Pal",
    "SECAM": "Secam",
    "SECAM60": "Secam",
}


def map_tv_format(raw: str | None) -> str | None:
    if raw is None:
        return None
    return TV_FORMAT_MAP.get(raw.strip().upper())


def format_manifest(info: dict) -> str:
    lines = ["("]
    lines.append(f'    title: "{escape_ron_string(info["title"])}",')

    if info.get("publisher"):
        lines.append(f'    publisher: Some("{escape_ron_string(info["publisher"])}"),')

    if info.get("date"):
        lines.append(f'    date: Some("{escape_ron_string(info["date"])}"),')

    if info.get("tv_format"):
        lines.append(f'    tv_format: Some({info["tv_format"]}),')

    if info.get("cart_type"):
        lines.append(f'    cart_type: Some("{escape_ron_string(info["cart_type"])}"),')

    hash_strs = ", ".join(f'"{h}"' for h in sorted(info["hashes"]))
    lines.append(f"    hashes: [{hash_strs}],")
    lines.append("    source: None,")
    lines.append(")")
    return "\n".join(lines) + "\n"


def merge_hashes(manifest_path: Path, new_hashes: set[str]) -> bool:
    content = manifest_path.read_text()
    m = re.search(r"^(\s*)hashes: \[(.*?)\],?$", content, re.MULTILINE)
    if m is None:
        return False
    existing = set(re.findall(r'"([0-9a-f]{40})"', m.group(2)))
    merged = sorted(existing | new_hashes)
    if set(merged) == existing:
        return False
    hash_strs = ", ".join(f'"{h}"' for h in merged)
    line = f"{m.group(1)}hashes: [{hash_strs}],"
    manifest_path.write_text(content[:m.start()] + line + content[m.end():])
    return True


def process_db(db_path: str) -> dict[str, dict]:
    with open(db_path) as f:
        db = json.load(f)

    games: dict[str, dict] = {}
    for rom in db["roms"]:
        title = rom.get("title")
        sha1 = (rom.get("sha1") or "").lower()
        if not title or not sha1:
            continue
        slug = slugify(title)
        if not slug:
            continue

        year = rom.get("year")
        date = str(year) if isinstance(year, int) else None
        rep = {
            "title": title,
            "publisher": rom.get("publisher") or None,
            "date": date,
            "tv_format": map_tv_format(rom.get("tvFormat")),
            "cart_type": rom.get("cartType") or None,
        }

        if slug not in games:
            games[slug] = {**rep, "hashes": {sha1}, "_rep_title": title}
        else:
            g = games[slug]
            g["hashes"].add(sha1)
            # Representative = alphabetically-first title.
            if title < g["_rep_title"]:
                g.update(rep)
                g["_rep_title"] = title

    return games


def write_manifests(games: dict[str, dict]) -> tuple[int, int, int]:
    created = updated = unchanged = 0
    for slug, info in sorted(games.items()):
        game_dir = VCS_DIR / slug
        manifest_path = game_dir / "manifest.ron"
        if manifest_path.exists():
            if merge_hashes(manifest_path, info["hashes"]):
                updated += 1
            else:
                unchanged += 1
            continue
        game_dir.mkdir(parents=True, exist_ok=True)
        manifest_path.write_text(format_manifest(info))
        created += 1
    return created, updated, unchanged


def main():
    if len(sys.argv) < 2:
        print(f"Usage: {sys.argv[0]} <db.json>")
        sys.exit(1)
    db_path = sys.argv[1]
    games = process_db(db_path)
    print(f"vcs_cart_db unique slugs: {len(games)}")
    created, updated, unchanged = write_manifests(games)
    print(f"  Created: {created}, Updated: {updated}, Unchanged: {unchanged}")


if __name__ == "__main__":
    main()
