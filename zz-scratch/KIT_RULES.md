# Kit-part rules for the #3392 helpers

JackiOh's web client paints card pictures as SVG "scenes" (`apps/web/src/cards/art/scene.ts`): a recipe places
kit parts from the scenery kit (`sceneKit.ts`, `SCENE_KIT`) and the figure kit (`figureKit.ts`, `FIGURE_KIT`)
in seven layers. Issue #3392 gives every Core card a scene of its own, so the kits need many new parts. Several
helpers write parts at once. Read `apps/web/src/cards/art/sceneKit.ts` (its header and a dozen existing parts:
`crate`, `tower`, `tree`, `cloud`, `moon`, `field`, `tileFloor`, `roomWall`, `lantern`, `moth`, `paleHand`) before
you write, and copy their style exactly. ART.md's "The art bible" / "Core" section says the look (read it).

## Where scenery code goes (never edit sceneKit.ts directly)

- Keep ALL your code in your scratch file `zz-scratch/kit-<X>.ts` (X = your letter, given in your brief). It holds
  sections, each starting with a header line `// @@ <block>`, e.g. `// @@ A terrain` (function declarations)
  and `// @@ A terrain kit` (registry lines, two-space indented, e.g. `  water: { category: "terrain", draw: water },`).
- Splice it in with `python3 zz-scratch/splice.py zz-scratch/kit-<X>.ts` (from the repo root): it replaces what
  lies between your `// <<block>>` and `// <</block>>` markers in sceneKit.ts, under a lock, and refuses code
  esbuild cannot parse. Re-run it after every change. Never edit sceneKit.ts with Edit/Write: others are splicing.
- The scratch file is spliced INTO sceneKit.ts, so use its private helpers directly: `local`, `shape`, `between`,
  `whole`, `turned`, `leant`, `sparkle`, `blob`, `groundBand`, `crescent`, `spiralArm`, `pt`, `circlePath`,
  `ellipsePath`, `rectPath`, `polygonPath`, `bandPath`, `fmt`, the flags `MAIN`, `INKED`, `MAIN_INKED`, the
  constants `EDGE`, `FAR_EDGE`, `SCENE_BOX`, `FULL_TURN`, `DEGREES`, and the types `Point`, `PartPlace`, `Rng`,
  `KitShape`. Name each function `<camelCaseId>Part` (`waterPart`, `cardFanPart`) so no name clashes with an
  existing function or another helper's.

## What a part is (sceneKit.ts's header, binding)

- `function <name>(at: PartPlace, rng: Rng): KitShape[]`, usually `const f = local(at);` then shapes built with
  `f.poly`, `f.circle`, `f.ellipse`, `f.band` (offsets from the anchor in the part's own frame, scaled by `at.s`,
  mirrored when `at.flip` is -1). A terrain part spans the box from its placement's height down (or a room's wall
  from the top down to it), in absolute box coordinates like `field`/`tileFloor`/`roomWall`; every other part
  stands on its anchor (base at the anchor), hangs from it, or centres on it, as its doc comment says.
- 1 to 12 shapes per call, from any seed, scale 0.5–1.5 and either flip. Merge small bits of one colour into one
  shape as several sub-paths (`a + b + c`), never letting two sub-paths of one shape overlap (fills are even-odd:
  an overlap punches a hole; a deliberate hole is a second sub-path inside the first).
- Paths only from the helpers (M L Q C A Z commands, no commas, no H/V/S/T): the tests hold every path to
  `^M[-\d. MLQCAZ]+$`. No text, nothing but `d` strings. Never `Math.random`.
- Colours: palette names only (`SCENE_PALETTE` in scene.ts). NEVER `gold-400`, `gold-300`, `gold-200`,
  `yellow-400` or `parchment` (Radiant's gold window). Silver is `slate-100`/`slate-400`. There is no pink or
  yellow: use `red-300`/`purple-400` for pinks, `orange-400`/`brass-500`/`white` for warm lights.
- Opacity in (0, 1]. A glow is a stack of translucent shapes, never a gradient.
- `MAIN` on the shapes a placement's `paint` repaints (the body colour), `INKED` on body-outline shapes (other
  families outline them), as the existing parts do. A little `rng` jitter (`between`, `whole`) so copies differ,
  the silhouette recognisable at every seed.
- A one-line doc comment on each part: what it is, where it stands relative to its anchor, its size at scale 1
  ("A lighthouse standing on the anchor, 36 tall at scale 1").
- Core's look: clean flat shapes edge to edge, highlights translucent `white`/`slate-100`, shade translucent
  `slate-900`, light from the upper left. Sizes are in the 100-unit box: a figure is 56 tall at scale 1, a crate
  about 7, a tree 16, a tower 30.
- Economical: a Mythic's picture holds ~150–220 shapes under 48 KiB, so prefer 3–8 shapes per part and short
  paths (a circle is cheap; a 40-point polygon is not).

## Checking and looking

- From the repo root: `PREVIEW_PARTS=a,b,c PREVIEW_NAME=kit<X> PREVIEW_SIZE=160 sh zz-scratch/run.sh 1700 1100`
  draws each part twice (scale 2.5 facing right, scale 1.2 facing left, anchor at 50,60 on a grey-blue ground)
  into `/tmp/preview/kit<X>.png` (Read it to look), and prints `CHECK <part> ...` for any part that breaks the
  rules over 40 seeds (no CHECK line: it passes). Look at every part once; about 30 parts per shot. Use your own
  PREVIEW_NAME.
- Run nothing else: no web suite, typecheck, eslint or cargo. Touch no other file. Do not commit.
- If the preview fails because ANOTHER helper's block broke (the log names its function), wait a minute, retry.

## Report

When done: every part id you added, one line each: `id` (category): what, anchor, size at scale 1, main paint.
Under 100 lines.
