# Real card art: the delivery convention

Every card draws procedural art (`procedural.ts`, `svg.ts`) until real art is delivered for it, and
the procedural art stays the fallback after that: a card the manifest does not list, a face whose file
fails to load, and a Radiant face with no file of its own (which shows the base file under a gold
tint) all fall back without a request or an error. This file is the convention an artist's
delivery meets (R658). `convention.ts` holds its numbers, and `convention.test.ts` holds the real
directory and `manifest.ts` to them in CI.

## A delivery is a file plus a manifest line

1. Put the file(s) in `apps/web/public/art/`, named for the card's catalog id
   (`packages/cards/catalog.json`, tokens included):
   - `<id>.webp` for the base face, for example `core-002.webp`;
   - `<id>-radiant.webp` for the Radiant face, for example `core-002-radiant.webp`. Optional: a
     card with only a base file shows it under a gold tint on its Radiant face.
2. Add the card's line to `ART_MANIFEST` in `manifest.ts`, naming the faces delivered:
   `"core-002": { base: true, radiant: true },` (or `{ base: true }`).
3. Credit the artist, if they are credited, as the card's `artist` in `packages/cards/flavour.json`
   (`{ "flavour": "…", "artist": "Name" }`, at most `ARTIST_MAX_CHARS` characters). The inspect
   views and the Card Almanac show it as "Art by Name". That file is not card data, so the credit
   needs no patch fragment.

## The file

| | |
|---|---|
| Format | WebP (`.webp`), lossy or lossless, sRGB. Alpha is allowed but not needed: the frame covers the edges. |
| Size | Square, exactly 512 x 512 px (`ART_SIDE_PX`). |
| Weight | At most 96 KiB per file (`ART_MAX_BYTES`). Lossy at quality 75-85 lands well under it. |
| Composition | The subject in the middle half. Every shape crops the square with `object-fit: cover`: the Unit portrait, the Spell window, the Field Spell arch and the Trap's notched window keep most of it, the board's oval keeps the middle, and the deck list's strip keeps a band across it a little above centre (`art.css`). |
| Radiant face | The same picture's Radiant version, at the same size: the frame adds the gold foil and the shimmer, so the art need not. |
| Content | Original work only: no Blizzard or Konami art, frames, logos or trademarks (docs/polish/6-cards.md). No text in the picture; the face prints the name. |

## What the check refuses

`pnpm vitest run --project web apps/web/src/cards/art/convention.test.ts` fails, naming the file, on:

- a manifest line for an id the catalog does not have, or one that lists neither face;
- a listed file that is missing from `apps/web/public/art/`;
- a file that is not WebP, is not 512 x 512, or is over 96 KiB;
- a file in `apps/web/public/art/` that no manifest line lists (a delivery that forgot its line).
