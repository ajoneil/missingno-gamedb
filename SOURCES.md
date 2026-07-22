# Metadata sources

Where the facts in this database come from, per tree, and which sources answer
which fields. A staged fact should carry the page that backed it as a `links`
entry, so the receipt lives in the manifest rather than in someone's memory.

Add to this file when a source proves itself — it is the durable home for
per-system cataloguing knowledge.

## Dump identity, every tree

[Hasheous](https://hasheous.org) maps a SHA-1 to what a signature database
(TOSEC/No-Intro-style) calls that dump. That name is what distinguishes an
original from a hack, a bad dump, or a prototype, so it is the first question to
ask about any artifact:

```
gamedb verify-hashes --key <tree>/<slug>    # asks per hash, records the answer
```

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
| vcs | [Atarimania](https://www.atarimania.com) | publisher, **country**, year, model/reference number | The first stop once a dump is identified. Covers the obscure regional reissues (CCE, Genus, Dynacom, Funvision) that encyclopaedias omit entirely. Search by title rather than guessing numeric page ids; robots.txt is empty, so ordinary reading is permitted. |
| vcs | AtariAge | — | **Behind a Cloudflare challenge.** Treat as blocked: note the gap rather than working around it. |
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
   A Wikipedia article's box art is a last resort (usually fair-use, worth noting).
3. Homebrew and demoscene: the project's own canonical host (GitHub raw URLs, the
   pouet prod page).

Never store-CDN URLs (itch/Steam image links churn) — a store page belongs in
`sources`, not `covers`.

## Licensing

This database is **CC0**. Wikipedia is **CC BY-SA**, and the two do not compose:
take the facts, never the prose. A description must be written in your own words,
or the repo's LICENSE quietly stops being true for that entry.
