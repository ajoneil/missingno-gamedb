# Atari VCS sources

Per-tree catalogue for `data/vcs`. The rules that apply everywhere — never
construct a URL, the ROM-hosting link ban, dump identity, titles, publishers,
cover art, licensing — are in [`README.md`](README.md).

## Catalogues

| Source | Good for | Notes |
|--------|----------|-------|
| **Atarimania** — read-only, **never linked** (banned; see README) | publisher, **country**, year, model/reference number, **alternate titles** | The first stop once a dump is identified, and the only source covering the obscure regional reissues (CCE, Genus, Dynacom, Funvision) encyclopaedias omit. Its alternate-title list is how a game's other skins are found. Page ids are numeric and listing-page naming is irregular, so reach a game page only via a web search or the site's own search. robots.txt is empty, so ordinary reading is permitted. Facts from here go into the manifest with no `links` entry. |
| [Atari Compendium manual archive](https://www.ataricompendium.com/archives/manuals/vcs/vcs_manuals.html) | scanned game manuals (`Manual` link); occasional design documents for unreleased titles | ~1000 static PDFs hosted on-site. **Get the exact filename from the index page — never guess a slug**; filenames are unpredictable (`3dtictactoe.pdf`, not `3-d_tic-tac-toe.pdf`). Sears-branded scans are separate files (`…-sears.pdf`). Vetted: printed matter only (ads, articles, books, comics, interviews, magazines, manuals, maps, newsletters, reviews) — no ROMs section. |
| [AtariProtos](https://www.atariprotos.com) | **prototypes and unreleased carts**: designer, publisher, why it was cancelled, how the dump surfaced, and gameplay of builds no manual covers | Vetted: documentation only, no ROM downloads in any section. Reach a game page via search; pages sit under `/2600/software/<name>/`. |
| AtariAge | gameplay descriptions + **manual scans** for obscure carts (Sancho, Zellers, Goliath) encyclopaedias omit | HTML pages Cloudflare-challenge WebFetch, but a plain `curl -A 'Mozilla/5.0'` reaches `software_page.php`, `manual_page.php` and the scan images fine — not blocked, just needs a normal client. A scanned manual IS a readable source: get the real full-page URL off `manual_page.php` (`src="/2600/manuals/<Name>/m_<Name>_N.jpg"` — read it, don't guess), `curl` the JPG, open it with the Read tool. Don't call a description unsourceable until you have read the manual scan this way. Wayback is the fallback for a dead page, but WebFetch can't reach web.archive.org — `curl` it. |
| Stella's properties database | board/cart type when a playtest boots wrong | The board drives the emulator, so a garbled playtest makes `cart_type` the first suspect — but a game rendering no stable frame on *any* valid board is a software problem, not a board mismatch. |

## Publishers

**Fox Video Games, Inc.** is the publisher for the whole Fox 2600 line, 1982 and
1983 alike — checked against fourteen manual copyright lines (Saratoga, then
Santa Clara from 1983). "20th Century Fox" appears on those manuals only as *"A
20th Century Fox Film Corporation Production"*, crediting the parent film studio
on a licensed property. An import had read that as the games publisher and
written "20th Century Fox Video Games" across 30 releases.

Renamings and brand splits that are **real**, so two spellings in the tree are
correct rather than a normalisation bug:

- **Computer Magic → CommaVid.** MagiCard's blue label kept the earlier name;
  Mines of Minos and Cakewalk carry CommaVid.
- **Spectravideo → Spectravision** on US 2600 carts. The tree holds both, and
  each is right for its own release.
- **Sancho / Tang's Electronic Co.** is one company; Atarimania files it under
  the combined heading.
- **Mattel Electronics vs M Network.** M Network was the brand on many Mattel
  2600 carts, but not all — Masters of the Universe's box, manual colophon and
  signature name all read Mattel Electronics.

## Regional and title conventions

- `~` in an entry title pairs an Atari name with its **Sears** rebadge
  (`Miniature Golf ~ Arcade Golf`). The ROM is identical, so a dump usually
  cannot tell you which box it came from.
- German mail-order and rebadge labels retitle heavily: Quelle, Video Gems,
  Goliath, Videospielkassette. Atarimania's alternate-title list is the way to
  find them. Watch for a common noun standing in as a publisher —
  *Videospielkassette* is simply German for "video game cartridge".
- Brazilian releases are **PAL-M**: PAL colour on NTSC timing. Never file one as
  `Pal`.

## Determining a dump's TV standard

The core does not auto-detect — the gamedb value drives the emulator, so a wrong
one shows as a wrong picture. Boot the dump headless and read the frame height:

```
missingno-debugger <rom> [--cart-type F0] --port 3401
curl -X POST localhost:3401/run && sleep 2 && curl -X POST localhost:3401/pause
curl -s localhost:3401/frame/raw     # → "height"
```

| height | standard |
|--------|----------|
| 228 | NTSC timing |
| 274 | PAL timing |

The region the emulator is set to does not change this — the ROM drives the
raster, and the region only sets the master clock. So a Brazilian dump measuring
228 is `PalM`: PAL colour on NTSC timing.

**A bankswitched cart needs `--cart-type`.** VCS carts carry no header, so an
image the core cannot size-detect fails to construct at all; name the board and
it loads.

## Recurring import defects

1. **One release holding every dump of a game**, stamped with whichever publisher
   sorted first, so the original ends up filed under a reissue's publisher.
2. **Padded dumps that fabricated releases.** A 4K game padded to 8K or 16K
   fingerprints as the wrong board and invents a release that never shipped.
   `move_artifact` it onto the dump it copies and `label_artifact` a defect;
   the phantom release prunes itself.
3. **Hacks filed by their title, not their base.** A hack names itself — check
   the signature entry for the base. ROM size is a decisive check: a 4K dump
   cannot hack an 8K game.
4. **Entries split by slug suffix** (`-ntsc`, `-pal`, `-f0`, `-a`) that
   `find_duplicates` will not surface, because their titles normalise
   differently or not at all. List the tree for the game's slug prefix.
