# ColecoVision sources

Per-tree catalogue for `data/colecovision`. The rules that apply everywhere —
never construct a URL, the ROM-hosting link ban, dump identity, titles,
publishers, cover art, licensing — are in [`README.md`](README.md).

The tree covers the ColecoVision cartridge library, including the unlicensed
releases the No-Intro DAT carries. Every site a manifest links goes through the
vetting in README.md before its first link.

## Catalogues

| Source | Good for | Notes |
|--------|----------|-------|
| **No-Intro** "Coleco - ColecoVision" DAT | dump identity, clone families, region/revision naming | The base import's source. This machine's IP is banned from datomatic; use the cached copy under the emulator repo's resources. |
| **MAME software list** (`hash/coleco.xml`) | year, publisher, developer, serial | A curation source, not a hash source: most entries split the cartridge into its chip images, so their SHA-1s rarely name a whole dump. Cross-check any release-level fact against the dump it describes. |
| **gamesdatabase.org** — **link freely** | manuals (direct PDFs) | First-choice `Manual` link. Take the PDF URL off the game page (`/game/coleco_vision/<name>`); the filename is not predictable and search-engine copies of it 404. Wherever its manual is linked, link the game page too (`Games Database`, `Community`). Where search finds no game page, the index `all_game_system_manuals_for-coleco_vision` lists every title with a manual; its rows are ASP.NET postbacks, and submitting one as a browser click would lands on the game page. Some PDFs filed here are another platform's edition (Spinnaker's cover Atari and Commodore 64): read the setup page before linking one as this port's manual. Its download rows are manuals and music only. |
| **ColecoVision Addict** (cvaddict.com) — **read only, never link** | manuals, box/label scans, publisher | Every game page carries a ROM download (`/roms/…`), so it is banned from `links`. Its manual PDFs are readable where gamesdatabase.org lacks one. |

## Cartridge boards

The tree names no boards: a plain cartridge is all the core builds. Titles that
need the Super Game Module or a MegaCart bank-switching board get an
`EmulationIncompatibility` flag when met; don't invent a board.

## TV standard

`tv_format` is recorded explicitly on every release; absent means unstated,
never a default. The import derives it from the release's markets — any NTSC
market (USA, Canada, Japan, Taiwan, Korea) makes it `Ntsc`, a Brazil-only
release is `PalM`, any other market set is `Pal`. Correct it per release where
a source shows the derivation wrong.

Brazil's machines presented PAL-M: PAL colour on NTSC timing, the standard the
software's home market saw.
