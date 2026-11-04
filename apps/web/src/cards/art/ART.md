# Real card art: the delivery convention

Every card draws procedural art (`procedural.ts`, `svg.ts`) until real art is delivered for it, and
the procedural art stays the fallback after that: a card the manifest does not list, a face whose file
fails to load, and a Radiant face with no file of its own (which shows the base file under a gold
tint) all fall back without a request or an error. This file is the convention an artist's
delivery meets (R660). `convention.ts` holds its numbers, and `convention.test.ts` holds the real
directory and `manifest.ts` to them in CI's daily super run (the pull-request web tests leave the
asset tests out).

## A delivery is a file plus a manifest line

1. Put the file(s) in `apps/web/public/art/`, named for the card's catalog id
   (`crates/cards/catalog.json`, tokens included):
   - `<id>.webp` for the base face, for example `core-002.webp`;
   - `<id>-radiant.webp` for the Radiant face, for example `core-002-radiant.webp`. Optional: a
     card with only a base file shows it under a gold tint on its Radiant face.
2. Add the card's line to `ART_MANIFEST` in `manifest.ts`, naming the faces delivered:
   `"core-002": { base: true, radiant: true },` (or `{ base: true }`).
3. Credit the artist, if they are credited, as the `artist` of the card's entry in
   `crates/cards/card-text.yaml` (`artist: Name`, beside its `flavour`, at most `ARTIST_MAX_CHARS`
   characters), then run `cargo test -p jackioh-tools --test export_card_text`, which writes it into
   `crates/cards/flavour.json`. The inspect views and the Card Almanac show it as "Art by Name". That
   file is not card data, so the credit needs no patch fragment.

## The file

| | |
|---|---|
| Format | WebP (`.webp`), lossy or lossless, sRGB. Alpha is allowed but not needed: the frame covers the edges. |
| Size | Square, exactly 512 x 512 px (`ART_SIDE_PX`). |
| Weight | At most 96 KiB per file (`ART_MAX_BYTES`). Lossy at quality 75-85 lands well under it. |
| Composition | The subject in the middle half. Every shape crops the square with `object-fit: cover`: the Unit portrait, the Spell window, the Field Spell arch and the Trap's notched window keep most of it, the board's oval keeps the middle, and the deck list's strip keeps a band across it a little above center (`art.css`). |
| Radiant face | The same picture's Radiant version, at the same size: the frame adds the gold foil and the shimmer, so the art need not. |
| Content | Original work only: no Blizzard or Konami art, frames, logos or trademarks (docs/polish/6-cards.md). No text in the picture; the face prints the name. |

## What the check refuses

`pnpm vitest run --project web apps/web/src/cards/art/convention.test.ts` fails, naming the file, on:

- a manifest line for an id the catalog does not have, or one that lists neither face;
- a listed file that is missing from `apps/web/public/art/`;
- a file that is not WebP, is not 512 x 512, or is over 96 KiB;
- a file in `apps/web/public/art/` that no manifest line lists (a delivery that forgot its line).

## The Stinkyscape set's own art

The Stinkyscape set's entries (`stinkyscape-…`) draw `stinkyscape.ts`'s procedural art instead of
`procedural.ts`'s: a space backdrop under one subject per family, each weapon the weapon its name says
(R2170000). It is the same fallback as the rest: a delivered file for one of its cards still wins.

## The Training Grounds set's own art

The Training Grounds set's entries (`traininggrounds-…`) draw `traininggrounds.ts`'s procedural art
the same way: a training yard under one subject per family, its dummies, ogres, knives, totems,
Felinors, runes, posters and Golem each what the card is (R2202000). Its Legacy cards #41–#60
(`SNAPSHOT_ART`) draw their Core card's subject in the yard, each with a touch of its own
(`LEGACY_TOUCHES`, R2724000); a delivered file is read by the Legacy card's own id.

## The full-art cards' moving art

A full-art card's procedural picture also has a moving version (R2715001, `motion.ts`): given the
card's recipe (`FULL_ART_MOTION`), the painters tag each layer by its role (background, subject,
over, light, props, motes), add particles that suit the card, and put per-card keyframes in the
picture, transform and opacity only, behind `prefers-reduced-motion`. Without a recipe the picture is
byte for byte the still one. The Animated art, the Legendary and the Mythic wear it (R2715000), a few
faces at once (`useArtMotion`). A delivered file is still art: a card with real art draws no moving
version, so delivering a full-art card's file means delivering its motion too, or the card stops
moving.

## Pictures (R2949000)

A look with a picture of its own, the Signature look today, draws the card's own painter re-seeded:
`pictures.ts`'s `artSeedOf` joins the card's id and the picture's name. The Stinkyscape and Training
Grounds painters keep the card's subject and grow the rest from that seed (backdrop, drift, layout,
particles); the procedural painter keeps what the card's name, tags and type pick (its theme,
composition and motif) and grows its figure, backdrop and particles from it. Either way the picture is
still a pure function of the card's id, name, tags and type. The cache keys the seed in the card id's place, so a look's picture is never
served for another's. The normal picture's seed is the card's id alone, so every still picture and
golden hash stands. The Signature look moves by `SIGNATURE_MOTION`, which no Animated motion is. A later
version with a picture of its own (#2940's Alt full art) adds its own name to `ArtPicture`, so it draws
apart from every other. A Starship's art, a Legacy card's and a delivered file keep their one picture.

## The versions' pictures (R2940001)

Alt art and Alt full art draw the picture `"alt"`, seeded apart by `artSeedOf` as the Signature
look's is, so the two versions share one picture and neither is the normal one or Signature's. The
Sketch (`"sketch"`) draws no picture of its own: `sketchSvg` lays the normal picture, whichever
painter drew it (a Legacy card's and a Starship's too), under one SVG filter, its greys dodged into
line work, hatched in slate ink where it is dark and grained, on slate paper; a Radiant face washes
its painted picture back over the pencil. Its strokes are seeded by the card's id, and the cache keys
it apart (`|sketch`), so the plain picture keeps its key and bytes. The filter's constants
(`SKETCH_*`) were tuned by eye on the component spec's screenshot (`card-versions-part-2`). A
delivered file is the normal picture's, so Alt art and the Sketch draw the painter's picture until an
artist delivers theirs.

## Part 3's pictures (R2942001)

Extended art draws the card's normal picture, a delivered file included, at the full-art shape: the
picture runs past its window across the face, and the face's frame and rules box lie over it,
translucent (cards.css). Pixel (`"pixel"`) and Comic (`"comic"`) draw no picture of their own: as
the Sketch does, each lays the normal picture, whichever painter drew it (a Legacy card's and a
Starship's too), under one SVG filter (`restyledSvg`). `pixelSvg` samples it once in the middle of
every `PIXEL_CELL` square and grows each sample by half a cell, a little past its cell, over the
picture blurred by half a cell: a browser grows a sample by whole pixels, so on a small face the
blur fills whatever seam is left between cells. On a base face it cuts every colour channel to its
nearest of `PIXEL_LEVELS`, 27 colours whose hues are multiples of 30°, none in Radiant's gold, and a
Radiant face keeps its painted colours. `comicSvg` smooths and brightens the picture
(`COMIC_SMOOTH`, `COMIC_LIFT`), pushes its colour (`COMIC_SATURATE`), flattens it to `COMIC_LEVELS`,
lays halftone dots of `COMIC_INK` over its lighter parts and inks every edge between its flat
colours; a Radiant face washes its painted picture back over it. Neither takes a seed, and the cache
keys each apart (`|pixel`, `|comic`), so the plain picture and the Sketch keep their keys and bytes.
A delivered file is the normal picture's, so Pixel and Comic draw the painter's picture until an
artist delivers theirs. The constants were set by eye on the component spec's screenshot
(`card-versions-part-3`).

## Part 4's pictures (R2943001)

Set showcase (`"showcase-<family>"`, by the card's set: `showcaseFamilyOf`, Classic+ and Meditative+
drawn as Classic and Meditative) draws no picture of its own: as the Sketch, Pixel and Comic do, it
lays the normal picture, whichever painter drew it, under one SVG filter (`showcaseSvg`), and may draw
markup of its own over it. Core's glass orb (`sc-core`) pushes the picture's colour and contrast and
lays a blue rim and a glint over it; Classic's temple stone (`sc-stone`) greys it and lights its
lightness as relief (`feDiffuseLighting`) in stone; Meditative's ink wash (`sc-ink`) greys, softens
and cuts it to `SHOWCASE_INK_LEVELS` of ink on paper, with a red seal; Stinkyscape's deep space
(`sc-space`) maps its lightness from the dark of space to violet, with sparse stars in its dark; and
Training Grounds' carved wood (`sc-wood`) cuts it to `SHOWCASE_WOOD_LEVELS` of wood and grains it. A
Radiant face washes its painted picture back over every treatment but Core's, whose colours are the
picture's own. The cache keys each apart (`|showcase-<family>`).

Out of frame draws the normal picture in two parts (`ArtPart`): the window its `"backdrop"`, every
layer but the subject's, and a cutout over the frame its `"subject"`, the subject's layers alone on
nothing (no sky, no vignette), in the same 100-unit box, so the two line up. The procedural painter's
subject is its subject-role ridges and its emblem; Stinkyscape's and Training Grounds' their
`moving("subject", …)` group, Training Grounds' shadow staying with the backdrop. The `"whole"`
picture is the one every face drew before, byte for byte, and keeps its cache key; each part is cached
apart (`|backdrop`, `|subject`). A delivered file has no parts, so a window drawing one gives way to
the painter, and a Starship in play and a Legacy card, whose painters draw one picture
(`hasArtParts`), are never offered Out of frame. The constants were set by eye on the component
spec's screenshot (`card-versions-part-4`).
