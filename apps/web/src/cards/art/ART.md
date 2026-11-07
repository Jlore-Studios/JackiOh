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
recipe draws its scene instead (R3331000, below): Arg Arg Defender is the first.

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
(`LEGACY_TOUCHES`, R2724000); a delivered file is read by the Legacy card's own id.

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
  `aimOf`), the motes scatter and settle. Press: the subject flinches (`scale`) and springs back on the overshoot
  curve. `attackDeclared`: the subject swings on the attack's own `--fx-lunge-x/y` and the motes fire. All in
  `rotate`, `translate`, `scale` and `opacity`, inside the reduced-motion gate, with `motion.css`'s times and curves.
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
- **By rarity:** a Common is one figure in a field (Speciesist, on guard at dusk); a Rare stands among
  props, a fence, a cart, a banner; an Epic sets hills behind and grass before it; a Legendary lights its
  moment with rays and a halo; a Mythic is a set piece no other card shares (Heroic Power's shrine on the
  heights, Ceaseless Void's giant-shaped hole before the void among a world's ruins).

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
of placements. A placement names a part of the kit, where it stands, its scale and facing, how many copies
(`count`, each but the first offset by up to `spread`, every copy's scale jittered by `SCENE_SCALE_JITTER`)
and a palette name that repaints the part's main shapes. `composeScene` draws every part from one stream,
seeded by the card's id (or by a picture's own seed, `core-002#alt`, so Alt art and the repaints grow the same
parts apart), and never reads the face: the Radiant face draws the same shapes, each mixed toward its layer's
gold (`SCENE_RADIANT_GOLD`), with the painter's rays. The figure stands in front of its subject layer's own
parts. `withScene` lays the scene into the procedural painter's `ArtSpec` (`artSpec`'s last argument, read
from the card's id), so it prints, moves (each layer on its role's beat: `back`, `subject`, `over`, `light`,
`motes`), splits for Out of frame (the subject layer and the emblem) and restyles as every procedural
picture does, and keeps the card's theme, layout, backdrop, sky scheme, emblem glyph and motif. The
Stinkyscape and Training Grounds painters hand a recipe card to it (`svg.ts` `sceneArtSvg`), and a Legacy
card keeps its Core card's old subject.

The **scene kit** (`sceneKit.ts`, `SCENE_KIT`) holds the scenery, at least `SCENE_KIT_MIN_PER_CATEGORY` of
each kind: terrain (fields, furrows, grave-mounds, cliffs, floors, rooms, streets, hills, asteroids),
architecture (fences, posts, gates, ruins, towers, columns, arches, tombstones), plants, creatures, objects
and tools, and weather and light (moons, stars, clouds, mist, shafts, halos, rays, sparks, embers, motes, a
void, lightning, nebulae, planets, a rim light). The **figure kit** (`figureKit.ts`, `FIGURE_KIT`) replaces
the painter's one silhouette: three poses, five heads, eight hairs and headgears, nine outfits by tag and
five held items, put together by a look (`FIGURE_LOOKS`, `figureLookFor`), so a Unit reads as who it is: a
Terminus, a CN or KY character, a Felinor, a Pancake, a Fruit, Jlockeed's people or a Human. The two kits
hold at least `SCENE_KIT_MIN_PARTS` parts; each part draws one to twelve filled shapes, a line drawn as a
thin band.

Five proof cards draw from recipes today and set the bar each set's pass meets: Corpse Plantation (Classic
#74), Speciesist (Core #2), Heroic Power (Core #98), Ceaseless Void (Core #100) and Arg Arg Defender
(Stinkyscape #15). Ceaseless Void's recipe keeps its painter's own subject (`ownSubject`), the giant its
composition draws (R3338001), in place of a figure. Every other card draws its picture byte for byte as
before, and the golden fixtures stand for it: `core-art-v0.1.json`, `art-before-2724.json` and `stinkyscape-art-v0.5.0.json` are records of the
old pictures now, not pins (R3331002), each holding every card with no recipe to its old hash and each card
with one to a new picture. A set's pass adds its cards' recipes from its own per-card brief table, takes them
off the detail floor's short list, and tunes them by eye on `scene-proofs.cy.tsx`'s kind of screenshot, at
hand size and at board size.

## The detail floor (R3331001)

`artRichness.test.ts` measures every catalog entry's base picture as its face draws it (`richness.ts`): the
elements it draws after its `<defs>` (`artElements`), and for a scene the layers and the kit parts it uses.
Each rarity has a floor (`ART_DETAIL_MIN_<RARITY>`): Common 30 elements, Rare 40, Epic 60, Legendary 90 and
Mythic 140, Legendary and Mythic in every layer; a token is held to Common's. The Glitch is not measured: its
window draws no picture. The floor starts as a ratchet: `fixtures/art-detail-short.json`
(`ART_DETAIL_SHORT`) names, sorted, the cards under their floor as #3331 left them, at most
`ART_DETAIL_SHORT_MAX`, and the test fails on a card under its floor the list does not name and on a listed
card that clears its floor, so a set's pass takes its cards off as it lifts them, the list only shrinks, and
the last pass empties it. A ceiling keeps phones fast: no picture's data URI, base or Radiant, is longer than
`ART_DETAIL_MAX_BYTES` (48 KiB), and the art cache (`ART_CACHE_MAX`) and the lazy drawing of faces scrolled
past (`near.ts`) are unchanged.
