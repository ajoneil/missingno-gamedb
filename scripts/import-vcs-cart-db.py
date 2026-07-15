#!/usr/bin/env python3
"""
Import the vcs_cart_db (Atari VCS / 2600 cartridge database) and generate
manifest.ron files under ../vcs/{slug}/.

Usage:
    python import-vcs-cart-db.py path/to/db.json [--regenerate]

Groups roms by title AND by the properties that belong to the cartridge rather
than to the game: its broadcast standard and its board. Two dumps sharing a
title but differing in either are different cartridges — a PAL Pitfall II is
not a USA one — so they get an entry each, suffixed by whichever of the two
actually differs. Collapsing them would make the entry's tv_format a coin toss
between the variants, and every ROM that lost the toss would be mislabelled.

A title whose dumps agree keeps the bare slug.

Without --regenerate this only merges new hashes into existing manifests,
leaving other fields alone. With it, every VCS manifest is rewritten from the
database and manifests no longer backed by it are removed.
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


def variant_suffix(value: str | None) -> str:
    return re.sub(r"[^a-z0-9]+", "", value.lower()) if value else "unknown"


def process_db(db_path: str) -> dict[str, dict]:
    with open(db_path) as f:
        db = json.load(f)

    # Key by the cartridge, not just the game: same title, different standard
    # or board = a different cartridge.
    variants: dict[tuple[str, str | None, str | None], dict] = {}
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
        tv_format = map_tv_format(rom.get("tvFormat"))
        cart_type = rom.get("cartType") or None
        rep = {
            "title": title,
            "publisher": rom.get("publisher") or None,
            "date": date,
            "tv_format": tv_format,
            "cart_type": cart_type,
        }

        key = (slug, tv_format, cart_type)
        if key not in variants:
            variants[key] = {**rep, "slug": slug, "hashes": {sha1}, "_rep_title": title}
        else:
            v = variants[key]
            v["hashes"].add(sha1)
            # Representative = alphabetically-first title.
            if title < v["_rep_title"]:
                v.update(rep)
                v["_rep_title"] = title

    return name_variants(variants)


def name_variants(variants: dict[tuple[str, str | None, str | None], dict]) -> dict[str, dict]:
    """Give each variant a directory. A title whose dumps agree keeps the bare
    slug; where they disagree, every variant is suffixed by the fields that
    differ, so no variant silently claims to be the game."""
    by_slug: dict[str, list[dict]] = {}
    for (slug, _, _), v in variants.items():
        by_slug.setdefault(slug, []).append(v)

    games: dict[str, dict] = {}
    for slug, group in by_slug.items():
        if len(group) == 1:
            games[slug] = group[0]
            continue
        differs_tv = len({v["tv_format"] for v in group}) > 1
        differs_cart = len({v["cart_type"] for v in group}) > 1
        for v in group:
            parts = [slug]
            if differs_tv:
                parts.append(variant_suffix(v["tv_format"]))
            if differs_cart:
                parts.append(variant_suffix(v["cart_type"]))
            games["-".join(parts)] = v
    return games


def write_manifests(games: dict[str, dict], regenerate: bool) -> tuple[int, int, int]:
    created = updated = unchanged = 0
    for slug, info in sorted(games.items()):
        game_dir = VCS_DIR / slug
        manifest_path = game_dir / "manifest.ron"
        if manifest_path.exists() and not regenerate:
            if merge_hashes(manifest_path, info["hashes"]):
                updated += 1
            else:
                unchanged += 1
            continue
        wanted = format_manifest(info)
        if manifest_path.exists():
            if manifest_path.read_text() == wanted:
                unchanged += 1
                continue
            manifest_path.write_text(wanted)
            updated += 1
            continue
        game_dir.mkdir(parents=True, exist_ok=True)
        manifest_path.write_text(wanted)
        created += 1
    return created, updated, unchanged


def prune(games: dict[str, dict]) -> int:
    """Drop manifests the database no longer backs — a slug that split into
    variants leaves its old collapsed entry behind."""
    removed = 0
    for game_dir in sorted(VCS_DIR.iterdir()):
        if not game_dir.is_dir() or game_dir.name in games:
            continue
        leftovers = [p for p in game_dir.iterdir() if p.name != "manifest.ron"]
        if leftovers:
            print(f"  kept {game_dir.name}: has data beyond its manifest")
            continue
        (game_dir / "manifest.ron").unlink(missing_ok=True)
        game_dir.rmdir()
        removed += 1
    return removed


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    regenerate = "--regenerate" in sys.argv[1:]
    if not args:
        print(f"Usage: {sys.argv[0]} <db.json> [--regenerate]")
        sys.exit(1)
    games = process_db(args[0])
    print(f"vcs_cart_db cartridge variants: {len(games)}")
    created, updated, unchanged = write_manifests(games, regenerate)
    print(f"  Created: {created}, Updated: {updated}, Unchanged: {unchanged}")
    if regenerate:
        print(f"  Pruned: {prune(games)}")


if __name__ == "__main__":
    main()
