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
(R2170000). It is the same fallback as the rest: a delivered file for one of its cards still wins. Two
cards have a look of their own (R3142002): a Human Unit whose name holds "drone" (the Drone Commander) is the
`crew` family's `commander`, an officer with a drone hovering behind her, where its name alone sends a card to
the `drone` family; and the Stinkyscape Golem, which prints no Starship Piece, is the `fleet` family's `golem`,
a stone-plated humanoid with engine nozzles, wing fins and a cockpit visor, found by its name (`FLEET_NAMES`)
and no longer by `STARSHIP_PIECES`, which `stinkyscape.test.tsx` holds to the catalog's. A card with a scene
recipe draws its scene instead (R3331000, below): since #3391 the 64 cards of its table, every card numbered 1 to
90 but the Starship Pieces (#3333's) and Olivian Expanse (#3338's), each draw a scene of their own (R3391000,
"Stinkyscape's scenes" below), and #3403's 103 entries, Stinkyscape #91 to #176 with their tokens, draw theirs
(R3403000, below); the cards other issues draw (the Starship Pieces, the Drone Battleship's layers, Anomaly Found!,
the Terminus tokens and Dice of Doom) keep their pictures.

The Drone Battleship's three layers (`stinkyscape-t-meltdown-layer`, `-propagation-layer`, `-shield-layer`) are the
decks of one ship (R3336000): the reactor keel with its core glowing through the vents, the hangar decks with their
bay doors and racked drones, and the command superstructure with its emitter spires and dome, each in its layer's
color, gilded when Radiant. A printed card draws its own deck large with the other two faint behind it. A layer on
the board (`MinionFace` gives `CardArt` its `ship`, `shipArtOf`) draws its own deck and every deck beneath it,
stacked, so the top card shows the ship as far as it stands, `data-art-decks` three, two or one; the keel's core
glows hotter and its cracks spread with the health it has lost (in `SHIP_HEAT_STEPS`) and a low Brittle, and the
hangar's lights stay lit for the bays that still hold drones. The three layers' hashes in
`fixtures/stinkyscape-art-v0.5.0.json` were recorded again for it, and no other entry's.

The Terminus cards draw pictures of their own (R3338001), found by id (`TERMINUS_KINDS`; Dice of Doom prints no
Terminus tag): Olivian Expanse an endless violet expanse with the Terminus cards as distant falling stars, Farewell
an empty table under a single lamp, KY's Last Exam one desk in an exam hall under a problem chalked across the sky,
Event Horizon a light caught at a black hole's edge, Last Word a hand catching a falling crown, 1v1 Final
Destination one lane of light over an abyss with a hero at each end, The Big Crunch the universe collapsing to a
point and Dice of Doom a hundred-sided die on black velvet, 67 glowing faintly on one face. None draws the shared
black hole's disk (`TERMINUS_DISK_BACK`), which a Terminus card the table does not list still draws. Core #100
Ceaseless Void, a procedural card, draws a composition of its own (`CARD_COMPOSITIONS`, `compositionOf`): a hole in
the shape of a giant, stars falling into its outline.

## The Training Grounds set's own art

The Training Grounds set's entries (`traininggrounds-…`) draw `traininggrounds.ts`'s procedural art
the same way: a training yard under one subject per family, its dummies, ogres, knives, totems,
Felinors, runes, posters and Golem each what the card is (R2202000). Its Legacy cards #41–#60
(`SNAPSHOT_ART`) draw their Core card's subject in the yard, each with a touch of its own
(`LEGACY_TOUCHES`, R2724000); a delivered file is read by the Legacy card's own id. Since #3393 every
entry of the set, its Legacy cards included, draws a scene recipe of its own first (R3393000, R3393001,
"Meditative+ and Training Grounds drawn as scenes" below), so this painter and `legacySvg` are the fallback
for a card with no recipe.

## A fused card's art (R3045000)

A fusion joins its ingredients' names and unions their tags, which this art reads. So `<CardArt>` first resolves the card
whose art it draws (`fusedArt.ts`): a fused definition holding a Miner (Stinkyscape #62 to #66) or a Weapon-tagged card
draws the last ingredient that is neither, the host an Equip was fused onto, by that card's id, name and tags, so its
delivered file, family, seed and theme are the host's. Any other card, and a fusion whose ingredients a digest id hides,
draws as it is handed; a Starship in play draws its own (R2249000).

## The Starship's art (R3333000, R3333001, R3333002)

Three files draw it, on `stinkyDraw.ts`'s primitives (the shapes `stinkyscape.ts` draws with, moved there unchanged):

- `starshipModules.ts`: each of the fifty piece definitions' module (`STARSHIP_MODULE_IDS`: the 47 cards printing
  Starship Piece and Stinkyscape #127 to #129), its kind, its name's tier, the hardpoint it mounts on (bow, spine,
  flank, keel or stern) and the bay its own card stands in. A module is drawn about its mount in flat fills, never a
  gradient, so any picture mounts it without its defs; the Beams, Repeaters, Cannons, Flak and Drills grow v1 to v5.
  An id that is no piece draws the spare plate.
- `starshipHulls.ts`: the hull designs (`STARSHIP_DESIGNS`, one for each of `STARSHIP_NAMES` and the shipyard's), the
  classes from a frigate to a dreadnought (`STARSHIP_CLASSES`), the hardpoints' placements, the hull, its ledger of
  running lights (one a piece, oldest at the bow, so the order shows), the escorts and the shipyard, and
  `StarshipArt`, the painter's inputs: its pieces oldest first, its launch, its design, its livery (`null`: the
  design's own) and its face. A cosmetic (#3330) hands the painter another design or livery; its memo key
  (`starshipArtKey`) holds them all.
- `stinkyscape.ts`: `starshipSvg(ship)`, the picture, under `STARSHIP_ART_MAX_BYTES` with 16 Radiant pieces, since a
  board can hold two; a piece's own card (`pieceCardSvg`), its module large on a cutaway of a hull in its bay; and
  `starshipModuleIconSvg`, the icon a Dormant hull's plates (`StarshipHull.tsx`) and the pieces list
  (`StarshipPieces.tsx`) show.

`<CardArt starship={{ launched, list }}>` reads the design from the face's name, the one a launched Starship took, and
marks it `data-art-design`. The collection's Dormant Starship, out of play, keeps its R503 picture.

## The full-art cards' moving art

A full-art card's procedural picture also has a moving version (R2715001, `motion.ts`): given the
card's recipe (`FULL_ART_MOTION`), the painters tag each layer by its role (background, subject,
over, light, props, motes), add particles that suit the card, and put per-card keyframes in the
picture, transform and opacity only, behind `prefers-reduced-motion`. Without a recipe the picture is
byte for byte the still one. Every animated level and the signed one above it wear it (R3015002, as the
Animated art, the Legendary and the Mythic did, R2715000), a few faces at once (`useArtMotion`). A delivered file is still art: a card with real art draws no moving
version, so delivering a full-art card's file means delivering its motion too, or the card stops
moving. R3340000: the 21 full-art cards' recipes are hand-written and every other Legendary and Mythic has
one of its own (`RARE_ART_MOTION`, built by `recipe` from the card's type, tags and set, none drawing a gold
particle: R2725004), so no Legendary or Mythic borrows a motion by id; `motion.test.ts` lists the ones that
still would and holds the list empty. Every card keeps the motion it borrowed before.

## Art that lives in play (R3340000)

A card on the board moves its art too, and answers the player (`artLive.ts`, `art/artLive.css`).

- **Who moves.** A board face (a Unit's `MinionFace`, a face-up backrow card's compact `CardFace`) moves its art for an
  animated or signed level and for every Mythic, in any look, in the one budget of motion slots the hand shares
  (`motionSlots.ts`, `COSMETIC_FOIL_ANIMATED_MAX`). The slots go to the face under the pointer or focus (and an
  attacker), then Mythics newest first, Legendaries, then the rest newest first; a freed slot passes on in the same
  tick; a hidden tab and a face off screen hold still. `animatedFoil`, Reduce motion, the system's setting and Fx
  intensity (`off` stills all, `low` leaves the first, the Mythics and the Legendaries) still rule. A face that holds
  no slot draws its still picture.
- **The layer stack.** A moving board face draws its picture three times, one image each (`CardArt` `live`): the
  backdrop on `.cf-art` itself (the painter's `part` "backdrop" with a *detached* motion, which draws no particles),
  the subject over it (`.cf-art-subject`, part "subject") and the motes (`.cf-art-motes`, `particleLayerUri`: the
  particles alone). So every painter's roles, subject, light and motes, give every card the reactions, and a recipe
  needs nothing of its own for them. A picture with no parts (a delivered file, a restyle, a Starship's, a Legacy
  card's, a Battleship's) stays one image and answers as a whole. So does a face that draws depth planes (R3344000, an Epic or rarer face):
  its planes are its layers, so its window is not split again.
- **The reactions.** Hover or focus: the subject turns and leans toward the pointer (`--art-aim-x`/`-y` from
  `aimOf`), the motes scatter and settle. Keyboard focus alone, which has no pointer coordinate, uses a subtle fixed
  upper-right pose; the pointer's place wins while both are on the face. Press: the subject flinches (`scale`) and
  springs back on the overshoot curve. `attackDeclared`: the subject swings on the attack's own `--fx-lunge-x/y` and
  the motes fire. All in `rotate`, `translate`, `scale` and `opacity`, inside the reduced-motion gate, with
  `motion.css`'s times and curves.
- **The marks.** A Unit's art wears cracks for the health it is missing (`wear.ts`: `damageLevel`, one more of
  `ART_WEAR_LEVELS` steps each time, each crack grown from the card's id so a level's cracks start the next
  level's), a glow in its card's theme colour when buffed (`--art-glow`) and frost when frozen
  (`ArtWear.tsx`). They read the view's numbers and keywords only, and are still drawings: reduced motion keeps them.

## Pictures (R2940001)

A look with a picture of its own, Alt art's today, draws the card's own painter re-seeded:
`pictures.ts`'s `artSeedOf` joins the card's id and the picture's name. The Stinkyscape and Training
Grounds painters keep the card's subject and grow the rest from that seed (backdrop, drift, layout,
particles); the procedural painter keeps what the card's name, tags and type pick (its theme,
composition and motif) and grows its figure, backdrop and particles from it. Either way the picture is
still a pure function of the card's id, name, tags and type. The cache keys the seed in the card id's place, so a look's picture is never
served for another's. The normal picture's seed is the card's id alone, so every still picture and
golden hash stands. A later version with a picture of its own adds its own name to `ArtPicture`, so it
draws apart from every other. A Starship's art, a Legacy card's and a delivered file keep their one picture.
A Classic+, Meditative+, Training Grounds, Meditative or Core card offered Alt art, and a card of #3406's table,
Stinkyscape's later cards, draws an alt recipe of its own instead (R3396000, R3397000, R3404000, R3398000 and
R3406000, below).

## One picture per variant (R3015002)

A variant is a ladder of levels (`versions.ts`, R3015000), and every level of it draws its first
level's picture (`artPictureOf`): Full art's levels the normal picture, Alt art's the `"alt"` one, the
Sketch's full art the `"sketch"`, each one-art variant's animated level its own restyle. A level adds
only the full-art frame, its picture's own motion (`artMotionFor`, the one motion a card has) and a
signature, so a signed level is its variant's full art, moving, with the signed name. #2949's Signature
look, a picture and a motion of its own, is gone: no `"signature"` picture and no `SIGNATURE_MOTION`.

## The versions' pictures (R2940001)

Alt art and Alt full art draw the picture `"alt"`, seeded apart by `artSeedOf`, so the two versions
share one picture and neither is the normal one. The
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

## Part 5's pictures (R2944001)

Immersive (`"depth"`) and Prismatic (`"prismatic"`) draw no picture of their own either: each lays the
normal picture under one SVG filter. Immersive's window draws the picture's `"backdrop"` as far off
(`depthSvg`, `dp-haze`): softened by `DEPTH_BLUR`, its colour dulled to `DEPTH_SATURATE` and seen
through sky-blue air at `DEPTH_HAZE_MIX`; over it a near layer (CardFace's `.cf-immersive`, the
window's own box and shape) draws the `"subject"` as it is, unfiltered, so the two line up as Out of
frame's do and the subject stands out of the haze. A Starship in play and a Legacy card, whose
painters draw one picture (`hasArtParts`), draw it whole and hazed with no near layer; the Collection's
tile shows the whole picture hazed. Prismatic (`prismaticSvg`, `pr-chrome`) greys the picture, bands
its grey through `PRISMATIC_CHROME_TABLE`, light and dark in turn as polished metal catches its
surroundings, maps it from slate to near-white chrome and lays an iridescent sheen over it, corner to
corner, `PRISMATIC_SHEEN`: its no-gold spectrum there and back, so no two neighbours mix to gold. A
Radiant face washes its painted picture back over both (`dp-wash`, `pr-wash`). The cache keys each
apart (`|depth`, `|prismatic`). The constants were set by eye on the component spec's screenshot
(`card-versions-part-5`).

## Part 6's pictures (R3017002)

Part 6's six looks are all filters over a painter's picture, as part 5's are, but four of them are
**repaints** (`ArtRepaint`, `isRepaint`): the painter draws a new picture of the card's subject, seeded
by the card's id and the picture's name (`core-022#night`, `artSeedOf`) as Alt art's is, and the filter
lies over that. Night (`nightSvg`, `nt-night`) dulls, dims and blues the picture toward a night sky and
draws a moon in its glow and stars at fixed spots high in the window, inside every shape's crop.
Watercolor (`watercolorSvg`, `wc-paint`) runs the picture's edges by noise, softens them, lays them on
paper and thins them toward it, grained. Stained-glass window (`stainedGlassSvg`, `sg-glass`) pushes the
picture's colour and cuts it to Pixel's 27 colours, lights it from behind and leads it along every edge
between its panes and along a noise contour that parts the broad ones. Tarot card (`tarotSvg`,
`tr-tarot`) mutes the picture under a blue and pale duotone inside an arched violet border on two
pillars, a sun and a crescent in its spandrels and stars on it, and no words. The other two
**restyle** the card's own picture, seeded by its id alone, as Pixel does: Woodcut (`woodcutSvg`,
`wd-print`) greys it and cuts it to three tones of ink and paper, gouging its middle tone with long
strokes, and Neon sign (`neonSvg`, `ne-neon`) runs a tube along every step of its smoothed grey, violet
in the dark and cyan in the light, glowing on a dark wall that shows the picture faintly.

A Legacy card and a Starship in play keep their one picture under each, as under the Sketch, and a
delivered file, the normal picture's, gives way to the painter for all six. A Radiant face washes its
painted picture back over each (`nt-wash`, `wc-wash`, `sg-wash`, `tr-wash`, `wd-wash`, `ne-wash`), and
the cache keys each apart (`|night`, `|watercolor`, `|stained-glass`, `|tarot`, `|woodcut`, `|neon`),
a repaint's under its own seed too. Every colour is palette.css's, none in Radiant's gold window. The
constants were set by eye on the component spec's screenshot (`card-versions-part-6`).

## The art bible (R3331000)

What each set family's pictures look like, so every picture pass under #3330 paints its set as one world. A
family's **home** is the place its pictures happen in (Set showcase's treatments start from the same five:
R2943001); its **palette** is named in palette.css's names, the only colours a scene kit paints with
(`SCENE_PALETTE`), none of them in Radiant's gold window (no `--pal-gold-400`, `--pal-gold-300`,
`--pal-gold-200`, `--pal-yellow-400` or `--pal-parchment` on a base face, R2725004); its **line** is the outline
its inked shapes take (`SCENE_LINES`); its **camera** is where the viewer stands. A glow is a stack of
translucent shapes, never a gradient, and a Radiant face repaints the same shapes toward its layers' golds.

Every family climbs the same ladder of rarity, held by the detail floor below:

- **Common:** one clear subject and a simple setting.
- **Rare:** a setting with props.
- **Epic:** a scene with a foreground, a middle and a background.
- **Legendary:** a composed scene with lighting and a moment of action.
- **Mythic:** a set piece of its own: many parts, depth, effects, and a silhouette nobody else has.

### Core

- **Home:** the glass orb: a clear, bright world as if seen through glass, its fields, heights and shrines
  in clean flat shapes.
- **Palette:** the blues and skies (`--pal-blue-800`, `--pal-blue-900`, `--pal-sky-300`, `--pal-sky-400`),
  `--pal-cyan-400` for power, the greens of its fields (`--pal-green-700`, `--pal-green-900`), and
  `--pal-slate-100` for glints and clouds; a dusk runs to `--pal-orange-400` at the horizon.
- **Light:** from the upper left, with a soft glow behind the subject.
- **Line:** none: shapes meet edge to edge, flat fills stacked in translucent layers.
- **Camera:** eye level, the subject standing in the middle half.
- **By rarity:** a Common is one figure in a plain place (Speciesist, flinging a bolt across a whitewashed
  checkpoint at noon); a Rare stands among props, a fence, a cart, a banner; an Epic sets its background behind
  and its foreground before it; a Legendary lights its moment with rays and a halo; a Mythic is a set piece no
  other card shares (Heroic Power's marble rotunda of thirteen glowing doorways round an orb on its crystal
  pedestal, Ceaseless Void's giant-shaped hole before the void among a world's ruins).

### Classic and Classic+

- **Home:** temple stone: an old world of carved stone, graveyards, farms and ruins under a low sun or a
  moon.
- **Palette:** `--pal-stone-500` and the slates (`--pal-slate-700`, `--pal-slate-800`, `--pal-slate-900`),
  the earth's woods (`--pal-wood-600`, `--pal-wood-700`, `--pal-wood-800`), `--pal-green-700` for growth,
  `--pal-red-600` for banners and blood, `--pal-green-400` for plague.
- **Light:** low, from the left, or a moon; long shadows.
- **Line:** `--pal-slate-900` ink, half a unit wide, on every inked shape.
- **Camera:** low, looking up at the subject.
- **By rarity:** a Common is one figure or thing on a plain ground; a Rare adds its props, a tombstone, a
  pitchfork; an Epic is a scene in depth (Corpse Plantation's moonlit graveyard farm, its rows of
  grave-mounds running to the horizon, hands and sprouts pushing up from the furrows, a bone scarecrow over
  it, plague mist in the rows and a lantern on a fence post); a Legendary lights a moment of it; a Mythic is
  a set piece of the old world, a temple, a siege, a harvest of the dead.

### Meditative and Meditative+

- **Home:** ink wash: mountains in mist, gardens, scrolls and still water.
- **Palette:** `--pal-slate-900` ink on `--pal-slate-100` paper, `--pal-jade-400` washes, and one
  `--pal-red-600` seal.
- **Light:** diffuse, with no cast shadow.
- **Line:** a brush: `--pal-slate-900`, nine tenths of a unit wide.
- **Camera:** high and distant, a small subject in a large space.
- **By rarity:** a Common is one figure or stone in the mist; a Rare adds a pine, a lantern, a bridge; an
  Epic layers peaks, a garden and a near branch; a Legendary catches a moment, a wave or a falling leaf, in a
  shaft of light; a Mythic is a whole scroll of its own.

### Stinkyscape

- **Home:** deep space: fleets, stations, asteroids and nebulae.
- **Palette:** `--pal-black` and `--pal-slate-900` for space, the violets (`--pal-violet-500`,
  `--pal-violet-700`, `--pal-purple-400`) for nebulae, `--pal-cyan-400` for lights and shields, and
  `--pal-slate-400` for hulls and armour.
- **Light:** a star's rim light from the right, and the glow of engines and screens.
- **Line:** fine `--pal-black`, four tenths of a unit wide.
- **Camera:** tilted three-quarter, as if floating.
- **By rarity:** a Common is one crew member or craft among the stars (Arg Arg Defender, shield raised); a
  Rare adds its cargo and its asteroid; an Epic layers a planet, a station and debris before it; a
  Legendary fires its weapons or launches; a Mythic is a fleet action of its own.

### Training Grounds

- **Home:** carved wood: the training yard, its palisade, its dummies and targets.
- **Palette:** the woods (`--pal-wood-600` to `--pal-wood-950`), the brasses (`--pal-brass-500`,
  `--pal-brass-600`), `--pal-green-700` for the grass beyond the wall and `--pal-sky-300` for the sky.
- **Light:** the sun, from the upper right.
- **Line:** `--pal-wood-950`, seven tenths of a unit wide.
- **Camera:** eye level, across the yard.
- **By rarity:** a Common is one recruit or dummy on the sand; a Rare adds the rack, the target, the totem;
  an Epic sets the palisade behind and the gear before it; a Legendary catches a strike landing; a Mythic is
  the whole yard at the moment of a bout.

## Scene recipes and kits (R3331000)

A **scene recipe** (`sceneRecipes.ts`, `SCENE_RECIPES`) is data per card: its family, its sky (two palette
names, top to bottom), the light behind it, where its emblem stands, an optional figure, and seven layers
back to front, `sky`, `far`, `mid`, `subject`, `near`, `light` and `particles` (`SCENE_LAYERS`), each a list
of placements, and optional further figures (`extras`, R3390000), each standing in a layer of its own. A
placement names a part of the kit, where it stands, its scale and facing, how many copies
(`count`, each but the first offset by up to `spread`, every copy's scale jittered by `SCENE_SCALE_JITTER`)
and a palette name that repaints the part's main shapes; a placement marked `props` (R3390000) moves on the
props beat, as a painter's motif glyphs and accent do, and never stands in the subject layer. `composeScene` draws every part from one stream,
seeded by the card's id (or by a picture's own seed, `core-002#alt`, so Alt art and the repaints grow the same
parts apart), and never reads the face: the Radiant face draws the same shapes, each mixed toward its layer's
gold (`SCENE_RADIANT_GOLD`), with the painter's rays. The figure stands in front of its subject layer's own
parts, and a recipe's further figures (`extras`, R3390000: a crowd, a companion, a rival) each in front of its
own layer's parts, after the main figure in the subject layer; a placement, a figure or a further figure may
`fade` (R3393000), its shapes' opacity scaled, for a ghost, a copy or a recruit flickering into being. R3391000:
a placement may turn a pointing part (`rotate`, in degrees clockwise: a beam, a light stream, a light cone, a
meteor, a smoke trail or a comet, each extending along +x unturned); every other part ignores the turn, and a
placement with none draws exactly as before. `withScene` lays the scene into the procedural painter's `ArtSpec` (`artSpec`'s last argument, read
from the card's id), so it prints, moves (each layer on its role's beat: `back`, `subject`, `over`, `light`,
`motes`, and its marked placements `props`), splits for Out of frame (the subject layer and the emblem) and restyles as every procedural
picture does, and keeps the card's theme, layout, backdrop, sky scheme, emblem glyph and motif. The
Stinkyscape and Training Grounds painters hand a recipe card to it (`svg.ts` `sceneArtSvg`), and a Legacy
card draws its own scene wearing its Core card's frozen emblem (`legacySceneSvg`, R3393001), never its Core
card's scene.

The **scene kit** (`sceneKit.ts`, `SCENE_KIT`) holds the scenery, at least `SCENE_KIT_MIN_PER_CATEGORY` of
each kind, 770 parts since the passes of Classic+ (R3390000), Core (R3392000), Classic+'s Alt art (R3396000),
Meditative+ and Training Grounds (R3393000), Stinkyscape (R3391000), Meditative (R3401000), Stinkyscape's later cards
(R3403000) and the Alt art passes after them: 64 of terrain (fields, furrows, grave-mounds, cliffs, floors, rooms, streets, hills,
asteroids, water, sand, snow, stages, a cavern, a chessboard, a floating island, ship decks, corridors, regolith,
lava flats, floating platforms...), 149 of architecture (fences, posts, gates, ruins, towers, columns, arches,
tombstones, windows, doors, houses, domes, a lighthouse, a rotunda, stations, docks, hangars, vaults, server cores,
cells, consoles, masts, bunks, seats, cockpits, portholes and the open space they look out on, cases, stalls,
disruptor beacons, reactors...), 35 plants, 84 creatures (a scarab, a gelatin cube, a panther, a dragon, a sheep, a
loaf, a Rush Token, soldiers, a crowd...), 306 objects and tools, 109 of weather and light (moons, stars, clouds,
mist, shafts, halos, rays, sparks, embers, motes, a void, lightning, nebulae, planets, a rim light, waves, a
rainbow, a cyclone, beams, light streams, light cones, meteors, smoke, comets, star glare, gas giants, shattered
space, light grids, rifts, frost, missiles, flame, hex shields...), and 23 craft (R3391000: fighters, a scout, a
walker, a skiff, drones (a helper with arms among them) and their corvette and battleship, a gunship, miners and a
barge, a trader, a flagship, a frame, a fused hull, wrecks, a ghost ship, a rocket, a tram, a ring of guns), each
facing +x about its anchor. The **figure kit** (`figureKit.ts`, `FIGURE_KIT`) replaces the painter's one
silhouette: 22 poses, 24 heads, 43 hairs and headgears, 57 outfits and 69 held items, put together by a look
(`FIGURE_LOOKS`, `figureLookFor`), so a Unit reads as who it is: a Terminus, a CN or KY character, a Felinor, a
Pancake, a Fruit, Jlockeed's people or a Human; a set's pass dresses a look further for one card
(`{ ...figureLookFor(tags), outfit, item, cloth }`). The two kits
hold at least `SCENE_KIT_MIN_PARTS` parts; each part draws one to twelve filled shapes, a line drawn as a
thin band.

Five proof cards drew from recipes first and set the bar each set's pass meets: Corpse Plantation (Classic
#74), Speciesist (Core #2), Heroic Power (Core #98), Ceaseless Void (Core #100) and Arg Arg Defender
(Stinkyscape #15, since redrawn to its brief by Stinkyscape's pass). Ceaseless Void's recipe keeps its
painter's own subject (`ownSubject`), the giant its composition draws (R3338001), in place of a figure.
Classic+'s pass draws every Classic+ entry from a recipe of its own (below, R3390000), Core's pass every Core
entry but Ceaseless Void, Speciesist's and Heroic Power's redrawn from their briefs (Core's scenes, below,
R3392000), the Meditative+ and Training Grounds pass every entry of those two sets (below, R3393000), Meditative's
every Meditative entry (below, R3401000), Stinkyscape's the 64 cards of its table (below, R3391000),
Stinkyscape's later cards theirs (below, R3403000), and Classic's every Classic entry but Corpse Plantation (below,
R3352000).
Every other card draws its picture byte for byte as before, and the golden fixtures stand for it: `core-art-v0.1.json`, `art-before-2724.json` and `stinkyscape-art-v0.5.0.json` are records of the
old pictures now, not pins (R3331002), each holding every card with no recipe to its old hash and each card
with one to a new picture. A set's pass adds its cards' recipes from its own per-card brief table, takes them
off the detail floor's short list, and tunes them by eye on `scene-proofs.cy.tsx`'s kind of screenshot, at
hand size and at board size.

## Classic+ drawn as scenes (R3390000)

Classic+ is the first set whose every entry draws from a recipe of its own: its 78 cards and 38 tokens, each
from its brief (#3390's table), in Classic's temple-stone look, base and Radiant. The recipes are three files in
id order, spread into `SCENE_RECIPES` after the proofs: `classicPlusRecipes1.ts` (#1 to #26 with #12's and
#19's tokens), `classicPlusRecipes2.ts` (#27 to #52 with #32's, #36's, #38's, #42's and #46's) and
`classicPlusRecipes3.ts` (#53 to #78, #65's, #73's, #75's and #76's tokens and T-AI-1 to 10). Each keeps its
brief's subject, setting and idea, its subject within x 30 to 70 and y 25 to 85, its foreground near, its middle
in the mid and subject layers and its background far, and stands its emblem small (size 8 at most) inside the
band every face shows.

Each humanoid Unit is a figure whose look starts from its tag's people and is dressed as who it is, no two
alike; the Second Amendment Snake, the Book Worm, Jlockeed's J15 Fighter and the Mulch Muncher are no person
and draw a creature or a machine of the scene kit as their subject. Crowds, companions and rivals are `extras`,
figures in their own layers (League of Losers' five, the Flagbearer's cat warriors, KY's students). The full-art
cards whose motions move their props (R2715001) mark the placements that move with them (`props`): banners,
coins, gears, weights, relics. The parts the briefs named joined the kits: the figure kit's new poses, heads,
headgear, outfits and items, and the scene kit's new terrain, buildings and furniture (`sceneKitPlaces.ts`),
objects (`sceneKitObjects.ts`) and living things, food and weather (`sceneKitNature.ts`), the three drawing
with the first kit's helpers (`kitDraw.ts`). Every entry clears its rarity's floor and is off the short list,
whose cap fell from 561 to 445; `classicPlusScenes.test.ts` holds the set to all of it, and
`classicplus-scenes.cy.tsx` shows every Epic, Legendary and Mythic entry at hand size and the rest as a contact
sheet, at desktop size and at 390x844.

## Core's scenes (R3392000)

Every Core entry but Ceaseless Void, whose picture #3338 owns, draws a scene recipe of its own, written from its
brief in #3392's table: `sceneRecipesCore1.ts` holds Core #1 to #37, `sceneRecipesCore2.ts` #38 to #74 with KY's
Empty Notebook and Spikey Pillow, and `sceneRecipesCore3.ts` #75 to #99 with the other tokens, all spread into
`SCENE_RECIPES`. Each recipe's doc comment gives its brief in a line and any detail changed to fit the kits; the
subject, the setting and the card's idea stay, and a part a brief named that the kits lacked joined them. Each
scene clears its rarity's floor (below) with room, its median detail climbing rarity by rarity: a Common's one
subject in a simple setting, a Rare's props, an Epic's far, middle and near ground, a Legendary's every layer with
its light and its moment of action, and a Mythic's set piece with a part no other scene draws (My Pawn's gloved
hand on its puppet strings, Zephrys's cyclone, Heroic Power's rotunda, Craft a Card's floating island).

A Unit's character comes from the kits, never the painter's silhouette: a figure, its look its tags' look
(`figureLookFor`) with the head, headgear, outfit and item its brief names, or, when its character does not stand
like a person (Jewelosco Scarab, Carnivorous Cube, Prem Panther, Straaza, Spikey Pillow, The Rock, the Sheep, Bread
and Rush Tokens), a creature part drawn for it in its subject layer. A scene's other people are its `extras`
(R3390000). KY wears one look on every Core card that draws him (`KY_TUTOR_LOOK`: spectacles, a cardigan, a stick of chalk). A
scene draws no motif glyph or accent, so each full-art Core card whose motion moves its props (R2715001) marks the
placements that move on that beat `props` (R3390000), its motion unchanged. The parts the briefs named joined the
kits after the old ones of their kinds, in `sceneKit.ts` and `figureKit.ts`; where Classic+'s pass had already
taken a name for a part of its own, Core's drawing took another (`sea` beside `water`, `playing-card` beside
`card`, `pose-stride` beside `pose-run`). Every Core entry is off the short list, whose cap fell from 445 to 337.
`coreScenes.test.ts` holds the recipes to this, and
`core-scenes.cy.tsx` draws every Epic, Legendary and Mythic scene at hand size and a contact sheet of the rest, at
a desktop's size and a phone's.

## Classic+'s Alt art restaged (R3396000)

Each of the 40 Classic+ cards offered Alt art draws an alt recipe of its own, written from #3396's brief for it:
the same subject and idea in another place, time or story moment. The recipes are two files in id order,
`classicPlusAltRecipes1.ts` (#2 to #36) and `classicPlusAltRecipes2.ts` (#37 to #78), spread into
`ALT_SCENE_RECIPES` (sceneRecipes.ts). `sceneRecipeOf` reads one under the card's Alt art seed (`<id>#alt`,
`SCENE_ALT_PICTURE`) alone: the card's id, a repaint's seed and any other seed keep the normal recipe, and every
level of Alt art draws the one alt picture (R3015002). Each recipe keeps these rules:

- **Place and palette.** Classic's family, the brief's place, time and palette, a sky sharing no colour with the
  normal recipe's and a light that suits the hour.
- **The same subject.** A Unit drawn as a figure keeps one; a figure both pictures draw is the normal one's person
  (`normalLookOf`: its head and outfit kept, only its cloth, headgear or held item changed); League of Losers keeps
  its five (`normalExtraLookOf`); the J15 Fighter stays the jet; a subject that is a part stays in the subject layer.
- **A new camera.** A close-up scales the subject up, a figure to about 1.6 to 1.9 standing below the window; a wide
  or distant view shrinks it, a figure to about 0.6 or less; a view from above lays ground across the box under small
  subjects. Either way the subject (its figure, else its subject layer's first part) moves at least 8 units, changes
  scale by at least a quarter, or a figure comes or goes.
- **Detail, emblem and props.** The rarity's floor and the byte ceiling (R3331001) on both faces, the emblem small
  inside the band every face shows, no props in the subject layer, and the props the full-art motions move
  (League of Losers, Felinor Flagbearer, J-lease J-Jungle EX-plorer) marked outside it.

The briefs' places added six parts: a boat, a tent and a waterfall (`sceneKitPlaces.ts`), and bamboo, a cactus and a
fox (`sceneKitNature.ts`). `ALT_PICTURE_CHANGE_MIN` (0.6) is the least share of an Alt art window's sampled points
that differ from the normal window, base and Radiant, sampled on card-versions.cy.tsx's grid and counted with its
threshold for a point that differs. `classicPlusAltScenes.test.tsx` holds every recipe to the rules and every Alt art
level's face to its picture and its Radiant marks, and `classicplus-alt-scenes.cy.tsx` measures each window's share
and screenshots each normal face beside its Alt art face.

## Core's Alt art restaged (R3398000)

Each of the 61 Core cards offered Alt art draws an alt recipe of its own, written from #3398's brief for it, under
Classic+'s rules above: the same subject and idea in another place, time or story moment, in Core's family. The
recipes are three files in id order, `coreAltRecipes1.ts` (#3 to #35), `coreAltRecipes2.ts` (#37 to #65) and
`coreAltRecipes3.ts` (#68 to #100), spread into `ALT_SCENE_RECIPES` after Classic+'s, and read as theirs are, under the
card's Alt art seed alone, at every level of Alt art. A figure both pictures draw is the normal one's person
(`normalCoreLookOf`), and Duplicating Felinors, Carnivorous Cube and True Strike keep their people
(`normalCoreExtraLookOf`). Where a brief changes who is in the picture, an extra figure stands for the new person
(Conjure KY's students, Masochism Mask's wanderer, Unlicensed Experimentation's golem, Combo-Index's cleaner,
Collateral Damage's inspector). Ceaseless Void's alt recipe is the void before it wakes: its swirl and a black pearl
in a tide pool, drawn from the kits, with no giant, so `ownSubject` stays its normal recipe's alone (R3338001).

The briefs' places added ten parts: a palm and a camel (`sceneKitNature.ts`), a crater and a van
(`sceneKitPlaces.ts`; the crater is `crater-field`, apart from R3397000's smoking `crater`), and a hay bale, a picnic
blanket, a rug, a hammock, a sleeping bag and a teapot (`sceneKitObjects.ts`; the blanket is `gingham-blanket`, apart
from R3397000's `picnic-blanket`, the hammock `beach-hammock`, apart from the Legacy touches' `hammock`, and the teapot
`pouring-teapot`, apart from the Meditative ink kit's `teapot`); a brief's smaller details stand in with parts the
kits hold, each named in its recipe's doc comment, and the sleepers sit.
`coreAltScenes.test.tsx` holds every recipe to the rules and every Alt art level's face to its picture and its
Radiant marks, and `core-alt-scenes.cy.tsx` measures each window's share against `ALT_PICTURE_CHANGE_MIN` and
screenshots each normal face beside its Alt art face.

## Meditative+ and Training Grounds drawn as scenes (R3393000, R3393001)

Every Meditative+ entry (#1 to #26 and their seven tokens) and every Training Grounds entry (#1 to #60 and the
Golem token) draws from a recipe of its own, each from its brief (#3393's table), base and Radiant. The recipes
are in id order in three tables spread into `SCENE_RECIPES` after the proofs: `sceneRecipesMeditativePlus.ts`
(#1 to #11, spreading `sceneRecipesMeditativePlus2.ts`'s #12 to #26), `sceneRecipesTrainingGrounds.ts` (#1 to
#40, spreading `sceneRecipesTrainingGrounds2.ts`'s Units from #21 on and #39's Golem token) and
`sceneRecipesLegacy.ts` (#41 to #60), each set's table split in two files as the Classic+ pass split its own.
Meditative+ paints in the Meditative family's ink wash: layered `peaks` in mist behind, a high and distant
camera, and exactly one red `seal` in the near layer, in a lower corner inside the band a face's window
shows (y 18 to 82 of the box), so every face shows it. Training Grounds paints in carved wood: the yard's
`palisade` and `sand` wherever its brief is set in the yard, the sun from the upper right but at night. Each
keeps its brief's subject, setting and idea, its subject within x 30 to 70 and y 25 to 85, and stands its emblem
small on its subject.

Every Unit is a figure from the figure kit in a look built from its tags' with its brief's own parts, never the
hooded traveller; a Unit that is a thing is built from figure parts too: the living potions a flask's body with
eyes (`outfit-flask`, `head-eyes`) on stubby legs, Flame Tongue Totem, Floodgates and the Training Dummy on a
post (`pose-post`), the Golem of lashed posts with a sack head, the Shredder on treads. A brief's other
characters are `extras`, and a ghost, a copy or a flicker a `fade`. The parts the briefs named joined the kits,
in modules of their own beside the first: the mountains' 38 (`sceneKitMountain.ts`), the yard's 35
(`sceneKitYard.ts`) and the Legacy cards' 14 yard touches and props (`sceneKitTouches.ts`), and the figure kit's
four new poses, three heads, six hairs and headgears, eight outfits and ten held items (its lab coat and club
Core's, `outfit-labcoat` and `item-club`, its tree-trunk club `item-trunk-club`). A part named as the kit names
one already is named for what sets it apart, so a module's spread never replaces it: the mountains' camp tent
and waterfall are `camp-tent` and `cascade`, apart from the Alt art scenes' `tent` and `waterfall`, and
`ink-pine`, `mossy-boulder`, `lacquer-pavilion`, `steam-curl`, `wood-ladder`, `yard-sand`, `yard-well`,
`smithy`, `plank-bench`, `pale-bean`, `bell-pole`, `target-board` and `hypno-bunny` stand apart from Core's
`pine`, `boulder`, `pavilion`, `steam`, `ladder`, `sand`, `well`, `forge`, `bench`, `jelly-bean`, `bell`,
`target` and `bunny`.

A Legacy card (R3393001) draws its own brief's scene and places its yard touch (`LEGACY_TOUCHES`) once, as the
kit part `LEGACY_TOUCH_PARTS` names; its face wears its Core card's frozen emblem, the glyph and paint of the
picture its Core card drew before any recipe, so the two still read as one card across the sets. It is still
one picture (no motion, no parts, no seed of its own) and never draws its Core card's scene, which every
Legacy card's Core card has since Core's pass (R3392000). The yard painter
(`traininggrounds.ts`) and `legacySvg` now draw only a card with no recipe. Every entry clears its rarity's floor
and is off the short list, whose cap fell from 337 to 289; `setScenes.test.ts` holds the two sets to all of it,
and `set-scenes.cy.tsx` shows every Epic, Legendary and Mythic entry at hand size and the rest as a contact
sheet, at desktop size and at 390x844.

## Meditative drawn as scenes (R3401000)

Meditative's pass draws every Meditative entry from a recipe of its own: its 102 cards and 30 tokens, each from its
brief (#3401's table), in the art bible's ink-wash look, base and Radiant. The recipes are five files in id order,
spread into `SCENE_RECIPES` after Classic+'s: `meditativeRecipes1.ts` (#1 to #14), `meditativeRecipes2.ts` (#15 to
#30 with #19's, #22's, #28's and #30's tokens), `meditativeRecipes3.ts` (#31 to #49 with #39's, #45's and #49's),
`meditativeRecipes4.ts` (#50 to #79 with #70's and #71's) and `meditativeRecipes5.ts` (#80 to #102 with #91's, #93's,
#95's, #96's and #97's). Each keeps its brief's subject, setting and idea and Classic+'s rules (its main subject within x 30
to 70 and y 25 to 85, its foreground near, its middle in the mid and subject layers, its background far, its emblem
small on its subject inside the band every face shows), and the family's: pale paper and mist skies or the brief's
hour in soft washes, ink-wash peaks and cloud behind, a high and distant camera that sees a small subject in a large
space, and **one red seal** (`ink-seal`, a red chop of abstract strokes with no glyph) in the near layer of every
picture, unrepainted, where a painter would stamp a scroll.

Each of the 43 Units that are a person is a figure whose look starts from its tag's people and is dressed as who it is,
no two alike; the 21 that are no person draw their part of the scene kit as their subject, with no figure: the
rhino, the clockwork sentry, the pea vine, the Shade-iris, the taotie, Yile's Virus, Devin Bot, the Sentient Cat
Ears, the Montaña Giant, Volatility's jar, Tatches the Totem, the True Sheep and the nine buildings of Jlockeed's
Evil Blueprints. The people round a subject are small brush figures (`ink-person`), a rival or companion who matters
an extra figure, and a foreground hand a hand of the kit (`hand-open`, `hand-hold`). API Key Fishing and Tatches the
Totem, whose full-art motions move their props (R2715001), mark the near-layer placements that move with them
(`props`). The parts the briefs named joined the kits: the figure kit's new headgear (headphones, a beanie, a
pompadour, a conductor's cap, a tall hat, a topknot), outfits (overalls, a kaftan, a tailcoat, wind-blown silks)
and held items (a lantern, a phone, a key, a baton, a brush, a loupe and torch, a fan, twin blades, a fishing rod, a comb), and eight scenery
modules beside Classic+'s, drawing with the same helpers (`kitDraw.ts`): ink-world terrain and architecture
(`sceneKitInkPlaces.ts`: peaks, terraces, still water, pagodas, pavilions, gates, wells), buildings
(`sceneKitInkBuildings.ts`), objects (`sceneKitInkObjects.ts`: the seal, paper lanterns, brushes, an abacus, a go
board), curios (`sceneKitInkCurios.ts`: jade, a bottle, a horned skull, a melon, drifting hearts, a torn ticket stub), machines
(`sceneKitInkMachines.ts`), plants and weather (`sceneKitInkNature.ts`: the pea vine, lotus, plum blossom, a great
tree, a trellis, ink cloud, a rift in the air, thought-shapes), creatures, people and hands
(`sceneKitInkCreatures.ts`) and the Units' characters (`sceneKitInkCharacters.ts`). Where an earlier pass had
already taken a name, the ink part took another (`ink-sheep` beside `sheep`, `ink-pavilion` beside `pavilion`,
`ink-card-fan` beside `card-fan`, `temple-bell`, `envelope-drift`, `booster-sack`, `peak-fortress`), so no old
picture changes; `ink-seal`, `ink-round-window` and the figure kit's `item-ink-brush` keep clear of the names
Meditative+'s and Training Grounds' pass (#3438) gave its own. Every entry clears its rarity's floor and is off the short list, whose cap fell from 289 to 157;
`meditativeScenes.test.ts` holds the set to all of it, and `meditative-scenes.cy.tsx` shows every Epic, Legendary
and Mythic entry at hand size and the rest as a contact sheet, at desktop size and at 390x844.

## Stinkyscape's scenes (R3391000)

Each of the 64 Stinkyscape cards of #3391's table draws from a recipe of its own (`stinkyscapeRecipes.ts`,
`STINKYSCAPE_RECIPES`, spread into `SCENE_RECIPES`), drawn from its brief: the subject, the setting and the idea
the brief names, a part it named that the kit lacked added to the kit. A person is the figure kit's, its look
starting from its tag's (a Human's head, a Felinor's) and dressed for the card; a ship Unit's character is its own
craft part in the subject layer (Stratos' boxy fighter, Falcon's needle, Paragon-X's blade, Ozark's barge), and
Imprisoned Cosmality is its creature. The bible's palette is the set's base, and a brief's own colours are drawn
in palette.css's nearest names (pink `red-300` and `purple-400`, teal `cyan-400` or `jade-400`, crimson `red-600`
or `red-900`, rust `wood-700`, amber `orange-400`). Each card clears its rarity's floor, the median rising from
Common to Mythic (Commons about 40 drawn elements, Rares about 60, Epics about 85, Legendaries about 115 and
Mythics 150 to 160), and each Mythic places a part no other card does: Tylannt's skiff, Rod of Ages' rod, Drone
Core X's battleship, Great Paperclip Theory's wire, Paragon-X's blade fighter and Prototype-X2's fused hull. The
Starship Pieces, the Dormant Starship (#3333) and Olivian Expanse (#3338) keep their pictures.
`stinkyscapeScenes.test.ts` holds the pass to its table, and `e2e/cypress/component/stinkyscape-scenes.cy.tsx`
shows it at hand size and on a contact sheet, at desktop size and at 390x844. Its further figures (a crew, a
squad, a rival on a leash) are R3390000's `extras`, and it wears the parts the passes before it added where a
brief names the same thing: Classic+'s spread and seated poses, goggles, lectern, shockwave, drum, playing card
and scales (R3390000), and Core's lab coat, work apron, anvil (a red-hot axe head of its own, `axe-head`, lying on
it) and conveyor (R3392000). A part of its own that looks like one of theirs draws something else or turns where
that one does not (a light stream, a light cone and a smoke trail that point and turn, a meteor, a column of
flame, a computer core, credit chips, lava rivers seen from high above, a gloved hand reaching up, a robot arm
whose tool bends down onto its work where Core's clamp grips upward), and where Core's pass had taken a name for
something else, its part took another (`head-wraith` beside Core's lantern head, `cracked-helmet` beside its
knight's helmet, `light-cone` beside its hanging spotlight). So did each part whose name a pass that landed
before it had taken, each named for what sets it apart, so no module's spread replaces it: beside #3403's parts
(R3403000) its `wedge-fighter`, `eye-drone`, `retro-rocket`, `welding-arm`, `reaching-hand`, `control-desk`,
`herb-cracker`, `ship-deck`, `moon-surface`, `riveted-porthole`, `reactor-core`, `ice-cube` and `comet-streak`
(the comet that turns), and the figure kit's `hair-professor` beside its `hair-silver` (its bubble helmet is
`space-helmet`, so the cracked one is `cracked-helmet`); beside the Meditative pass's (R3401000) its
`ring-station` and `ringed-rift`; and beside the Alt art passes' rope (R3397000) its `swing-rope`.

## Stinkyscape's later cards drawn as scenes (R3403000)

Each of the 103 entries of #3403's table, Stinkyscape #91 to #176 and their tokens with the tokens of the early
cards it names (#8's, #10's, #11's, #12's, #18's and #84's), draws from a recipe of its own, from its brief, in
Stinkyscape's deep-space look, base and Radiant. The recipes are three files in id order, spread into
`SCENE_RECIPES` as `STINKYSCAPE_91_RECIPES` after Classic+'s: `stinkyscape91Recipes1.ts` (the early tokens and #91
to #119), `stinkyscape91Recipes2.ts` (#120 to #159 with #135's, #137's, #147's, #148's and #150's tokens) and
`stinkyscape91Recipes3.ts` (#160 to #176, the drones, the resources and T-AI-1 to 9). Each keeps its brief's
subject, setting and idea, its subject within x 30 to 70 and y 25 to 85, and its emblem small inside the band every
face shows. The cards the table leaves to other issues (every Starship Piece and the three Jlockeed weapons that
are Pieces when Embiggened, the Dormant Starship, the Drone Battleship's layers, Anomaly Found!, the Terminus tokens
and Dice of Doom) have no recipe and draw as before. R3406000 gives those of them #3406's table names an alt recipe,
their normal faces unchanged (below).

Each humanoid Unit is a figure dressed as who it is, no two alike; fourteen Units are no person and draw a creature
or a machine of the kit as their subject: the Wasp and the Hornet, Megabase, the Sack, the Stinkyscape Golem, the
four Asteroids, the Spice Tower, the Drone Interceptor and Corvette, Escort Protocol and Combat Subroutine. Crews,
crowds and rivals are `extras`. The parts the briefs named joined the kits: the figure kit's space headgear (a
neural crown, a bubble helmet, silver hair, a top hat), its flight suit, greatcoat, sleeveless vest,
plated parka, frock coat, cloak and harness, and its shock-axe, dart, orb, slate, torch and vial, after #3401's; and
four scene-kit modules of space beside #3390's, #3393's and #3401's, drawing with `kitDraw.ts` and spread into `SCENE_KIT`:
`sceneKitSpace.ts` (decks, bulkheads, regolith, craters, ducts, a planet's limb, stations, gates, pads, windows and
their fittings), `sceneKitCraft.ts` (starships, fighters, freighters, shuttles, rockets, probes, drones, robots,
mechs, a wasp, a golem, an alien, a crew), `sceneKitGear.ts` (consoles, a diagram of light, cables, ore, wreckage,
credit chips, a roulette table, capsules, a kicking sack, pages, a blueprint, a reset lever, a drill, the chaos egg,
and the beams, bolts, lasers, walls, plumes and flashes of energy) and `sceneKitCosmos.ts` (comets, frost and ice, a
planet's ring, alarms, streaks, waves, flares, wormholes, rifts, galaxies, stink clouds). The pointing pose and the
hoodie are Core's and the headphones Meditative's; a part named as the kit names one already is named for what sets it apart (`outfit-sleeveless`,
`outfit-plated-parka`, `item-shock-axe`, `obs-bench`, `lamp-tent`, `ticker-tape`, `speed-streaks`, `surf-wave`,
`crew-crowd`, `kicking-sack`, beside the Meditative pass's crane, rift and blueprint `shipyard-crane`, `space-rift`
and `holo-blueprint`, and beside the Alt art pass's smoking crater `bowl-crater`), so no module's spread replaces another's. Every entry clears its rarity's floor and is
off the short list, whose cap fell from 157 to 129; `stinkyscape91Scenes.test.ts` holds the pass to all of it, and
`stinkyscape91-scenes.cy.tsx` shows every Epic, Legendary and Mythic entry at hand size and the rest as a contact
sheet, at desktop size and at 390x844.

## Meditative+ and Training Grounds Alt art restaged (R3397000)

Each of the 36 Meditative+ and Training Grounds cards offered Alt art (14 and 22) draws an alt recipe of its own,
written from #3397's brief for it, under Classic+'s rules (above): the same subject and idea in another place, time
or story moment, read under the card's Alt art seed alone, one picture at every Alt art level. The recipes are
`meditativePlusAltRecipes.ts` and `trainingGroundsAltRecipes.ts`, spread into `ALT_SCENE_RECIPES` after Classic+'s.
A Training Grounds card's painter (`trainingGroundsArtUri`) reads its scene under the seed it is given, so its Alt
art level draws the alt recipe as its normal face draws its own.

- **The family.** A Meditative+ alt keeps the ink line and exactly one red `seal` in the near layer, where the
  normal picture stamps it, inside the band every face shows; it drops the ink-wash peaks where its brief leaves the
  mountains (under a lake, in bamboo, on a picnic blanket). A Training Grounds alt keeps the carved-wood line and,
  as its normal picture, no seal, and leaves the yard wherever its brief does.
- **The same subject.** A figure both pictures draw is the normal one's person (`normalLookOf`); a person the brief
  keeps beside it is the extra at the same index in both pictures, with the normal one's head and outfit
  (`normalExtraLookOf`: Bloodbloom's mage, Dogpile's foe, Group leader's column, Right-Hand Felinor's knight,
  Seance's recruits, Tutorial Master's recruit, Call to Chaos's instructor); a subject that is a part stays in the
  subject layer.
- **Camera and detail.** Classic+'s new camera (the subject moved, rescaled or a figure come or gone); the rarity's
  floor and the byte ceiling on both faces, far, mid, subject and near layers from Epic up, and Goetzmann Game
  Design, the one with full-art motion, a layer for each role it moves.

A detail the kit lacks is left out and named in the recipe's comment, never drawn as another part. The briefs added
thirteen parts: a goat, a deer, a duck and walnuts (`sceneKitNature.ts`); a crater, a sandcastle, a snowman, a
water wheel and a rowboat (`sceneKitPlaces.ts`, the rowboat apart from the sailing `boat`); and a cork board, a
washing line, a picnic blanket and a hanging rope (`sceneKitObjects.ts`). KY's Competition shares #3401's split
`melon`. `setAltScenes.test.tsx` holds every recipe to the rules and every Alt art level's face to its picture and
its Radiant marks, and `set-alt-scenes.cy.tsx` measures each window against `ALT_PICTURE_CHANGE_MIN` and screenshots
each normal face beside its Alt art face.

## Meditative's Alt art restaged (R3404000)

Each of the 54 Meditative cards offered Alt art draws an alt recipe of its own, written from #3404's brief for it:
the same subject and idea in another place, time or story moment, as Classic+'s pass did (R3396000). The recipes are
three files in id order, `meditativeAltRecipes1.ts` (#1 to #38), `meditativeAltRecipes2.ts` (#40 to #71) and
`meditativeAltRecipes3.ts` (#72 to #100), spread into `ALT_SCENE_RECIPES` after Classic+'s and read under the card's
Alt art seed alone (`sceneRecipeOf`), at every level of Alt art. Each recipe keeps Classic+'s rules and the family's:

- **Place and palette.** Meditative's family and brush line, the brief's place, time and palette, a sky sharing no
  colour with the normal recipe's and none of Radiant's gold window (no `gold-*`, `yellow-400` or `parchment`).
- **One red seal.** Exactly one `ink-seal` in the near layer, unrepainted, inside the band every face shows, as
  every Meditative picture stamps (R3401000).
- **The same subject.** A Unit drawn as a figure keeps one; a figure both pictures draw is the normal one's person
  (`meditativeLookOf`: its head, outfit and skin kept, only its cloth, headgear or held item changed), and a
  companion it keeps is the normal picture's (`meditativeExtraLookOf`); a subject that is a part (the melon, the
  abacus, the pea vine, the taotie, the totem and the rest the test names) stays in the subject layer, placed first.
- **A new camera.** A close-up scales the subject up, a figure to 1.6 to 1.9 standing below the window (y 112 to 118)
  so its head and shoulders fill it; a wide or distant view shrinks it, a figure to about 0.6 or less; a view from
  above lays the floor, the sand or the felt across the box. The subject moves at least 8 units, changes scale by at
  least a quarter, or a figure comes or goes.
- **Detail, emblem and props.** The rarity's floor and the byte ceiling (R3331001) on both faces (Epic and rarer
  drawing far, mid, subject and near), the emblem small inside the band, no props in the subject layer, and the props
  API Key Fishing's and Tatches the Totem's full-art motions move marked outside it.

The briefs added three parts: a fisherman's oar lying level and a sofa (`sceneKitInkObjects.ts`), and a tower crane
(`sceneKitInkMachines.ts`). `meditativeAltScenes.test.tsx` holds every recipe to the rules and every Alt art level's
face to its picture and its Radiant marks, and `meditative-alt-scenes.cy.tsx` measures each window's share against
`ALT_PICTURE_CHANGE_MIN` (0.6) and screenshots each normal face beside its Alt art face.

## Stinkyscape's later Alt art restaged (R3406000)

Each of the 54 cards of #3406's table, every Stinkyscape card from #91 on and every Stinkyscape token offered Alt art
but the Dormant Starship, draws an alt recipe of its own, written from #3406's brief for it, under Classic+'s rules
(above): the same subject and idea in another place, time or story moment, read under the card's Alt art seed alone,
one picture at every Alt art level. The recipes are three files in id order, spread into `ALT_SCENE_RECIPES` as
`STINKYSCAPE_91_ALT_RECIPES` after the other sets': `stinkyscape91AltRecipes1.ts` (#93 to #115, with
`normalLookOf` and `normalExtraLookOf`, read from #3403's recipe files so no import goes round),
`stinkyscape91AltRecipes2.ts` (#116 to #146) and `stinkyscape91AltRecipes3.ts` (#147 to #177).

- **A piece of its own painter.** Eighteen of the 54 have no normal recipe: the fourteen Starship Pieces, Jlockeed's
  Siege Tank, Yamato Cannon and Missile Pod, and Dice of Doom. Their normal face and every seed but the Alt art one
  keep their own painter's picture; the Stinkyscape painter tries the scene first (svg.ts `sceneArtSvg`), so their
  Alt art seed alone finds the alt recipe. Each stands its piece in its subject layer as a part of the scene kit,
  under a sky that is neither `black` nor `slate-900`.
- **The same subject.** A figure both pictures draw is the normal one's person (`normalLookOf`; only its item, cloth
  or hair may change); a person the brief keeps is the extra at the same index (`normalExtraLookOf`: Scornboi's
  aide, ETHICS's members); a subject that is a part stays in the subject layer; Pocket Peptides' held vial stands on
  the table as a `vial`; and Megabase and Huge Asteroid, seen from inside, stand an `escalator` and a `reactor` there.
- **Camera and detail.** Classic+'s new camera and new sky for the 36 with a normal recipe; the rarity's floor and
  the byte ceiling on both faces, far, mid, subject and near layers from Epic up, and Dice of Doom, the one with
  full-art motion, a layer for each role it moves.

A detail the kit lacks is left out and named in the recipe's comment, never drawn as another part (Vxyl's folded
cloak, A Very Dandy Lad's parasol, Arctic Citadel's parka at his waist, the stars on the wizard's robe, the
Commander's chin on her fist, No-No Words' hand over the mouth, IN33DH3's crossed legs, Core System's dust sheets),
and ETHICS's papers are scrolls under every other member's arm, so six do not take its picture past the byte ceiling. Fifteen parts joined the kit in a module of their own, `sceneKitStinkyAlt.ts`, spread
into `SCENE_KIT` after every other: the pieces (a nanobot swarm, a hull plate, a cartridge, an overdrive engine, a
warp ring, a rail gun, a great cannon, a missile pod, a capacitor cell, a flank pod, a launcher tube and a
hundred-sided die with no digits), a pineapple, an escalator and a glowing seashell. `stinkyscape91AltScenes.test.tsx`
holds every recipe to the rules and every Alt art level's face to its picture and its Radiant marks, and
`stinkyscape91-alt-scenes.cy.tsx` measures each window against `ALT_PICTURE_CHANGE_MIN` and screenshots each normal
face beside its Alt art face.

## Classic's scenes (R3352000)

Classic's pass draws every Classic entry from a recipe of its own, each from its brief (#3352's table), in the art
bible's temple-stone look, base and Radiant: Classic #1 to #73, #75 to #90 and the Glitch, while Corpse Plantation
keeps its proof recipe (above). The recipes are three files in id order, spread into `SCENE_RECIPES` after
Meditative's: `sceneRecipesClassic1.ts` (#1 to #30), `sceneRecipesClassic2.ts` (#31 to #60) and
`sceneRecipesClassic3.ts` (#61 to #90 and the Glitch). Each keeps its brief's subject, setting and idea and
Classic+'s rules (its main subject within x 30 to 70 and y 25 to 85, its emblem small on its subject), draws one
geometry on both faces, and grows richer with its rarity: the median element count climbs from Common to Mythic,
and In Too Deep, the Mythic, draws a part no other scene draws, its spiralling shaft (`spiral-shaft`).

Each of 28 Units is a figure whose look starts from its tags' and is dressed as who it is, no two alike and none
the hooded traveller; Paul Allen's Ghost is one of them, a void head in hood and rags inside a shield bubble. The
six that are no person draw their creature part of the scene kit as their subject, with no figure:
Turtinator, Pestilent Slime, Hired Shrimp, Plague Crawler, Plague Charger and Lane Eater. A scene's other people are
its `extras` (R3390000). The Glitch draws a recipe though its card's window shows no picture;
`classic-scenes.cy.tsx`'s contact sheet shows it. In Too Deep, whose full-art motion moves its props (R2715001), marks its
ladders and hanging ropes `props`, its motion unchanged.

The parts the briefs named joined the kits: the figure kit's crouch (`pose-crouch`), three heads (`head-sheep`,
`head-ape`, `head-nose`), five headgears (`jester-cap`, `cat-hood`, `beak-mask`, `king-crown`, `jagged-crown`), three
outfits (`outfit-harlequin`, `outfit-travel-cloak`, `outfit-spider`) and six held items (`item-card-fan`,
`item-cleaver`, `item-abacus`, `item-cookie`, `item-grapple`, `item-rake`), after the old ones of their kinds in
`figureKit.ts`, and eighteen scenery parts in a module of their own, `sceneKitClassic.ts` (its props in
`sceneKitClassicProps.ts`), spread into `SCENE_KIT` after Stinkyscape's, with only the Alt art passes' modules after
it, Classic's (below) and then the Stinkyscape one's (above): the creatures (`tortoise`, `slime`, `shrimp`,
`crawler`, `maw`, `skeleton`, `lizard`, `leaping-deer`, `mule`), the objects (`pickle`, `sceptre`, `spike-trap`,
`tesla-coil`, `web`, `vat`, `chalice`), a
`blast-crater`, and In Too Deep's `spiral-shaft`, none taking an old part's name (the deer and the crater beside
R3397000's `deer` and `crater`, the cloak beside R3403000's `outfit-cloak`, above). Every Classic entry is off the
short list, whose cap fell from 129 to 10 (Stinkyscape's first pass, #3391, had taken 30 off and left the cap at
129); `classicScenes.test.ts` holds the recipes to all of it, and `classic-scenes.cy.tsx` shows every Epic,
Legendary and Mythic scene at hand size, base and Radiant, and a contact sheet of the rest, at desktop size and at
390x844.

## Classic's Alt art restaged (R3388000)

Each of the 50 Classic cards offered Alt art draws an alt recipe of its own, written from #3388's brief for it, under
Classic+'s rules (above), but with both a new sky and a new camera, never one alone: the same subject and idea in
another place, time or story moment, in Classic's family, read under the card's Alt art seed alone, one picture at
every Alt art level. Corpse Plantation is one of them: its normal face keeps the proof scene, and its Alt art is the
plantation at harvest noon. The recipes are three files in id order, spread into `ALT_SCENE_RECIPES` after the other
sets': `classicAltRecipes1.ts` (#1 to #26, with `normalClassicLookOf` and `normalClassicExtraLookOf`, read from
Classic's three recipe files so no import goes round), `classicAltRecipes2.ts` (#27 to #54) and
`classicAltRecipes3.ts` (#55 to #90).

- **The same subject.** A figure both pictures draw is the normal one's person (`normalClassicLookOf`; only its
  cloth, headgear or held item may change); Felinor Feelings, Ancient Acquisition, Nature Titan, Common Resources and
  Pile On keep their people, the extras at the same index (`normalClassicExtraLookOf`); a new person is a new extra
  (Shadowstep's merchant, Anti-Greed Machine's treasure hunter, Corpse Plantation's farmhands, King Wagtoggle's
  shepherd); and a subject that is a part stays in the subject layer. A close-up of hands or of an object draws no
  figure: Book of Blood, Prep, Final Gambit and Plague Nuke draw open hands, Flame Lance the fiery tip through a
  shield, and In Too Deep only the shaft's mouth, its adventurer gone down it.
- **Camera and detail.** A head-and-shoulders close-up stands its figure at scale 1.6 to 1.9 low in the window (Boots
  on the Ground's ground-level shot larger still), a wide view at 0.6 or less; the rarity's floor and the byte ceiling
  on both faces, far, mid, subject and near layers from Epic up, all seven from Legendary up; and In Too Deep, the one
  with full-art motion, marks its rope and packs `props`.

A detail the kit lacks is drawn with a part it holds, named in the recipe's comment: cyan rays from the Palantir's orb
for its beam, a reaching hand holding the InfiniScepter, lanes for Pickle's paths, a small void swirl for Exile's
portal, a red road and white trees for Book of Blood's stream and birches, a black ghost figure for Shadowstep's
shadow, violet flowers for lavender and crocuses, a mesa for Counterspell's sea stack, three steam curls for Lizard's
Breath's wisps, a temple for the tribunal, coral-painted bushes and ferns for Turtinator's reef, a snowfield for Rapid
Draw's roof, a cascade for the alpine stream, an awning for the umbrella, sparks for Joro's silk (he hangs upright: a
figure cannot be turned), a space helmet for the diver's, a big crate for the auction block, the vat spilling for the
lab's outflow, grey spores for ash, a cargo net for the fisher's net, a cot and a set-down helmet for Hired Shrimp's
bunk, steam for breath, boards for the pier, ice shards for icicles, a boat for Living Bomb's barge and the riverboat
shrine, a statue and a boulder for broken golems and the glow chute for the recycler's hopper, brimmed hats and rakes
for straw hats and scythes, a sack for the souvenir, rows of whitewashed houses for Lockdown's city from above, and a
brown thread tied to a post and duffels for In Too Deep's rope and packs. Two parts joined the kit in a module of
their own, `sceneKitClassicAlt.ts`, spread into `SCENE_KIT` after Classic's own and before the Stinkyscape Alt art
pass's: a frog (Transmutable Toxins) and a shark (Turtinator). `classicAltScenes.test.tsx` holds every recipe to the
rules and every Alt art level's face to its picture and its Radiant marks, and `classic-alt-scenes.cy.tsx` measures
each window against `ALT_PICTURE_CHANGE_MIN` and screenshots each normal face beside its Alt art face.

## The detail floor (R3331001)

`artRichness.test.ts` measures every catalog entry's base picture as its face draws it (`richness.ts`): the
elements it draws after its `<defs>` (`artElements`), and for a scene the layers and the kit parts it uses.
Each rarity has a floor (`ART_DETAIL_MIN_<RARITY>`): Common 30 elements, Rare 40, Epic 60, Legendary 90 and
Mythic 140, Legendary and Mythic in every layer; a token is held to Common's. The Glitch is not measured: its
window draws no picture. The floor starts as a ratchet: `fixtures/art-detail-short.json`
(`ART_DETAIL_SHORT`) names, sorted, the cards under their floor as #3331 left them, exactly
`ART_DETAIL_SHORT_MAX` of them (a pass that takes cards off lowers the cap with them, #3613), and the test fails
on a card under its floor the list does not name and on a listed card that clears its floor, so a set's pass takes
its cards off as it lifts them, the list only shrinks, and the last pass empties it. A ceiling keeps phones fast:
no picture's data URI, base or Radiant, is longer than `ART_DETAIL_MAX_BYTES` (48 KiB), and the art cache
(`ART_CACHE_MAX`) and the lazy drawing of faces scrolled past (`near.ts`) are unchanged.
