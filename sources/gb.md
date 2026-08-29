# Game Boy and Game Boy Color sources

Per-tree catalogue for `data/gb` and `data/gbc`. The rules that apply everywhere
are in [`README.md`](README.md).

## Catalogues

| Source | Good for | Notes |
|--------|----------|-------|
| [gbdev](https://gbdev.io) database and its Homebrew Hub | homebrew authorship, licence, canonical cover art | Primary sources beat aggregators: prefer the author's own repo or site to a catalogue entry. |
| The project's own repo or site | everything, for homebrew | GitHub raw URLs are the canonical host for cover art and downloads. |

## Hardware facts

The curator auto-stages what a fetched or booted cartridge header states — SGB
and CGB enhancement, and the board with the ROM and RAM chips it names — filling
unknowns only, and reports header-vs-db conflicts in the verify status.

Override `cart_type` via `update_game` when the truth differs from the header:
**unlicensed carts lie**. A stated board replaces the header's word whole, parts
and all, so state every part the cart has rather than the one that differs.
