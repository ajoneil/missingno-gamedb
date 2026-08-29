# Game Boy and Game Boy Color sources

Per-tree catalogue for `data/gb` and `data/gbc`. The rules that apply everywhere
— never construct a URL, the ROM-hosting link ban, dump identity, titles,
publishers, cover art, licensing — are in [`README.md`](README.md).

## Catalogues

| Source | Good for | Notes |
|--------|----------|-------|
| [Games Database](https://www.gamesdatabase.org) — **link freely** | **game manuals** (direct PDFs), **box, cart and title-screen scans**, publisher, developer, year, category | The tree's first stop for the commercial library: 423 Game Boy and 295 Game Boy Color manuals. Vetted for the SG-1000 tree already — `robots.txt` is `Allow: /` and the site hosts no ROMs. It is an ASP.NET application, so reaching anything on it takes the site's own search; see below. |
| [gbdev](https://gbdev.io) database and its Homebrew Hub | homebrew authorship, licence, canonical cover art | Primary sources beat aggregators: prefer the author's own repo or site to a catalogue entry. |
| The project's own repo or site | everything, for homebrew | GitHub raw URLs are the canonical host for cover art and downloads. |
| MobyGames — **agents cannot read it; unvetted** | — | `robots.txt` carries an explicit `User-agent: ClaudeBot` / `Disallow: /`. No facts and no links until a human vets it, exactly as with SMS Power! on the SG-1000 tree. |
| Hidden Palace — **ask before reading** | prototypes | Its `robots.txt` carries `Content-Signal: search=yes, ai-input=no, ai-train=no`. Storing a link is not "ai-input"; an agent reading its pages for facts is the thing it declines. The tree already holds links here from earlier work. |

## Games Database: reaching a page

**Never take a Games Database URL from a web search.** Its media filenames are
indexed stale, and a stale one returns the site's own *"Error 404 — This page has
left the archive"* rather than anything that looks like a miss. Five were tried
and all five 404ed, including one read off the site's *own live* system page —
so a broken link there is not evidence the file is absent. The manual that
search called `Adventures_of_Lolo,_The_-_1995_-_Nintendo.pdf` is really
`Adventures_Of_Lolo_-_1994_-_Nintendo.pdf`: different capitalisation, different
year, no article. Guessing the shape from a row is the same mistake — read the
URL off the game page.

The search form posts from the site root and lands on a plain results URL,
`list.aspx?in=1&searchtext=<term>&searchtype=1`. Each result row is a
`__doPostBack('GridView1','GAME$<n>')` rather than a link, and following it
lands on `/game/nintendo-game-boy/<slug>`, which is where a current media URL
can finally be read. The rows themselves are worth reading first: the results
table (`id="GridView1"`) gives Game, System, Publisher, Developer, Category and
Year, which is how the right system's row is picked out of a title shared across
a dozen platforms.

```python
import re, html, urllib.parse, urllib.request, http.cookiejar
BASE = "https://www.gamesdatabase.org/"
opener = urllib.request.build_opener(
    urllib.request.HTTPCookieProcessor(http.cookiejar.CookieJar()))
opener.addheaders = [("User-Agent", "Mozilla/5.0")]
state = lambda p: {n: html.unescape(v) for n, v in re.findall(
    r'<input type="hidden" name="(__[A-Z]+)"[^>]*value="([^"]*)"', p)}

def post(url, fields, referer):
    body = urllib.parse.urlencode(fields).encode()
    r = opener.open(urllib.request.Request(url, body, {"Referer": referer}), timeout=60)
    return r.geturl(), r.read().decode("utf-8", "replace")

page = opener.open(BASE, timeout=60).read().decode("utf-8", "replace")
url, results = post(BASE, {**state(page), "txtsearch": TITLE,
                           "cmdSearch": "Search", "RadSearchType": "1"}, BASE)
# rows: re.findall(r'GAME\$(\d+)', results) against the GridView1 table
game_url, game = post(url, {**state(results), "__EVENTTARGET": "GridView1",
                            "__EVENTARGUMENT": f"GAME${n}"}, url)
# media: re.findall(r'(?:href|src)="([^"]*/Media/[^"]+)"', game)
```

**A game page carries other systems' media too.** The Game Boy page for
Adventures of Lolo links the *NES* advert scan. The system is in the path
(`/Media/SYSTEM/Nintendo_Game_Boy/…`), so check it before staging anything.

## Manuals

A manual is the best source this tree has for gameplay, and it documents *this*
cart where an encyclopaedia article documents a multi-platform game as a whole.
The PDF is linked from the game page under `/Manual/formated/`; download it and
read it as page images.

Coverage is 423 of the Game Boy library and 295 of the Game Boy Color's, so a
game having no manual here is ordinary and not worth a second search. Record the
language on the link as you curate it.

## Cover art

README.md's order stands — **Hasheous, then libretro-thumbnails** — with Games
Database as the fallback those two do not cover.

Its scans are the right *kind*: `Box/big/` holds a full box front showing the
Game Boy banner, which is what README.md's cover rule asks for, and the page also
carries box-back, cartridge, cartridge-top, title-screen and marquee art. **But
every scan is watermarked** with the site's own domain across the image, so it
loses to a clean scan of the same art at any resolution.

The thumbnail on the game page is not the image to stage. Follow the artwork
page it links — `/media/<system>/artwork-box/<year>/<slug>` — and take the
`Box/big/` URL off that.

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
