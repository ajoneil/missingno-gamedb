#!/usr/bin/env python3
"""
Restructure the catalogue from flat games/{slug}/ into per-console trees
gb/{slug}/, gbc/{slug}/, vcs/{slug}/.

- Parses the No-Intro ClrMamePro DATs (GB + GBC), grouping by slug.
- Classifies each commercial title as gb or gbc using, in order:
    1. gameboy-headers (gbheaders.json) cgb-mode by slug,
    2. MAME software-list GBC compatibility tag by sha1,
    3. default gb.
- Moves each existing games/{slug}/ dir (commercial or homebrew) into its
  console tree, preserving every field/subdir; commercial manifests get their
  hashes merged with the DAT. New commercial titles get a fresh manifest.
- Routes homebrew by the gbdev database `platform` field, and re-imports new
  homebrew from a local gbdev clone.

This does NOT touch vcs/ (see import-vcs-cart-db.py) and does NOT commit.
"""

import json
import re
import shutil
import sys
from collections import Counter, defaultdict
from pathlib import Path

ROOT = Path(__file__).parent.parent
GAMES_DIR = ROOT / "games"
GB_DIR = ROOT / "gb"
GBC_DIR = ROOT / "gbc"
VCS_DIR = ROOT / "vcs"

RES = Path("/home/andrew/Projects/missingno/receipts/resources/vcs-gamedb-import")
DAT_GB = RES / "nointro-gb.dat"
DAT_GBC = RES / "nointro-gbc.dat"
GBHEADERS = RES / "gbheaders.json"
MAME_GB = RES / "mame-gameboy.xml"
MAME_GBC = RES / "mame-gbcolor.xml"
DB_JSON = RES / "db.json"
GBDEV_ENTRIES = Path(
    "/home/andrew/Projects/missingno/receipts/resources/gbdev-database/entries"
)

sys.path.insert(0, str(Path(__file__).parent))
import importlib.util

_spec = importlib.util.spec_from_file_location(
    "import_homebrew_hub", Path(__file__).parent / "import-homebrew-hub.py"
)
homebrew = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(homebrew)


def slugify(name: str) -> str:
    s = re.sub(r"\s*\(.*?\)", "", name).strip()
    s = re.sub(r"[^a-z0-9]+", "-", s.lower()).strip("-")
    return re.sub(r"-+", "-", s)


def parse_region(name: str) -> str | None:
    m = re.search(r"\(([^)]*)\)", name)
    return m.group(1) if m else None


def escape_ron_string(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


# ---------------------------------------------------------------------------
# No-Intro DAT (ClrMamePro text)
# ---------------------------------------------------------------------------

def parse_dat(path: Path) -> dict[str, dict]:
    """slug -> {names: set, hashes: set}"""
    text = path.read_text(errors="replace")
    games: dict[str, dict] = {}
    blocks = re.split(r"\ngame \(", text)
    for block in blocks[1:]:
        nm = re.search(r'name "([^"]*)"', block)
        if not nm:
            continue
        name = nm.group(1)
        slug = slugify(name)
        if not slug:
            continue
        sha1s = {h.lower() for h in re.findall(r"sha1 ([0-9A-Fa-f]{40})", block)}
        if not sha1s:
            continue
        g = games.setdefault(slug, {"names": set(), "hashes": set()})
        g["names"].add(name)
        g["hashes"] |= sha1s
    return games


def merge_dats() -> dict[str, dict]:
    combined: dict[str, dict] = {}
    for path in (DAT_GB, DAT_GBC):
        for slug, g in parse_dat(path).items():
            c = combined.setdefault(slug, {"names": set(), "hashes": set()})
            c["names"] |= g["names"]
            c["hashes"] |= g["hashes"]
    return combined


# ---------------------------------------------------------------------------
# gameboy-headers classification
# ---------------------------------------------------------------------------

def build_headers_map() -> dict[str, set]:
    """slug -> set of cgb-mode tokens ('cgb only' / 'dmg+cgb' / 'monochrome')"""
    data = json.load(open(GBHEADERS))
    m: dict[str, set] = defaultdict(set)
    for entry in data:
        fn = entry.get("filename", "")
        stem = re.sub(r"\.(gbc?|gb)$", "", fn, flags=re.IGNORECASE)
        slug = slugify(stem)
        if not slug:
            continue
        mode = entry.get("cgb mode")
        m[slug].add(mode if mode else "monochrome")
    return dict(m)


# ---------------------------------------------------------------------------
# MAME GBC-compatibility map
# ---------------------------------------------------------------------------

def build_mame_maps() -> tuple[set, set]:
    """(all_sha1_in_mame, gbc_tagged_sha1)"""
    import xml.etree.ElementTree as ET
    all_sha1: set[str] = set()
    gbc_sha1: set[str] = set()
    for path in (MAME_GB, MAME_GBC):
        root = ET.parse(path).getroot()
        for sw in root.findall("software"):
            has_gbc = any(
                sf.get("name") == "compatibility" and sf.get("value") == "GBC"
                for sf in sw.findall("sharedfeat")
            )
            for rom in sw.findall(".//rom"):
                sha1 = rom.get("sha1")
                if not sha1:
                    continue
                sha1 = sha1.lower()
                all_sha1.add(sha1)
                if has_gbc:
                    gbc_sha1.add(sha1)
    return all_sha1, gbc_sha1


# ---------------------------------------------------------------------------
# Existing games/ inventory
# ---------------------------------------------------------------------------

def read_manifest_hashes(manifest: Path) -> set[str]:
    if not manifest.exists():
        return set()
    m = re.search(r"hashes: \[(.*?)\]", manifest.read_text(), re.DOTALL)
    if not m:
        return set()
    return set(re.findall(r'"([0-9a-f]{40})"', m.group(1)))


def inventory_existing() -> tuple[dict[str, set], set[str]]:
    """Return (commercial slug->existing hashes, homebrew slug set)."""
    commercial: dict[str, set] = {}
    homebrew_slugs: set[str] = set()
    for d in sorted(GAMES_DIR.iterdir()):
        if not d.is_dir():
            continue
        manifest = d / "manifest.ron"
        if manifest.exists() and "HomebrewHub" in manifest.read_text():
            homebrew_slugs.add(d.name)
        else:
            commercial[d.name] = read_manifest_hashes(manifest)
    return commercial, homebrew_slugs


# ---------------------------------------------------------------------------
# Classification
# ---------------------------------------------------------------------------

def classify(slug: str, hashes: set[str], headers: dict[str, set],
             gbc_sha1: set[str]) -> tuple[str, str]:
    """Return (platform, rule)."""
    if slug in headers:
        modes = headers[slug]
        platform = "gbc" if "cgb only" in modes else "gb"
        return platform, "headers"
    if hashes & gbc_sha1:
        return "gbc", "mame"
    return "gb", "default"


def mame_class(hashes: set[str], all_sha1: set[str], gbc_sha1: set[str]):
    if not (hashes & all_sha1):
        return None
    return "gbc" if (hashes & gbc_sha1) else "gb"


# ---------------------------------------------------------------------------
# Manifest writing
# ---------------------------------------------------------------------------

def format_commercial(title: str, region: str | None, hashes: set[str]) -> str:
    lines = ["(", f'    title: "{escape_ron_string(title)}",']
    if region:
        lines.append(f'    region: Some("{escape_ron_string(region)}"),')
    hash_strs = ", ".join(f'"{h}"' for h in sorted(hashes))
    lines.append(f"    hashes: [{hash_strs}],")
    lines.append("    source: None,")
    lines.append(")")
    return "\n".join(lines) + "\n"


def update_existing_manifest(manifest: Path, new_hashes: set[str],
                             title: str | None, region: str | None):
    """Merge hashes, add title/region only if missing. Preserve all else."""
    content = manifest.read_text()

    m = re.search(r"^(\s*)hashes: \[(.*?)\],?$", content, re.MULTILINE)
    if m:
        existing = set(re.findall(r'"([0-9a-f]{40})"', m.group(2)))
        merged = sorted(existing | new_hashes)
        if set(merged) != existing:
            hash_strs = ", ".join(f'"{h}"' for h in merged)
            line = f"{m.group(1)}hashes: [{hash_strs}],"
            content = content[:m.start()] + line + content[m.end():]

    if title and not re.search(r"^\s*title:", content, re.MULTILINE):
        content = content.replace(
            "(\n", f'(\n    title: "{escape_ron_string(title)}",\n', 1)
    if region and not re.search(r"^\s*region:", content, re.MULTILINE):
        content = re.sub(
            r'(title: "[^"]*",\n)',
            rf'\1    region: Some("{escape_ron_string(region)}"),\n',
            content, count=1)

    manifest.write_text(content)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    for p in (DAT_GB, DAT_GBC, GBHEADERS, MAME_GB, MAME_GBC):
        if not p.exists():
            print(f"Missing source: {p}")
            sys.exit(1)

    print("Parsing No-Intro DATs...")
    dat = merge_dats()
    print(f"  DAT slugs: {len(dat)}")

    headers = build_headers_map()
    all_mame, gbc_mame = build_mame_maps()
    print(f"  header slugs: {len(headers)}  mame sha1: {len(all_mame)} "
          f"(gbc-tagged {len(gbc_mame)})")

    existing_comm, existing_hb = inventory_existing()
    old_total = len(existing_comm) + len(existing_hb)
    print(f"  existing commercial dirs: {len(existing_comm)}  "
          f"homebrew dirs: {len(existing_hb)}  (old total {old_total})")

    GB_DIR.mkdir(exist_ok=True)
    GBC_DIR.mkdir(exist_ok=True)

    report = {
        "rule": Counter(),
        "platform": Counter(),
        "new_commercial": 0,
        "preexisting_commercial_in_dat": 0,
        "preexisting_commercial_not_in_dat": 0,
        "moved_commercial": 0,
        "hb_moved": Counter(),
        "hb_new": Counter(),
        "disagreements": [],
        "hb_collisions": [],
        "hb_commercial_skips": [],
        "old_total_expected": old_total,
    }

    # ---- commercial slugs: DAT ∪ existing commercial dirs ----
    all_comm_slugs = set(dat) | set(existing_comm)

    for slug in sorted(all_comm_slugs):
        dat_hashes = dat.get(slug, {}).get("hashes", set())
        exist_hashes = existing_comm.get(slug, set())
        hashes = dat_hashes | exist_hashes

        platform, rule = classify(slug, hashes, headers, gbc_mame)
        dest_root = GBC_DIR if platform == "gbc" else GB_DIR

        # Disagreement detection: headers vs MAME both have an opinion.
        if slug in headers:
            mc = mame_class(hashes, all_mame, gbc_mame)
            hc = "gbc" if "cgb only" in headers[slug] else "gb"
            if mc is not None and mc != hc:
                report["disagreements"].append((slug, hc, mc))

        # Skip DAT-driven commercial creation if the slug is an existing
        # homebrew dir (it is relocated by the homebrew pass instead).
        if slug in existing_hb:
            report["hb_collisions"].append(slug)
            continue

        report["rule"][rule] += 1
        report["platform"][platform] += 1

        old_dir = GAMES_DIR / slug
        if slug in existing_comm and old_dir.exists():
            # Representative title/region for gap-filling only.
            names = dat.get(slug, {}).get("names")
            title = min(names) if names else None
            region = parse_region(title) if title else None
            shutil.move(str(old_dir), str(dest_root / slug))
            update_existing_manifest(
                dest_root / slug / "manifest.ron", dat_hashes, title, region)
            report["moved_commercial"] += 1
            if slug in dat:
                report["preexisting_commercial_in_dat"] += 1
            else:
                report["preexisting_commercial_not_in_dat"] += 1
        else:
            # New commercial from DAT.
            names = dat[slug]["names"]
            title = min(names)
            region = parse_region(title)
            (dest_root / slug).mkdir(parents=True, exist_ok=True)
            (dest_root / slug / "manifest.ron").write_text(
                format_commercial(title, region, hashes))
            report["new_commercial"] += 1

    # ---- homebrew ----
    fresh_hb = build_fresh_homebrew()
    print(f"  fresh gbdev homebrew (GB/GBC playable): {len(fresh_hb)}")

    # Relocate existing homebrew dirs, routed by fresh platform (default gb).
    existing_hb_platform = {}
    for slug in sorted(existing_hb):
        platform = fresh_hb.get(slug, {}).get("platform", "gb")
        dest_root = GBC_DIR if platform == "gbc" else GB_DIR
        old_dir = GAMES_DIR / slug
        if old_dir.exists():
            shutil.move(str(old_dir), str(dest_root / slug))
            existing_hb_platform[slug] = platform
            report["hb_moved"][platform] += 1

    # Write/refresh homebrew manifests from fresh gbdev data. Never clobber a
    # commercial manifest that owns the same slug (commercial precedence, as
    # the original homebrew importer applied).
    for slug, info in sorted(fresh_hb.items()):
        dest_root = GBC_DIR if info["platform"] == "gbc" else GB_DIR
        game_dir = dest_root / slug
        manifest = game_dir / "manifest.ron"
        if manifest.exists() and "HomebrewHub" not in manifest.read_text():
            report["hb_commercial_skips"].append(slug)
            continue
        is_new = slug not in existing_hb
        game_dir.mkdir(parents=True, exist_ok=True)
        manifest.write_text(info["content"])
        if is_new:
            report["hb_new"][info["platform"]] += 1

    # ---- verify every old dir was relocated, then delete games/ ----
    leftovers = [d.name for d in GAMES_DIR.iterdir() if d.is_dir()]
    report["leftovers"] = leftovers
    moved_total = (report["moved_commercial"]
                   + sum(report["hb_moved"].values()))
    report["old_total"] = report["old_total_expected"]
    assert not leftovers, f"Unrelocated dirs remain in games/: {leftovers[:20]}"
    assert moved_total == report["old_total_expected"], (
        f"moved {moved_total} != old {report['old_total_expected']}")
    shutil.rmtree(GAMES_DIR)

    return report


def build_fresh_homebrew() -> dict[str, dict]:
    """slug -> {platform: 'gb'|'gbc', content: ron} from local gbdev clone."""
    result: dict[str, dict] = {}
    if not GBDEV_ENTRIES.is_dir():
        print(f"  gbdev entries not found at {GBDEV_ENTRIES}; skipping homebrew "
              f"refresh (existing homebrew defaults to gb/).")
        return result
    for game_dir in sorted(GBDEV_ENTRIES.iterdir()):
        gj = game_dir / "game.json"
        if not gj.exists():
            continue
        try:
            entry = json.load(open(gj))
        except (json.JSONDecodeError, OSError):
            continue
        platform = entry.get("platform")
        if platform not in ("GB", "GBC"):
            continue
        content = homebrew.format_manifest(entry)
        if content is None:
            continue
        slug = entry["slug"]
        result[slug] = {"platform": platform.lower(), "content": content}
    return result


if __name__ == "__main__":
    report = main()
    import pickle
    (RES / "_report.pkl").write_bytes(pickle.dumps(report))
    print("\nSummary:")
    print(f"  rule breakdown: {dict(report['rule'])}")
    print(f"  platform: {dict(report['platform'])}")
    print(f"  new commercial: {report['new_commercial']}")
    print(f"  moved commercial: {report['moved_commercial']}")
    print(f"  homebrew moved: {dict(report['hb_moved'])}")
    print(f"  homebrew new: {dict(report['hb_new'])}")
    print(f"  disagreements: {len(report['disagreements'])}")
    print(f"  homebrew/DAT slug collisions: {len(report['hb_collisions'])}")
