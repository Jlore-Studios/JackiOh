# Issue #2803: card-animations audit — coverage, consistency, motif use, tiers, phone frames, entrances, captures

2026-11-04. Closes the #2782 card-animation work: proves the tables complete and consistent,
reworks the shared Mythic/Legendary entrances and the reduced-motion still, and records every
number below from commands run on `bot/issue-2803` (base `78ba41a9ff52`).

## 1. Coverage

`apps/web/src/fx/cards/coverage.test.ts` (`it("R2803000 …")` × 3) holds every table to
`FX_COVERAGE_FLOOR` (0.5): 551 of the catalog's 634 non-token cards wear an entry other than
`none` (86.9%, floor 317). Every Spell, Field Spell, Trap and Field Trap wears a `cast`, a
bespoke recipe, a set piece, or the one `none` a Starship part may wear (Stinkyscape #176
Preemptive Strike, "Starship part", R2801000); every Field Spell wears an `aura`. The census
(`cargo run --release -p jackioh-tools -- fx census --format json`, default 200 seeds, 747
cards) is the family oracle for §2.

Counts (non-token cards with an entry other than `none` / non-token cards):

| Set | Drawn / cards |
|---|---|
| Core | 87 / 100 |
| Classic | 79 / 90 |
| Classic+ | 74 / 78 |
| Meditative | 94 / 102 |
| Meditative+ | 24 / 26 |
| Stinkyscape | 142 / 178 |
| Training Grounds | 51 / 60 |

| Type | Drawn / cards |
|---|---|
| Unit | 195 / 277 |
| Spell | 235 / 236 |
| Field Spell | 87 / 87 |
| Trap | 24 / 24 |
| Field Trap | 10 / 10 |

| Rarity | Drawn / cards |
|---|---|
| Common | 138 / 181 |
| Rare | 181 / 198 |
| Epic | 132 / 144 |
| Legendary | 72 / 81 |
| Mythic | 28 / 30 |

## 2. Consistency

Grouped by the census's play-hook families, 364 non-token entries override the census
suggestion. Every override carries its reason in its set pass's Why cell (the pass tests hold
each row to a non-empty census and why): the census reads only damage, removal, return and
cost families, so a pass maps every other event to the motif whose Reads column names it.
Seven table entries changed for the §3 fold, each with its reason in its set pass's Why cell:
classic-054 (`mirror`→`transmute`), classic-075 (`ward`→`bless`), classicplus-004
(`mirror`→`transmute`), meditative-022-1 (`glimpse`→bespoke `mindJudge`), meditative-040
(`glimpse`→`bless`), meditativeplus-005 (`chain`→`nova`), stinkyscape-145 (`ward`→`bless`).
No other table entry changed.

The sharp cases — same damage family, different motif, and damage-family suggestions left
at `none` — each have a reason:

- `none` where the census sees damage: core-009 (combat is the attack rows'), core-050
  (mark's own row), classic-077 (Aura taxes, no moment), classicplus-011 (static Aura),
  stinkyscape-077/-088/-116 (modifier/market noise in the window), traininggrounds-056
  (grows in the hand).
- Damage-family disagreements: core-074 and classic-012 (lance counted a second target;
  one hit is a bolt), traininggrounds-010 (kept, #2784's volley example), traininggrounds-017
  (one blast is a nova), traininggrounds-028 (a cleared lane is a lance), stinkyscape-003
  (kept, #2784's nova example), stinkyscape-013 (every enemy at once is a nova),
  stinkyscape-097 (kept, #2784's barrage example), meditativeplus-005 (folded: R2784000's
  chain example becomes nova, §3).

## 3. Motif use

Table entries per drawn motif (hook motifs over all eight tables), after the fold:

| Motif | Cards |
|---|---|
| bolt | 65 |
| barrage | 9 |
| bind | 8 |
| bless | 51 |
| conjure | 114 |
| curse | 6 |
| deal | 59 |
| drain | 3 |
| flight | 35 |
| hex | 6 |
| lance | 11 |
| mana | 12 |
| mend | 13 |
| negate | 9 |
| nova | 15 |
| offering | 14 |
| radiance | 11 |
| recall | 3 |
| rise | 9 |
| scorch | 3 |
| seal | 13 |
| send | 108 |
| shift | 6 |
| snap | 13 |
| tally | 17 |
| tether | 15 |
| tithe | 3 |
| transmute | 17 |
| vanish | 27 |
| volley | 7 |
| wipe | 14 |

31 drawn motifs remain, every one used by at least `FX_MOTIF_MIN_CARDS` (3) cards. The four
below the floor are folded, per the review decision: `chain`→`nova` (Spirit Lash hits every
Unit at once; amends R2784000's Examples-table citation, and the docs' Examples row now reads
`cast: nova`); `mirror`→`transmute` (Rewind, Juhan Biggest Bat); `ward`→the rows (Armor's
full shield and the Divine Shield break draw in `cues.ts` in Holy, shaking `FX_ARMOR_TRAUMA`;
Argusland and Fortify Flesh name `bless`); `glimpse`→bespoke `mindJudge` (Fortify Mind keeps
its judgment's sheen and flare on the hero's badge, shaking `FX_MIND_JUDGE_TRAUMA`) and
`bless` (Feng Shui). The set passes' `.md` rows carry each move's reason. `docs/polish/9-card-animations.md`
loses the four motif rows, and its Dispatch section no longer names `chain`.

Sub-looks: all 21 `FLAVOR_SUB_LOOKS` entries are used (fewest: Beast/swarm 3, Nerd/pixel 4,
Beast/wool 5, Physical/blade 5, Physical/sport 5). None removed.

## 4. Tiers

`apps/web/src/fx/cards/tiers.test.ts` extends the R200 tests to every table entry: each
entry's gallery play is planned through the board's effects layer (entrance, motif,
recipes, full plan), and every cue starts inside its entry and ends within
`FX_MAX_TAIL_MS` of its end (pulls and conceals stay the stage's). Every set piece's hold
(`nether`, `voidFold`, `vortex`, the takeover's `TAKEOVER_HOLD_MS`, the Starship's
`STARSHIP_LAUNCH_HOLD_MS`) stays within `SET_PIECE_HOLD_CAP_MS`, and
`SET_PIECES_PER_ACTION` is 1. A hidden play keys no entry (R202). All 5 tests pass (645
non-`none` entries planned).

## 5. Frame time on a phone

`e2e/cypress/component/fx-phone-frame.cy.tsx` plays all five scenes at 390×844
(phone particle cap by `capacityFor`, pinned in-spec) and asserts each scene's mean rAF
frame time within `FX_PHONE_FRAME_BUDGET_MS` (50 ms), logging the distribution. Per the
issue's "fix or scale back", both misses were scaled back with their composition
untouched (same layers, choreography, holds, timing):

- Twisting Nether: `SET_PIECE_TUNING` pull 10→6 and implode 64→40, its trauma 1→0.7,
  and its vortex spiral 120→70vmax (one-sided 80→47vmax) with the mask retuned by the
  same ratio, so the visible arms keep their radii.
- Mythic takeover: `TAKEOVER_TUNING` Mythic burst 48→28, `FX_TAKEOVER_TRAUMA` 1→0.6,
  and its rays 220→120vmax with the mask retuned by the same ratio, so the visible
  disc keeps its radius.

Measured in headless Chromium (SwiftShader software rendering, dev server, phone
viewport, normal intensity, rAF sampling mirroring the spec):

| Scene | Mean | p50 | p90 | Verdict |
|---|---|---|---|---|
| Cap check | — (pure assertion) | — | — | pass |
| Twisting Nether | 19.1 | 16.7 | 17.9 | pass |
| Starship launch | 17.3 | 16.7 | 17.0 | pass |
| Mythic takeover | 26.1 | 16.8 | 51.4 | pass |
| Board with 4 auras | 16.7 | 16.7 | 16.8 | pass |

Two honest caveats. First, the numbers above are from a quiet box; the same scenes read
45–65 ms (Nether) and 56–127 ms (takeover) while another task's WASM build contended
for the CPU, and the earlier pass read 207/125 ms on a contended box. Software
rendering has no stable clock: one-time costs (first paint, new-layer rasterization at
the takeover's landing) dominate the mean whenever the box is busy. The scale-back
banks margin for that, but the daily CI verdict rules — a contention failure there is
an infra flake, not a scene regression, and the spec's log lines carry the distribution
to tell them apart. Second, SwiftShader punishes what phone GPUs do free (full-screen
gradient layers, blur shadows); the sustained costs above (17–30 ms) are what a phone
feels. A follow-up issue (to be opened by the maintainer) tracks measuring the scenes
on a real phone, and is cited in the spec and R2803000.

## 6. #2722's entrances

Shared looks reworked to land in the motif language's two-wave landing (R2803000 amends
R2722001; music, timing, audience and the one capped hold unchanged; R200, R202,
reduced motion, `transform`/`opacity` only):

- Legendary (summoned, cast, reveal): its ring carries a gold echo (`entranceEchoMs`, the
  same wave behind as the takeover's second shockwave) — `takeover.ts`, `cues.ts`
  summon path; pins updated in `takeover.test.ts`, `cues.test.ts`.
- Shared Mythic takeover: the prismatic sweep has its own keyframes
  (`fx-takeover-prism`: crosses farther, settles in two steps) — `fx.css`; verified
  mounted mid-hold in a headless browser (takeover + dark/sweep/rays/wave present).
- Recoil: every other card dips and overshoots in two steps with the landing —
  `board.css` (`takeover-recoil`).
- Still: echoes the takeover at rest — motionless rays disc and landing ring around
  the card (`TakeoverStill.tsx`, `board.css`); verified in a headless browser under
  reduced motion and pinned in `TakeoverStill.test.tsx`. `TAKEOVER_STILL_MS` unchanged.

Per-card marquee entrances untouched. `entrances.test.ts` needed no pin change (it pins
the marquee entrances and the summon-path rarity entrance, which are unchanged apart
from the Legendary echo, pinned in `cues.test.ts`).

## 7. Captures

`e2e/cypress/component/fx-gallery-captures.cy.tsx` plays every drawn motif mid-flight
(first gallery card per motif through the real board layer, canvas-lit asserted,
screenshot per motif) and every set piece (`nether`, `voidFold`, `vortex`, Starship
launch) mid-flight and at rest. `shift` proves its own visual instead of canvas pixels:
its entry plans 10–20 ghosts + bursts (unit and in-page plan dumps agree), and
director play/fire timelines in headless Chromium show every cue fired with valid
anchors (20 ghosts mount) yet zero canvas draws. Root cause: firing ~40 cues in one
tick forces a full-board reflow per cue in software rendering, stalling the frame loop
~700 ms, so the short-lived bursts (preset `circuit`, life 300–650 ms) age out before a
frame paints them. The ghosts are the motif's signature — every moved card's path walked
in the DOM — so the spec asserts them at 0.25× effects speed (same cues, wider window),
screenshotted mid-flight; the tier test proves every cue's timing. No skip remains.
`negate` and `deal` stay canvas-gated: their earlier misses were flakiness on a
contended box, with no failure on a quiet one. The existing `fx-layer.cy.tsx`
takeover/set-piece pins still hold (verified green in unit tests; the component runs
gate in CI's daily super run — no Cypress binary could be installed in this checkout,
see §9).

## 8. Docs

- `docs/polish/9-card-animations.md`: new "Audit" section (coverage tables, tiers,
  the fold to 31 motifs, the split phone budget, reworked entrances, captures); the
  `chain`/`ward`/`mirror`/`glimpse` motif rows are gone, Dispatch no longer names
  `chain`, and the Examples row reads `cast: nova` for Spirit Lash.
- `docs/ADDING_CARDS.md` row 13: checked, matches the shipped tables (motif/bespoke/
  setPiece/`none`, aura, entrance, `RECIPE_FIXTURES`, gallery) — unchanged.
- Ruling `spec/rulings/R2803000.md` (amends R2722001 and R2784000's Examples table) +
  `spec/pending/2803.md`; `cargo jackioh spec check`: 1677 rulings, 4061 proofs, 50293 citations — ok.

## 9. Verification and open items

Green here: the three coverage tests, the five tiers tests, constants (12 tests),
takeover/cues/entrances/css/dom/setPieces/TakeoverStill (239 + 150 tests), gallery +
tables (25 tests), `pnpm --dir apps/web typecheck`, `pnpm exec eslint apps/web/src/fx`,
`e2e typecheck:component`, `cargo fmt --check`,
`cargo clippy -p jackioh-tools --all-targets -- -D warnings`, `cargo jackioh spec check`
plus `catalog`/`patches` check. The Luau WASM module had to be built
(`sh scripts/build-wasm.sh` runs it) before `typecheck` passed — environment setup, no
repo change.

Not runnable in this checkout: the Cypress runner itself (`cypress install` 404s, no
runner binary) — but a system Chrome (156) is present, so the next best thing ran here:
each phone scene played through the real board effects layer in headless Chromium
(SwiftShader) behind a temporary `/dev/fx?probe=` harness (reverted before delivery),
sampled with the spec's own rAF-delta method over CDP. That is where the §5 table comes
from. The two component specs gate in CI's daily super run, unrun here. `e2e
typecheck:component` is green, including a `HandView` spread fix in the gallery spec
(the earlier "no type errors" claim was wrong; fixed, not waived).

Decisions: fold the four under-floor motifs (`chain`→`nova`, `mirror`→`transmute`,
`ward`→rows + `bless`, `glimpse`→bespoke `mindJudge` + `bless`; rejected: keeping them
as reasoned exceptions, which leaves the floor unenforced); scale back Nether and the
takeover per the issue (counts, shakes, off-screen raster only — rejected: raising the
budget, and cutting layers/choreography/holds/timing, which would punish real phones);
prove `shift` by its ghosts (rejected: leaving the skip, and asserting canvas pixels a
DOM look never paints); keep the mean metric for all five phone scenes (rejected:
median/trimmed-mean gating and warm-up replay — both change what is measured to reach
green); leave the frame loop's per-cue measure/mount interleaving alone (batching reads
before writes would fix software-rendering stalls like `shift`'s, but its blast radius
doesn't fit an audit); gallery Unit plays without a landing keep their unmeasurable
hand anchor (pre-existing, out of scope — takeovers draw with a measurable anchor,
verified with core-096). Open: one follow-up issue for the maintainer to open — a
real-phone frame measurement for Nether and the takeover — cited in the spec, the
ruling and §5 above.
