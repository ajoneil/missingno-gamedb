# MissingNo Game Database

Multisystem game catalogue — commercial, homebrew, and demoscene — used by
[MissingNo](https://github.com/ajoneil/missingno). One RON manifest per game,
modelling games → releases (region / revision / hardware variants) → artifacts
(ROM dumps identified by SHA-1).

## Structure

```
data/
  gb/{slug}/manifest.ron    ← Game Boy (incl. CGB-enhanced dual-mode games)
  gbc/{slug}/manifest.ron   ← Game Boy Color (CGB-required)
  vcs/{slug}/manifest.ron   ← Atari VCS/2600
crates/
  gamedb/                   ← schema library: types, loader, validator
  gamedb-cli/               ← `gamedb` maintenance tool
```

A manifest holds the game's identity (title, kind: game / demo / demoscene,
developer, links, cover and screenshot URLs, mod derivation) and its releases:
regions, date, publisher, status (released / WIP / beta / prototype),
platform hardware facts (SGB/CGB enhancement for GB; TV format and board for
VCS), download sources, and ROM artifacts.

## Maintenance

```
cargo run -p missingno-gamedb-cli -- validate .   # schema + database rules
cargo run -p missingno-gamedb-cli -- fmt .        # canonical formatting
```

Manifests are kept in canonical formatting (enforced by `validate`) so every
change reviews as a minimal git diff.

## License

CC0 1.0 Universal — public domain dedication.
