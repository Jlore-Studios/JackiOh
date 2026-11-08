# Recipe rules for the #3392 recipe helpers

JackiOh's web client paints card pictures as SVG scenes. A scene recipe (`apps/web/src/cards/art/scene.ts`,
`SceneRecipe`) is data per card: a sky, a light, an emblem, an optional figure, an optional `cast` of more
figures, and seven layers back to front (`sky`, `far`, `mid`, `subject`, `near`, `light`, `particles`), each a list
of placements of kit parts (`{ part, x, y, scale?, flip?, count?, spread?, paint? }`: `count` copies, each but the
first offset by up to `spread`; `paint` repaints the part's main shapes). Issue #3392 gives every Core card a scene
of its own, drawn from its brief. You write ONE file of them: `apps/web/src/cards/art/sceneRecipesCore<n>.ts` (its
skeleton exists: a header comment and an empty `CORE_SCENES_<n>` map). Your briefs are in
`zz-scratch/briefs-<n>.md`.

## Read first

- `apps/web/src/cards/art/scene.ts` (types, `composeScene`), `sceneRecipes.ts` (the proof recipes: copy their
  style), ART.md's "The art bible" → "Core" and "Scene recipes and kits" sections.
- `sceneKit.ts`: every scenery part, its doc comment giving its anchor and its size at scale 1 (the new ones sit
  between `// <<X ...>>` marker lines). `grep -n '^/\*\*\|^function\|^  [a-z"-]*: {' apps/web/src/cards/art/sceneKit.ts`
  lists them compactly. `figureKit.ts`: poses (`POSE_ANCHORS` gives each pose's neck, hand and hip), heads, hair,
  outfits, items, `FIGURE_LOOKS`, `figureLookFor`, `KY_TUTOR_LOOK`.

## Each recipe

- `const NAME_OF_CARD: SceneRecipe = { family: "core", sky, light, emblem, figure?, cast?, layers }`, one per card,
  named after it (`BIG_D_FENDER`, `FOUR_MANA_SEVEN_SEVEN`, `FULLSEND`, `MISS_MROW`), with a one-line doc comment:
  `/** Big D-fender (Core #1, Common Unit): <the brief in a line>. <Any change made to fit the kits, and why.> */`
  (a token: `(Core token, Spell)`). At the end: `export const CORE_SCENES_<n>: Readonly<Record<string,
  SceneRecipe>> = { "core-001": BIG_D_FENDER, ... };` in id order. Import `type SceneRecipe` from "./scene.ts" and
  what you use of `figureLookFor`, `KY_TUTOR_LOOK` from "./figureKit.ts". Keep the skeleton's header comment.
- `family: "core"`, never `ownSubject`. Core's look: a clear bright world as if through glass, clean flat shapes,
  blues and skies, cyan for power, field greens, slate-100 glints; `light` (the glow behind the subject) from the
  upper left unless the brief names its own source (a lamp, the moon, a forge, a spotlight, a portal). Skies: two
  palette names top to bottom, matching the brief's light (noon sky-400→blue-200; morning sky-300→slate-100; dusk
  blue-800→orange-400; night slate-900→blue-900; an interior a room pair, e.g. slate-100→stone-500). Never a gold
  name anywhere: gold-400, gold-300, gold-200, yellow-400, parchment.
- The subject (the figure or main thing) within x 30–70 and y 25–85: a face's window shows about y 18–82 of the
  100 box. A figure stands on its `y` (its feet), 56 tall at scale 1 (at y 86, scale 1, its head reaches y 30).
  Floors and horizons about y 60–75.
- `emblem: { x, y, size: 5–8, rotate: 0 }`: small, on a prop or a chest, never over a face (a figure's chest is
  about 28–32 × scale above its feet, its head 44–55 above).
- A Unit's character: `figure: { x, y, scale, pose, look }` with `look` built on `figureLookFor(<its tags>)`, e.g.
  `{ ...figureLookFor(["Human"]), hair: "helmet", outfit: "outfit-plate", item: "item-tower-shield", cloth:
  "slate-400" }`, or, where the briefs file says so, a creature part in the `subject` layer. A spell's people are
  figures too. More people go in `cast: [{ x, y, scale, pose, look, layer: "mid" }]` (drawn in front of that
  layer's parts). A figure costs ~20 shapes: a crowd is a `crowd` or `soldier` part, not twenty figures. A token's
  tags give the default look; build the look you need. KY is always exactly `look: KY_TUTOR_LOOK`.
- Detail by rarity, measured as the base face's `elements` (≈ your scene's shapes + ~5), every picture under 48
  KiB (~190 bytes a shape: stay under ~220 shapes):
  - Common and Token: 34–70 (floor 30): one clear subject and a simple setting in `mid`, maybe `particles`.
  - Rare: 46–90 (floor 40): the same plus props in `mid` or `near`.
  - Epic: 66–130 (floor 60): `far`, `mid` and `near` ALL used (the brief's background, middle, foreground).
  - Legendary: 96–180 (floor 90): ALL SEVEN layers used, rays or a halo in `light`, the moment in `particles`.
  - Mythic: 146–220 (floor 140): all seven layers, and its own unique part (never another Mythic's).
  Medians must climb Common < Rare < Epic < Legendary < Mythic: no rich Commons, no thin Epics.
- No two recipes alike: vary camera, horizon, sky and placement card to card.
- A part the kits lack that a brief names as its subject or key prop: write it into your own scratch kit file
  `zz-scratch/kit-R<n>.ts` with sections `// @@ R<n> extra` (functions, named `<camelCase>Part`) and
  `// @@ R<n> extra kit` (registry lines, two-space indent, with a category), spliced by
  `python3 zz-scratch/splice.py zz-scratch/kit-R<n>.ts`, following `zz-scratch/KIT_RULES.md`. Keep extras few; a
  small detail is changed to fit the kit and said so in the doc comment.

## Looking and measuring

- From the repo root: `PREVIEW_IDS=core-001,core-002,... PREVIEW_NAME=rec<n> PREVIEW_SIZE=200 sh zz-scratch/run.sh
  1700 1100` draws each card's base and Radiant picture (the dashed box is a face's window) into
  `/tmp/preview/rec<n>.png` (Read it) and prints each card's elements, layers, parts and bytes. About 12 cards a
  shot. Look at every card and fix what reads wrong: the subject must read at hand size, the brief's idea visible,
  nothing floating oddly, the subject inside the window.
- Run nothing else: no web suite, typecheck, eslint or cargo; touch no other file; do not commit.
- If the preview fails because another helper's code broke sceneKit.ts, wait a minute and retry.

## Report

For every card in your file, one row: `| id | name | rarity | type | base elements | layers | character | verdict |`
(character: the figure's look in short, e.g. `Human: helmet, outfit-plate, item-tower-shield`, or the creature
part, or `—`; verdict: `from its brief` or the change made and why). Then any extra parts you added.
