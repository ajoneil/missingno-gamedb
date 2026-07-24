# Metadata sources

Where the facts in this database come from, per tree, and which sources answer
which fields. A staged fact should carry the page that backed it as a `links`
entry, so the receipt lives in the manifest rather than in someone's memory.

**Never construct a URL on any of these sites — not a page, not a listing, not
an index.** Every URL you fetch must come from somewhere: a search result, a
link read off a page you already fetched, or a filename convention this file
explicitly documents as constructible (libretro's No-Intro-named thumbnails are
the only current one). A guessed URL that 404s pollutes the site's logs like a
probe, and a guessed URL that *resolves* is worse — you may be reading the
wrong page while believing you searched. Reach a site through its search box,
a web search, or its index pages, and read the real `href`.

Add to this file when a source proves itself — it is the durable home for
per-system cataloguing knowledge.

**No Internet Archive links in the database for now** (Andrew, 2026-07-23):
the legality of linking archive.org items is undecided. Reading IA — Wayback
copies of blocked pages, item metadata for research — stays fine; what's out
is `archive.org` URLs in `links`. Revisit when the policy is settled.

## Dump identity, every tree

[Hasheous](https://hasheous.org) maps a SHA-1 to what a signature database
(TOSEC/No-Intro-style) calls that dump. That name is what distinguishes an
original from a hack, a bad dump, or a prototype, so it is the first question to
ask about any artifact:

```
gamedb verify-hashes --key <tree>/<slug>    # asks per hash, reports the answer
```

The answer is reported, never stored: a hash is re-checkable at any time, so
the manifest carries no verification evidence.

Ask it per entry, as part of curating that entry — never sweep the database.

Its bracket flags are the signal: `[h]` hack, `[t]` trained, `[tr]` translation,
`[cr]` cracked, `[a]` alternate, `[b]` bad dump, `[o]` overdump, `[f]` fixed.

Two cautions. Hasheous answers little about the *release* — publisher, country
and date usually need a catalogue below. And its `AIDescription` attribute is
machine-generated: never stage it.

Signature-database years also disagree with encyclopaedias — TOSEC in particular
is often a year or two off a documented release date. A disagreement is a
conflict to report, not a licence to restage the date.

## Release facts, per tree

| Tree | Source | Good for | Notes |
|------|--------|----------|-------|
| vcs | [Atarimania](https://www.atarimania.com) | publisher, **country**, year, model/reference number | The first stop once a dump is identified. Covers the obscure regional reissues (CCE, Genus, Dynacom, Funvision) that encyclopaedias omit entirely. **No constructed URLs of any kind** — page ids are numeric and the listing-page naming is irregular, so reach a game page only via a web search or the site's own search; robots.txt is empty, so ordinary reading is permitted. |
| vcs | AtariAge | gameplay descriptions + **manual scans** for obscure carts (Sancho, Zellers, Goliath) encyclopaedias omit | **HTML pages Cloudflare-challenge WebFetch, but a plain `curl -A 'Mozilla/5.0'` reaches `software_page.php`, `manual_page.php` and the scan images fine** — not blocked, just needs a normal client. A scanned manual IS a readable source: get the real full-page URL from `manual_page.php?SystemID=2600&SoftwareLabelID=<id>&currentPage=N` (`src="/2600/manuals/<Name>/m_<Name>_N.jpg"` — read it off the page, don't guess), `curl` the JPG, and open it with the Read tool (renders as a page image). Don't declare a gameplay description unsourceable until you've read the manual scan this way. Wayback (`archive.org/wayback/available`) is the fallback for a dead page, but WebFetch can't reach web.archive.org — `curl` it. |
| vcs | Stella's properties database | board/cart type when a playtest boots wrong | The board drives the emulator, so a garbled VCS playtest makes `cart_type` the first suspect. |
| vcs | [Atari Compendium manual archive](https://www.ataricompendium.com/archives/manuals/vcs/vcs_manuals.html) | scanned game manuals (`Manual` link) | ~1000 static PDFs hosted on-site. **Get the exact filename from the index page — never guess a slug.** The filenames are unpredictable (`3dtictactoe.pdf`, not `3-d_tic-tac-toe.pdf`): WebFetch the index and read the `href`. Sears-branded scans are separate files (`…-sears.pdf`) for the Sears reissue. |
| gb / gbc | [gbdev](https://gbdev.io) database, the project's own repo/site | homebrew authorship, licence, canonical cover art | Primary sources beat aggregators: prefer the author's repo to a catalogue entry. |
| demoscene | [pouet.net](https://www.pouet.net) | party, release date, group, prod imagery | Primary for demoscene productions. |

A `Manual` link should point at the **direct PDF** wherever one exists (the
Atari Compendium `.pdf`, an author's hosted PDF), not at a page that merely
links the manual — the reader wants the document, not a landing page. Fall back
to a containing page only when no direct file is available.

## Cover art

Remote URLs only — the database stores a link, never the bytes; it hosts and
redistributes no image. That is why third-party box art is fine here: pointing at
a service's image is not republishing it. Take the image from the record of the
dump actually meant: a hack's record carries the hack's art.

1. Hasheous (`.../api/v1/images/<id>`) for commercial games — it is built to serve
   emulator libraries, so it is the right primary source, not a fallback. Note it
   serves JPEG bytes under an `image/png` content type.
2. libretro-thumbnails when Hasheous has no image — also purpose-built for
   frontends, keyed by No-Intro filename (article suffixed: `… , The (USA).png`).
   Link their own host, `thumbnails.libretro.com/<system>/Named_Boxarts/<name>.png`
   (URL-encode spaces AND apostrophes: `%20`, `%27`), not raw.githubusercontent —
   the GitHub raw path is coupled to the repo's branch layout.
   A Wikipedia article's box art is a last resort (usually fair-use, worth noting).
3. Homebrew and demoscene: the project's own canonical host (GitHub raw URLs, the
   pouet prod page).

Never store-CDN URLs (itch/Steam image links churn) — a store page belongs as a
`DownloadPage` link, not in `covers`.

## Licensing

This database is **CC0**. Wikipedia is **CC BY-SA**, and the two do not compose:
take the facts, never the prose. A description must be written in your own words,
or the repo's LICENSE quietly stops being true for that entry.
