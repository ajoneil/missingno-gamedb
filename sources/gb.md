# Game Boy and Game Boy Color sources

Per-tree catalogue for `data/gb` and `data/gbc`. The rules that apply everywhere
— never construct a URL, the ROM-hosting link ban, dump identity, titles,
publishers, cover art, licensing — are in [`README.md`](README.md).

## Catalogues

| Source | Good for | Notes |
|--------|----------|-------|
| [Games Database](https://www.gamesdatabase.org) — **link freely** | **game manuals** (direct PDFs), **box, cart and title-screen scans**, publisher, developer, year, category | The first stop for the commercial library. Vetted for the SG-1000 tree already — `robots.txt` allows all and it hosts no ROMs. Reaching anything on it takes the site's own search; see below. |
| [gbdev](https://gbdev.io) database and its Homebrew Hub — **link freely, read via GitHub** | homebrew authorship, licence, canonical cover art; Pandocs for cartridge-header tables | Primary sources beat aggregators: prefer the author's own repo or site to a catalogue entry. `robots.txt` disallows ClaudeBot, so the rendered site is off limits, but the `gbdev/database` and `gbdev/pandocs` repos are not — read both from GitHub. Homebrew Hub game pages are `/game/<slug>/`; the `/games/` form 404s. |
| [WLS](https://wls.hu) — **link freely** | the László Rajcsányi (WLS) homebrews: Blitz Bomber, Blockade, Berks, Bonkers, The Farm, G-Man, Climb It… | The author's own site, one page per game with the premise, a play-online build, a `Download` zip that hash-matches the No-Intro aftermarket dump, and a mock box image on the site's own host. His itch.io pages have gone 404, so this is the canonical page. |
| The project's own repo or site | everything, for homebrew | GitHub raw URLs are the canonical host for cover art and downloads. |
| MobyGames — **agents cannot read it; unvetted** | — | `robots.txt` disallows ClaudeBot outright. No facts and no links until a human vets it. |
| insideGadgets — **agents cannot read it** | aftermarket carts of homebrew | `robots.txt` disallows ClaudeBot, so its pages cannot be opened and therefore cannot be linked. |
| Hidden Palace — **banned** | — | Hosts the prototype ROMs it documents, so it is never linked (see README); its `robots.txt` also carries `Content-Signal: ai-input=no`, so it is not read either. |
| The Video Games Museum — **agents cannot read it; unvetted** | — | `robots.txt` names `ClaudeBot` in a long AI group closing with `Disallow: /`. Read a group to its directive before fetching. No facts and no links until a human vets it. |
| Handheld Underground (hhug.me) — **read-only, never linked** | provenance of the Taiwanese unlicensed dumps — which multicart a rip came off, and whether a dump is raw or header-fixed | The dumping project behind the `[multicart rip]` signature names. It serves the ROMs it documents, so the README's host ban applies: read it, cite it in chat, never link it. It has no `robots.txt`. |

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

**A single exact match redirects straight to the game page**, so zero result rows
is not absence — follow the redirect.

**Results page at 40 rows**, so a common word buries the Game Boy entry: post the
list back through its own `DropSys` system filter rather than paging.

**Its publisher and developer fields conflate similarly-named companies**, so
take both off the manual where one exists. The swap is to a real company with a
near-identical name, and reads as plausible unless the artefact is checked.

## Manuals

A manual documents *this* cart where an encyclopaedia article documents a
multi-platform game as a whole, so it is the best gameplay source this tree has.
Games Database links the PDF from the game page; download it and read it as page
images. Coverage is roughly a third of each library, so a game having none is
ordinary and not worth a second search. Record the language on the link.

**For a Japan-only release the box back stands in for a missing manual.**
Japanese boxes print gameplay copy rather than a marketing line, so
`artwork-box-back` usually describes a title that has neither manual nor article.
It also carries the compatibility badges: 通信ケーブル対応 is the only statement of
link-cable support, which the header does not hold.

## Cover art

README.md's order stands — Hasheous, then libretro-thumbnails. **A watermarked
scan is never staged**, whatever it would otherwise win on: Games Database
stamps its own domain across every image, so its artwork is a way to *read* a
box — the title as the cart prints it, the publisher logo — and never a cover.
Where those two hold nothing, the entry keeps no cover and the report says so.

**Hasheous usually loses to libretro here**, so compare before keeping what it
staged: it serves the art cropped free of the banner, seal and publisher logo,
which the banner rule demotes at any size, and sometimes another game entirely
out of a grouped record.

**`cover_candidates` lists the repo's `Named_Boxarts` rather than guessing a
filename**, matching a title through the region and enhancement qualifiers a
No-Intro name carries. A reported libretro candidate is therefore a file the
repo holds, and no candidate means it holds none.

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

**The emulator models the mapper**, so every volume plays; their menus cycle
the selection with SELECT and launch with START.

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
