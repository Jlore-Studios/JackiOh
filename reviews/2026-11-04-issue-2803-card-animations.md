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
No table entry changed.

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
  stinkyscape-097 (kept, #2784's barrage example), meditativeplus-005 (kept, R2784000's
  chain example).

## 3. Motif use

Table entries per drawn motif (hook motifs over all eight tables):

| Motif | Cards |
|---|---|
| bolt | 65 |
| barrage | 9 |
| bind | 8 |
| bless | 48 |
| chain | 1 |
| conjure | 114 |
| curse | 6 |
| deal | 59 |
| drain | 3 |
| flight | 35 |
| glimpse | 2 |
| hex | 6 |
| lance | 11 |
| mana | 12 |
| mend | 13 |
| mirror | 2 |
| negate | 9 |
| nova | 14 |
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
| transmute | 15 |
| vanish | 27 |
| volley | 7 |
| ward | 2 |
| wipe | 14 |

All 35 drawn motifs are used. Four sit below `FX_MOTIF_MIN_CARDS` (3) and stay as reasoned
exceptions: `chain` (Spirit Lash, R2784000's Examples-table citation), `ward` (Argusland's
standing shield, Fortify Flesh's Armor), `mirror` (Rewind's repeated Cry, Juhan Biggest
Bat's transformed Bat), `glimpse` (Fortify Mind's and Feng Shui's judgments). Folding any
of them would break its set pass and the design doc's motif tables, so the audit keeps
them; `docs/polish/9-card-animations.md` ("Audit") and R2803000 record the decision.

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

`e2e/cypress/component/fx-phone-frame.cy.tsx` plays Twisting Nether, the Starship launch,
a Mythic takeover and a board with 4 auras at 390×844 (phone particle cap by
`capacityFor`, pinned in-spec) and asserts each scene's mean rAF frame time within
`FX_PHONE_FRAME_BUDGET_MS` (50 ms).

Local figures (headless Chromium, software rendering, dev server — environment-bound,
not a phone verdict): calm board 79 frames, mean 25.6 ms; Nether hold 9–10 frames, mean
~231–252 ms; a single bolt burst ~2 frames, mean ~928 ms. The canvas layer already
avoids `shadowBlur` (`canvasFx.ts`: layered strokes and gradients under `lighter`), so
the local cost is SwiftShader rasterization, which no phone GPU shares; the audit
scales back no art on headless numbers. The spec gates in CI.

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
launch) mid-flight and at rest. The existing `fx-layer.cy.tsx` takeover/set-piece pins
still hold (verified green in unit tests; the component run gates in CI — no Cypress
binary could be installed in this checkout, see §9).

## 8. Docs

- `docs/polish/9-card-animations.md`: new "Audit" section (coverage tables, tiers,
  motif-use floor and exceptions, phone budget, reworked entrances, captures).
- `docs/ADDING_CARDS.md` row 13: checked, matches the shipped tables (motif/bespoke/
  setPiece/`none`, aura, entrance, `RECIPE_FIXTURES`, gallery) — unchanged.
- Ruling `spec/rulings/R2803000.md` (amends R2722001) + `spec/pending/2803.md`; `cargo
  jackioh spec check`: 1671 rulings, 4046 proofs, 50123 citations — ok.

## 9. Verification and open items

Green here: the three coverage tests, the five tiers tests, constants (12 tests),
takeover/cues/entrances/css/dom/setPieces/TakeoverStill (239 + 150 tests), gallery +
tables (25 tests), `pnpm --dir apps/web typecheck`, `pnpm exec eslint apps/web/src/fx`,
`cargo fmt --check`, `cargo clippy -p jackioh-tools --all-targets -- -D warnings`,
`cargo jackioh spec check`. The Luau WASM module had to be built (`JACKIOH_WASM_NORMAL=0
sh scripts/build-wasm.sh`) before `typecheck` passed — environment setup, no repo change.

Not runnable in this checkout: the three Cypress component files (new) and
`fx-layer.cy.tsx` (pins unchanged) — `cypress install` 404s here and no system
browser satisfies the runner, so they gate in CI. The e2e `typecheck:component`
reports only the pre-existing missing `pkg/luau` artifact in untouched
`apps/web/src/wasm/luau.ts` (also fixed by the Luau build above for the web gate);
the two new specs contribute no type errors.

Decisions: keep (not fold) the four under-floor motifs, each with its set-pass
reason (rejected: folding, which breaks passes and R2784000's cited example, or
re-pointing other cards at them); no art scale-back on headless frame numbers
(rejected: tuning scenes to SwiftShader); gallery Unit plays without a landing keep
their unmeasurable hand anchor (pre-existing, out of scope — takeovers draw with a
measurable anchor, verified with core-096).
