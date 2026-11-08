# Brief: write Classic+ scene recipes for issue #3390 (common part; your card list is a separate file)

Worktree: /home/runner/work/_temp/work/item-3390 (JackiOh, a card game). The web client paints each card's art as a
procedural SVG. A **scene recipe** (data) composes a card's picture from kit parts in seven layers. Issue #3390 gives
every Classic+ entry (116 cards and tokens) a recipe of its own, drawn from its brief, richer the rarer. Three agents
each fill one file; you fill ONLY `apps/web/src/cards/art/classicPlusRecipes<N>.ts` (N in your card list's brief).
Do not edit any other file (the kits are other agents' work; if a part is missing or broken, work around it and say so
in your report). Do not commit, stash, switch branches or run git commands that change state.

IMPORTANT, your session may be cut off at any moment (usage limits): SAVE TO DISK EARLY AND OFTEN. Write the file in
batches (header + the first 6-8 recipes + the export table, then add recipes batch by batch, keeping the export table
current), so a cut-off loses at most one batch.

## Read first (in this order)
1. `apps/web/src/cards/art/ART.md`: "The art bible" (esp. "Classic and Classic+" and the rarity ladder), "Scene recipes and
   kits", "Classic+ drawn as scenes (R3390000)".
2. `apps/web/src/cards/art/scene.ts`: `SceneRecipe`, `ScenePlacement` (part, x, y, scale, flip, count, spread, paint,
   props), `FigureRecipe`, `ExtraFigure` (a figure + `layer`), `SCENE_PALETTE`, `composeScene`.
3. `apps/web/src/cards/art/sceneRecipes.ts`: the five proof recipes. CORPSE_PLANTATION (a Classic Epic) is the bar and the
   style to copy: one `const` per card with a one-line doc comment `/** <Name> (Classic+ #<n>, <Rarity>): <the brief in a line>. */`.
   `classicPlusRecipes3.ts` is another agent's finished draft in the same format (read a few recipes of it as examples).
4. `apps/web/src/cards/art/figureKit.ts`: `FigureLook`, the ids (PoseId, HeadId, HairId, OutfitId, ItemId), `POSE_ANCHORS`,
   `FIGURE_LOOKS` / `figureLookFor` (a tag's default look), each part's doc comment. Read the doc comments and the id
   unions, not every draw function.
5. The scenery kits' parts and what they are: `.scratch-3390/bbox.tsv` lists every scene part's extent around its anchor
   at scale 1 (x and y offsets) and its shape count; the doc comment above each draw function in `sceneKit.ts`,
   `sceneKitPlaces.ts`, `sceneKitNature.ts`, `sceneKitObjects.ts` says its anchor and size (grep `^/\*\*` in them rather
   than reading whole files). Terrain parts (field, hills, water, stairs, mountains, cave, tile-floor, rock-floor,
   ice-floor, street, furrows, room-wall, cliff) span the whole box from the placement's y down (room-wall and cave: from
   the top down to y).
6. `apps/web/src/cards/art/classicPlusScenes.test.ts`: the tests your recipes must pass (read every `it`).

## The objects kit may still be being drawn when you start
Another agent is drawing `sceneKitObjects.ts` (OBJECTS_KIT) now. Its ids and anchors are fixed; write against them:
scroll (16w 6h, lying), tome (open book 16w 7h), card (blank card 7w 10h, tilted), coins (stack + loose, 12w 6h), chest
(open, 16w 13h), gems (scatter 14w 4h), hourglass (7w 12h), gear (centred, r 8), flask (8w 12h, glowing), cauldron (18w 12h),
pipes (30w 16h), axe (haft foot at anchor, 22 long, tilted right), blade-ring (centred, r 12), arrow (centred, 18 long,
flying the way it faces), spatula (planted, 20 tall), bellows (centred 14w 8h, nozzle the way it faces), weights (stone
dumbbells 18w 6h), weapon-rack (18w 20h), gift-box (8w 9h), drum (12w 12h), astrolabe (12w 16h), slate-board (chalk slate on
legs 18w 14h), padlock (5w 7h), chain (from the anchor 22 to the right, sagging), key (centred, 12 long), candle (2w 6h +
flame), bones (heap + skull 12w 5h), pebbles (16w 3h), knuckles (centred 8w 4h), lyre (8w 12h), scales (a stone balance 22w
16h), dice (two, 7w 3h), bread (loaf 8w 4h), quill (in an inkwell 4w 13h). Each stands on its anchor unless "centred" and
has one `main` shape a placement's `paint` recolours. A part not yet in `OBJECTS_KIT` crashes a render (`kit.draw` of
undefined): check with `grep -c '^  "\?<id>"\?:' apps/web/src/cards/art/sceneKitObjects.ts` before rendering a card that
uses it, and render the cards that need none of them first. Never edit that file yourself.
The figure kit is finished: poses pose-stand, -guard, -raise, -spread (arms wide), -strike (hands overhead), -run, -sit,
-flex, -kneel; heads head-human, -felinor, -pancake, -fruit, -void, -bat, -skull, -mushroom, -sun, -golem, -automaton,
-goblin, -mummy; hair/headgear hair-short, hair-long, hood, hat-pointed, antennae, visor, toque, leaf-crown, kerchief,
hat-brim, goggles, helm, tonsure; outfits outfit-tunic, -armor, -coat, -jacket, -suit, -fur, -stack, -peel, -void, -apron,
-robe, -plate, -tabard, -poncho, -wraps, -muscle, -drum, -cardigan, -leaves, -patched, -stone; items item-sword, -shield,
-staff, -book, -blaster, -mallet, -dagger, -spoon, -banner (a flag on a pole ~26 above the hand), -flask, -bow, -spear,
-coin, -sling, -tray, -scroll, -hourglass, -sack, -tower-shield.

## The file you fill
- a header comment like sceneRecipes.ts's: `// R3390000: Classic+'s scene recipes, part <N> of three (#<first> to #<last> ...)`,
  a line on what they are (ART.md, "Classic+ drawn as scenes"), presentation only (CLAUDE.md rule 7);
- `import { figureLookFor } from "./figureKit.ts";` (if used) and `import type { SceneRecipe } from "./scene.ts";`;
- one `const NAME: SceneRecipe = { ... };` per entry, in id order, each with its one-line doc comment;
- `export const CLASSIC_PLUS_RECIPES_<N>: Readonly<Record<string, SceneRecipe>> = { "classicplus-001": DOOM_SHROOM, ... };`
  in id order. Formatting as sceneRecipes.ts (two-space indent, double quotes, trailing commas, one placement per line;
  lines up to about 160 characters).

## What every recipe must satisfy (tests check all of these)
- `family: "classic"`.
- `emblem` always set: `size` ≤ 8 (use 5-7), `x` 10..90, `y` 22..84, `rotate` 0 or small. Put it where it reads as part
  of the scene (on a banner, an altar, a chest, a shield, a door, a book, in the sky as a sigil), NEVER over a face.
- Element floor (counted on the SVG: shapes + about 4). Targets, measured with the sheet below:
  | rarity | elements ≥ | layers |
  |---|---|---|
  | Common, Token | 34 | any; use sky/mid/subject + one of near/light/particles |
  | Rare | 46 | any (use 5+) |
  | Epic | 68 | must include far, mid, subject and near (use 6-7) |
  | Legendary | 100 | all seven: sky, far, mid, subject, near, light, particles |
  | Mythic | 150 | all seven |
- Bytes: each picture, base and Radiant, ≤ 46 KiB (hard cap 48). Mythics run closest: prefer many small parts (motes,
  sparks, pebbles, grass-tuft) over many big terrain parts; each terrain part is a long path.
- Colours: only `SCENE_PALETTE` names, NEVER `gold-400`, `gold-300`, `gold-200`, `yellow-400` or `parchment` anywhere (sky,
  light.paint, a placement's paint, a figure's skin or cloth). Gold things use `brass-500`, `brass-600` or `orange-400`.
- Units (type "Unit"): every humanoid Unit has a `figure` whose `look` starts from `figureLookFor(<its tags>)` and is
  overridden into a look of its own, e.g. `look: { ...figureLookFor(["Human"]), hair: "kerchief", outfit: "outfit-apron",
  item: "item-spoon", cloth: "blue-800" }`. It must NOT equal its tag's default look, and no two Units' looks may be equal
  (across all three files: so vary skin/cloth too). A figure stands with its feet at (x, y), 56 units tall at scale 1:
  `y: 86, scale: 1.05` puts its crown near y 27; the item is at the pose's hand. Put it in the middle (x 40-60). Four Units
  are creatures with NO figure, drawing a kit part as subject in the `subject` layer: classicplus-003 `snake`,
  classicplus-039 `grub`, classicplus-051 `jet`, classicplus-064 `boar`.
- Crowds, companions, rivals: `extras: [{ layer: "mid", x, y, scale, pose, look, flip? }, ...]`, each a whole figure drawn in
  front of that layer's parts. Further back = smaller (0.4-0.7) and higher y. Tokens and spells may use a `figure` or extras
  for their people too (the Unit look rules apply only to Units).
- `props: true` on a placement makes it move on the props beat; NEVER on a placement in the `subject` layer.
- Full-art cards' motions need a shape in every role they move (back = sky/far/mid layers, subject = subject layer, over =
  near layer, light = light layer, props = a placement marked `props: true` outside subject, motes = particles layer):
  classicplus-012: back, subject, over. classicplus-019: back, subject, light, props. classicplus-026: subject, light, props.
  classicplus-046: subject, light, props. classicplus-048: subject, back, props. classicplus-062: subject, props, motes.
  classicplus-069: subject, props. classicplus-075: subject, back, props. (Props: banners, coins, gears, weights, relics.)
- Every recipe composes at least one `inked` shape (any solid kit part is inked).

## How to compose (art direction)
- The 100-unit box. A face's window shows about y 18 to 82, a Unit's portrait 12 to 88; x 0-100 shows. Keep the brief's
  SUBJECT within x 30-70 and y 25-85, large and readable at 150 px wide. FOREGROUND in `near` (y 80-100, at the sides or
  bottom edge, never hiding the subject), MIDDLE in `mid` and `subject`, BACKGROUND in `far` and `sky`. Light effects
  (halo, rim-light, light-shaft, god-rays) in `light`; motes, sparks, embers, spores, snow, petals, dust, rain, bubbles,
  glitch in `particles`.
- Classic's look: temple stone, an old world of carved stone, ruins, shrines and farms under a low sun from the left or a
  moon; stone-500, slate-700/800/900, wood-600/700/800, green-700 growth, red-600 banners/blood, green-400 plague. The brief's
  light picks `sky` (two palette names, top to bottom) and `light` (the glow: x, y, r, paint, opacity 0.3-0.6).
- Keep each brief's subject, setting and idea; change a detail to fit the kit if needed (say so in the verdict).
- Richer the rarer: Common one clear subject on a plain setting; Rare adds props; Epic a scene in depth; Legendary lights a
  moment of action; Mythic a set piece with many parts.
- `count` + `spread` multiply small parts cheaply; `paint` recolours a part's main shapes; `flip: -1` faces left.
- Readability beats count: no clutter over the subject; the subject is the clearest thing, contrasted against the sky.

## Look at what you made (required) and measure it
Scratch sheet (do not delete or edit it): `apps/web/src/cards/art/zz-sheet.scratch.test.ts`. The web tests need the
WebAssembly build in `apps/web/src/wasm/pkg` (the lead builds it; if vitest fails for want of it, wait and retry).

    cd /home/runner/work/_temp/work/item-3390
    CP_IDS=classicplus-001,classicplus-002,classicplus-003 CP_SHEET=p<N>a pnpm --dir apps/web exec vitest run src/cards/art/zz-sheet.scratch.test.ts
    google-chrome --headless --disable-gpu --window-size=1400,900 --screenshot=/tmp/cp/p<N>a.png file:///tmp/cp/p<N>a.html

It writes `/tmp/cp/<id>-base.svg` / `-radiant.svg`, an HTML sheet (base and Radiant side by side at 150 px, a dashed box
marking the face window y 18-82) and `/tmp/cp/p<N>a.tsv` (each card's elements, floor, layers, parts, KiB; also printed).
Read the PNG. Sheets of 6-8 cards; name sheets with your N (`p1a`, `p1b`, ...) so you never overwrite another agent's.
Fix every card whose subject does not read at 150 px or that is under its target. Look at EVERY card at least once.

Then run (failures on other files' cards are not yours, none of yours may fail):

    pnpm --dir apps/web exec vitest run src/cards/art/classicPlusScenes.test.ts src/cards/art/artRichness.test.ts src/cards/art/scene.test.ts
    pnpm --dir apps/web typecheck 2>&1 | grep classicPlusRecipes<N>     # must print nothing
    pnpm exec eslint apps/web/src/cards/art/classicPlusRecipes<N>.ts

Never add eslint-disable, @ts-ignore or @ts-expect-error.

## Report (your final message), and a copy on disk
Write `.scratch-3390/verdicts<N>.tsv` (one line per entry: `<id>\t<verdict>`, verdict "✅" when the scene is the brief, else a
short note of what changed and why) as soon as you have verdicts, and keep it current. Your final message: that the file
is complete, any kit part that was missing, broken or misdrawn (with the card it affected), and the test, typecheck and
eslint output summaries.
