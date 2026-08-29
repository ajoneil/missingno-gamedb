# Game Boy and Game Boy Color sources

Per-tree catalogue for `data/gb` and `data/gbc`. The rules that apply everywhere
— never construct a URL, the ROM-hosting link ban, dump identity, titles,
publishers, cover art, licensing — are in [`README.md`](README.md).

## Catalogues

| Source | Good for | Notes |
|--------|----------|-------|
| [Games Database](https://www.gamesdatabase.org) — **link freely** | **game manuals** (direct PDFs), **box, cart and title-screen scans**, publisher, developer, year, category | The first stop for the commercial library. Vetted for the SG-1000 tree already — `robots.txt` allows all and it hosts no ROMs. Reaching anything on it takes the site's own search; see below. |
| [gbdev](https://gbdev.io) database and its Homebrew Hub | homebrew authorship, licence, canonical cover art | Primary sources beat aggregators: prefer the author's own repo or site to a catalogue entry. |
| The project's own repo or site | everything, for homebrew | GitHub raw URLs are the canonical host for cover art and downloads. |
| MobyGames — **agents cannot read it; unvetted** | — | `robots.txt` disallows ClaudeBot outright. No facts and no links until a human vets it. |
| Hidden Palace — **ask before reading** | prototypes | Its `robots.txt` carries `Content-Signal: ai-input=no`. Storing a link is not "ai-input"; an agent reading its pages for facts is what it declines. |

## Games Database

It is an ASP.NET application: the search and every result row are postbacks
carrying the page's viewstate, so neither has a URL to request. Submitting the
search form from the site root lands on a `list.aspx` results URL; following a
result row reaches the game page, and that page is the only place a current
media URL can be read.

**Never take one of its URLs from a web search.** Its media filenames are
indexed stale, and a stale one returns the site's own 404 page — which reads as
an absent manual rather than a wrong URL. Deriving a filename from a result row
fails the same way.

The results table gives system, publisher, developer, category and year per row,
which is how the right platform is picked out of a title shared across many. **A
game page also links other systems' media**, so check the system in the media
path before staging anything from it. The page is staged as a `Community` link:
`Wiki` is for a wiki.

## Manuals

A manual documents *this* cart where an encyclopaedia article documents a
multi-platform game as a whole, so it is the best gameplay source this tree has.
Games Database links the PDF from the game page; download it and read it as page
images. Coverage is roughly a third of each library, so a game having none is
ordinary and not worth a second search. Record the language on the link.

## Cover art

README.md's order stands — Hasheous, then libretro-thumbnails — with Games
Database as the fallback those two do not cover. Its scans show the whole box
including the platform banner, but **every one is watermarked** with the site's
domain, so it loses to a clean scan of the same art at any resolution. The
thumbnail on a game page is not the image to stage: follow the artwork page it
links and take the full-size URL from there.

## Sachen multicarts

They all arrive titled just "4 in 1", so the games each one holds are the only
thing telling them apart, joined into the title with `+`.

**Take those names off the cart, not off a catalogue.** The catalogues agree
with each other and are wrong about a name on nearly every volume, and about the
slot order. The menu table is plain ASCII in the ROM, so the names are readable
without booting — which matters, because most of these carts cannot reach their
own menu. Search the dump for a word from the catalogue's guess and dump around
the hit: the table is either one padded field per name followed by a genre tag,
or a two-column grid splitting each name across two rows, and it sometimes ends
with a copyright line that is the only date any source gives.

Record what the cart shows, misspellings included — the font renders D as O, and
that is the cart's own text rather than a decoding slip. The exception is a name
the layout truncated: the fields are fixed width, so an abbreviation that exactly
fills one is the menu shortening a longer name, the way a No-Intro filename
substitutes a dash for a colon.

**Their headers are scrambled and state nothing usable**, reading as an ordinary
mapper that is not the cart. State `SachenMmc1`, with the dump's own length as
its `rom`. Descrambling is what identifies the board: under

    address & ~0x53 | address>>6 & 0x01 | address>>3 & 0x02
                    | address<<3 & 0x10 | address<<6 & 0x40

a Sachen cart's Nintendo logo checksums to 5542 or 7484 at `0x184`, where an
ordinary cart reads 5446 unscrambled at `0x104`.

**The emulator has no Sachen mapper.** Only the small volumes play, the MBC1 the
image falls back to spanning them entirely; the larger ones need an outer-bank
register that is not modelled and fail in whatever way their reset bank
dictates. Each earns an `EmulationIncompatibility` flag naming only its symptom.

## Hardware facts

The curator auto-stages what a fetched or booted cartridge header states — SGB
and CGB enhancement, and the board with the ROM and RAM chips it names — filling
unknowns only, and reports header-vs-db conflicts in the verify status.

Override `cart_type` via `update_game` when the truth differs from the header:
**unlicensed carts lie**. A stated board replaces the header's word whole, parts
and all, so state every part the cart has rather than the one that differs.

A box scan is a second witness to what the header says: the Super Game Boy and
Game Boy Color banners are printed on the packaging, so a `features` list can be
checked against the art rather than trusted from the header alone.
