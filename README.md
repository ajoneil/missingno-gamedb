# MissingNo Game Data

Catalogue of known Game Boy games — commercial and homebrew — with optional deep per-game data for save parsing, adventure logs, cheat codes, and community resource links.

Used by [MissingNo](https://github.com/ajoneil/missingno), a Game Boy emulator.

## Structure

```
games/
  {slug}/
    manifest.ron       ← identity, hashes, source, links (every game has this)
    family.ron          ← variant grouping for multi-release commercial games (optional)
    knowledge/          ← memory maps for save parsing (optional)
    watchpoints/        ← adventure log triggers (optional)
    cheats/             ← Game Genie / GameShark codes (optional)
    resources.ron       ← curated external links (optional)
shared/                 ← reusable data (text encodings, species tables)
scripts/                ← import scripts for populating the catalogue
```

## License

CC0 1.0 Universal — public domain dedication.
