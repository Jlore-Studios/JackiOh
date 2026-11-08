# Brief: draw the scene kit's objects module (sceneKitObjects.ts) for issue #3390

Worktree: /home/runner/work/_temp/work/item-3390 (JackiOh, a card game; the web client paints each card's art as a
procedural SVG). Fill `apps/web/src/cards/art/sceneKitObjects.ts`, which today exports an empty `OBJECTS_KIT`.
sceneKit.ts already spreads `OBJECTS_KIT` into `SCENE_KIT`, so every part you add is usable by recipes. Other agents
write the recipe files at the same time and already place these parts by the ids, anchors and sizes below: touch ONLY
sceneKitObjects.ts. Do not commit, stash, switch branches or run git commands that change state.

IMPORTANT, your session may be cut off at any moment: write the file in pieces and SAVE EARLY. First write the whole
file with all 34 parts drawn simply (so every id exists and recipes stop crashing), run the checks, then improve parts
by eye.

## Read first
- `apps/web/src/cards/art/kitDraw.ts`: the helpers (`local` frame, `shape`, `MAIN`, `INKED`, `MAIN_INKED`, `blob`,
  `sparkle`, `turned`, `leant`, `between`, `whole`, `rectPath`, `ellipsePath`, `circlePath`, `polygonPath`, `bandPath`).
- `apps/web/src/cards/art/sceneKitNature.ts` (skim its first 150 lines and its table): a finished module in exactly the
  style to match (file header comment, one doc comment per part saying its anchor and size at scale 1, the
  `as const satisfies Readonly<Record<string, KitPart>>` table at the end).
- `apps/web/src/cards/art/scene.ts`: `KitShape`, `PartPlace`, `SCENE_PALETTE` (the only colours).

## The rules every part keeps (tests enforce them)
- `draw(at: PartPlace, rng: Rng): KitShape[]` returns 1 to 12 shapes. Build coordinates only through `const f = local(at)`
  (so scale and flip work), as offsets from the anchor (y grows downward; "up" is negative dy).
- Path data uses only the commands M L Q C A Z (test regex `/^M[-\d. MLQCAZ]+$/`): use `f.poly`, `f.circle`, `f.ellipse`,
  `f.band`, `f.p` in template strings, and `fmt` from `./hash.ts` for any raw number. No `H`, `V`, `S`, `T`.
- Every paint is a `SCENE_PALETTE` name, NEVER `gold-400`, `gold-300`, `gold-200`, `yellow-400` or `parchment` (Radiant's
  gold window, R2725004). Gold/brass things use `brass-500`, `brass-600`, `orange-400`; paper `slate-100` or `white`;
  silver `slate-400`/`slate-100`. Opacities in (0, 1].
- Randomness only from `rng` (via `between`/`whole`), a little per part (a tilt, a count, a jitter).
- Flags: solid body shapes `INKED` (Classic outlines them slate-900); the one shape a recipe recolours `MAIN` /
  `MAIN_INKED` (a placement's `paint` repaints `main` shapes). Highlights and glows are not inked.
- A glow is a stack of 2-3 translucent shapes, never a gradient. Filled even-odd: a hole is a second sub-path.
- Keep paths short (each picture has a 48 KiB budget and a Mythic draws 150 shapes): few points per shape, 3-7 shapes
  per part is typical.
- Pure: no Math.random, Date or I/O.

## The parts (id: what; anchor / size at scale 1). Keep these anchors and sizes: recipes are written against them.
A part stands on its anchor (bottom centre at x, y) and draws upward unless the row says "centred". Things that read
better tilted are drawn already tilted by a small random angle (recipes cannot rotate a part, only flip it).

| id | what | anchor / size at scale 1 |
|---|---|---|
| scroll | a parchment scroll half unrolled, rolled ends dark-edged (main = paper, slate-100) | lying on anchor, 16 wide, 6 tall |
| tome | a heavy open book, leather cover (main = cover, red-900), pages fanned, a ribbon | on anchor, 16 wide, 7 tall |
| card | one blank playing card standing tilted ±15°, a plain border and a simple diamond on its back, no text (main = back, blue-800) | bottom at anchor, 7 wide, 10 tall |
| coins | a short stack of silver coins and 2-3 loose ones (main = coin, slate-400) | on anchor, 12 wide, 6 tall |
| chest | a wooden chest, lid thrown open, iron bands, a glint inside (main = wood, wood-700) | on anchor, 16 wide, 13 tall |
| gems | a scatter of 4-6 faceted gems in two or three colours (main = the largest gem) | about anchor, 14 wide, 4 tall |
| hourglass | an hourglass in a wooden frame, sand running (main = sand, brass-500) | on anchor, 7 wide, 12 tall |
| gear | a toothed gear with a hub hole (main = metal, slate-400) | centred, radius 8 |
| flask | a round-bottomed glass flask, glowing liquid (main = liquid, green-400), a glass highlight | on anchor, 8 wide, 12 tall |
| cauldron | an iron pot on three short legs, its brew at the rim (main = brew, green-400), a few bubbles | on anchor, 18 wide, 12 tall |
| pipes | copper pipes with two elbows, a flange and a valve wheel (main = pipe, brass-600) | bottom centre at anchor, 30 wide, 16 tall |
| axe | a broad executioner's axe, its haft rising from the anchor tilted 20-35° to the right, a big crescent head at the top (main = blade, slate-400) | haft foot at anchor, 22 long |
| blade-ring | 6-8 swords in a spinning ring, points outward, a faint motion swirl (main = blades, slate-100) | centred, radius 12 |
| arrow | an arrow flying the way it faces (right at flip 1): shaft, head, fletching, a faint speed streak behind (main = fletching, red-600) | centred, 18 long |
| spatula | an iron spatula planted blade-down, handle up (main = blade, slate-400) | blade's foot at anchor, 20 tall |
| bellows | leather bellows, nozzle pointing the way it faces, handles behind (main = leather, wood-700) | centred, 14 wide, 8 tall |
| weights | two stone dumbbells lying on the ground, one cracked (main = stone, stone-500) | on anchor, 18 wide, 6 tall |
| weapon-rack | a wooden rack holding 3-4 spears and a sword upright (main = wood, wood-700) | on anchor, 18 wide, 20 tall |
| gift-box | a ribboned gift box with a bow (main = paper, red-600) | on anchor, 8 wide, 9 tall |
| drum | a round war drum, laced hide, two straps (main = body, red-600) | on anchor, 12 wide, 12 tall |
| astrolabe | a brass armillary: rings on a stand (main = brass, brass-500) | on anchor, 12 wide, 16 tall |
| slate-board | a wooden-framed dark slate on short legs, chalk shapes on it (triangles, a curve; never letters or digits) (main = slate, slate-800) | on anchor, 18 wide, 14 tall |
| padlock | an iron padlock, shackle and keyhole (main = body, slate-700) | bottom at anchor, 5 wide, 7 tall |
| chain | a sagging chain of 6-9 links from the anchor to 22 to the right (main = links, slate-400) | starts at anchor, 22 long, sags ~5 |
| key | a large old key lying across, bow ring and bit (main = metal, slate-400) | centred, 12 long |
| candle | a wax candle with a flame and a small 2-3 shape glow (main = wax, slate-100) | on anchor, 2 wide, 6 tall plus flame |
| bones | a small heap of bones with one skull (main = bone, slate-100) | on anchor, 12 wide, 5 tall |
| pebbles | 5-8 small scattered stones (main = stone, stone-500) | about anchor, 16 wide, 3 tall |
| knuckles | a pair of iron brawler's knuckles (main = iron, slate-400) | centred, 8 wide, 4 tall |
| lyre | a wooden lyre with strings (main = wood, wood-600) | on anchor, 8 wide, 12 tall |
| scales | a stone balance: a post, a beam tipped by a random angle, a pan hung from each end (main = stone, stone-500) | on anchor, 22 wide, 16 tall |
| dice | two dice with pips (main = die, slate-100) | on anchor, 7 wide, 3 tall |
| bread | a crusty loaf, scored top (main = crust, wood-600) | on anchor, 8 wide, 4 tall |
| quill | a feather quill standing in an inkwell (main = feather, slate-100) | on anchor, 4 wide, 13 tall |

Every part's category is "objects" (`{ category: "objects", draw: … }`). Name the table `OBJECTS_KIT` exactly and keep the
ids exactly as above. No id may repeat one in `sceneKit.ts`, `sceneKitPlaces.ts` or `sceneKitNature.ts` (the test checks);
none above does. Start the file's header comment with `// R3390000: the scene kit's objects (ART.md, "Scene recipes and
kits"), which the Classic+ scenes (#3390) added:` and list them in a sentence or two, like sceneKitNature.ts's header.
Import only what you use (eslint flags unused imports).

## Look at what you draw
Scratch viewer (do not delete it): `apps/web/src/cards/art/zz-kit.scratch.test.ts`. The web tests need the WebAssembly
build (`apps/web/src/wasm/pkg`); if vitest fails because it is missing, wait a few minutes and retry.

    cd /home/runner/work/_temp/work/item-3390
    KIT_PARTS=scroll,tome,card,coins,chest,gems KIT_SHEET=objects1 KIT_Y=70 pnpm --dir apps/web exec vitest run src/cards/art/zz-kit.scratch.test.ts
    google-chrome --headless --disable-gpu --window-size=1200,400 --screenshot=/tmp/kit/objects1.png /tmp/kit/objects1.svg

Read the PNG. Each cell is the 100-unit box at 2x, the part drawn at (50, KIT_Y) (add KIT_SCALE=2 to see small parts
bigger). Look at every part at least once (sheets of 6 parts, window 1200x400 per row of 6) and fix what does not read.

## Check before you finish
    cd /home/runner/work/_temp/work/item-3390
    pnpm --dir apps/web exec vitest run src/cards/art/scene.test.ts
    pnpm --dir apps/web typecheck 2>&1 | grep sceneKitObjects    # must print nothing
    pnpm exec eslint apps/web/src/cards/art/sceneKitObjects.ts
Failures in scene.test.ts about recipes or other files are not yours; "every part draws clean shapes" and the palette
test must pass for your parts. Never add eslint-disable, @ts-ignore or @ts-expect-error.

Report: the ids you drew, any deviation from the anchors/sizes above, and the check output.
