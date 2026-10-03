# `@jackioh/cards` — the card layer

317 card definitions in three sets — Core (100 cards and 11 tokens), Classic (90 cards) and
Classic+ (78 cards and 38 tokens), 268 cards and 49 tokens in all — one script file and one test
file per card, the catalog query every random pool in the game goes through, and the catalog's
patch history.

`SPEC.md` is the only source of rules and card text: §8 has every card (Classic in §8.6, Classic+ in
§8.7) and §7 every token. `BUILD.md` M4-T1…M4-T4 is Core's work order and M9 the new sets' (patch
v0.2.0, issue #40), and their must-pass tables are the per-card acceptance lists. The design brief
`docs/classic-sets.md` is where the new sets came from, not a rule. This file is the contract
*between* the card files: what a card file looks like, what the harness gives a test, and what order
the waves go in. When this README and SPEC.md disagree, SPEC.md wins and this file is the bug.

```
packages/cards
├── catalog.json                 card data, proved against SPEC §8 by test/catalog.test.ts (M4-T1, M9)
├── patches                      the catalog's history (R388, §8 below)
│   ├── patches.json             every shipped patch in ship order: { version, date, title, source, notes }
│   ├── <version>.json           the whole catalog as that patch left it, v0.1.0 … v0.2.0
│   ├── shipped.json             every shipped patch's { version, commit, blob }
│   └── pending/                 one fragment per patch being built: { version, title, sources, notes, cards }
├── src
│   ├── catalog-data.ts          the ONE reader of catalog.json: CATALOG, CATALOG_VERSION, cardDef(id)
│   ├── query.ts                 SPEC §5.1's catalog.query — the only random-pool source
│   ├── index.ts                 CARDS, registerAll() — the registry (M4-T2)
│   └── scripts
│       ├── _generated.ts        GENERATED list of the script files present — never edit by hand
│       ├── NNN-slug.ts          one Core card, one file: { def, base, radiant }
│       ├── classic/             one Classic card per file, named the same way
│       └── classic-plus/        one Classic+ card or token per file (tokens NNN-k-slug.ts, AI cards t-ai-NN-slug.ts)
├── scripts                      tooling (node + fs live here, never in src/)
│   ├── naming.ts                the id <-> filename convention every script shares, set folders included
│   ├── gen-registry.ts          rewrites src/scripts/_generated.ts from the three folders
│   ├── missing-tests.ts         prints catalog ids with no test file (M4-T3 acceptance)
│   ├── validate-catalog.ts      catalog data checks, counted per set (M4-T1)
│   └── …                        the `patches` script (§8) and the `loc` generator (§1)
└── test
    ├── _harness.ts              scenario() — the only way a card test builds a game (M4-T3)
    ├── globalSetup.ts           regenerates the script barrel before every test run
    ├── catalog.test.ts          SPEC §8 as a fixture table, counted per set (M4-T1)
    ├── game-summary.test.ts     the engine's summarizeGame over real games (R376)
    ├── query.test.ts            the §5.1 pools (M4-T2)
    ├── references.test.ts       each entry's `refs` against its texts (R279), and the rules-words list (R381)
    ├── registry.test.ts         every catalog id has a script, every script a catalog entry (M4-T2)
    ├── NNN-slug.test.ts         one Core card, one test file
    ├── classic/                 one Classic card per test file, named as its script
    └── classic-plus/            one Classic+ card or token per test file, named as its script
```

## 1. The card-file contract

**One card, one file, one test file.** (CLAUDE.md rule 6.) The file name is the SPEC §5 index,
three digits, plus a slug, in its set's folder: Core at the top of `src/scripts/` and `test/`,
Classic in `classic/`, Classic+ in `classic-plus/`.

| card | script | test |
| --- | --- | --- |
| #1 Big D-fender (`core-001`) | `src/scripts/001-big-d-fender.ts` | `test/001-big-d-fender.test.ts` |
| #43 Big Felinor (`core-043`) | `src/scripts/043-big-felinor.ts` | `test/043-big-felinor.test.ts` |
| #51.1 KY's Empty Notebook (`core-051-1`) | `src/scripts/051-1-kys-empty-notebook.ts` | `test/051-1-kys-empty-notebook.test.ts` |
| Rush Token (`core-t-rush`) | `src/scripts/t-rush.ts` | `test/t-rush.test.ts` |
| Ghoul Token (`core-t-ghoul`) | `src/scripts/t-ghoul.ts` | `test/t-ghoul.test.ts` |
| C #43 Plague Nuke (`classic-043`) | `src/scripts/classic/043-plague-nuke.ts` | `test/classic/043-plague-nuke.test.ts` |
| C+ #12.1 Devour (`classicplus-012-1`) | `src/scripts/classic-plus/012-1-devour.ts` | `test/classic-plus/012-1-devour.test.ts` |
| T-AI-1 Helpful Assistant (`classicplus-t-ai-01`) | `src/scripts/classic-plus/t-ai-01-helpful-assistant.ts` | `test/classic-plus/t-ai-01-helpful-assistant.test.ts` |

**Ids and indices** (SPEC §5). A card's id is its set and its index: `core-043`, `classic-043`,
`classicplus-043`; a token a card defines adds its number (`core-051-1`, `classicplus-012-1`), and a
shared token is named (`core-t-rush`; the ten AI cards `classicplus-t-ai-01` … `-10`). `classicplus`
has no hyphen inside the set part, so an id still splits one way. An `index` (`"43"`, `"12.1"`,
`"T-AI-1"`) is unique only within its set, so nothing looks a card up by index alone: by id, or by
`(set, index)` — `cardDefByIndex(set, index)` here, `defByIndex(set, index)` in the engine.

`scripts/missing-tests.ts` is the authority on the pairing: for every catalog id whose test file it
cannot find it prints the id, the card name and the exact path it expected, and it prints nothing
when all 317 are covered. Run it to learn what to call your files:

```
pnpm --filter @jackioh/cards run missing-tests | grep core-043
core-043  Big Felinor  ->  test/043-big-felinor.test.ts
```

**The exports.** Exactly three, per SPEC §10.9:

```ts
// src/scripts/043-big-felinor.ts
// #43 Big Felinor (SPEC §8.2). Destroys every non-Felinor on both sides; radiant: enemy side only.
import { destroy, targetsInScope } from "@jackioh/engine/effects";
import type { Effect, EffectContext, Script } from "@jackioh/engine";
import { cardDef } from "../catalog-data";

export const def = cardDef("core-043");

export const base: Script = {
  cry: (ctx) => [/* Effect[] */],
};

export const radiant: Script = {
  cry: (ctx) => [/* Effect[] */],
};
```

A script in a set folder is one level deeper, so it imports `cardDef` from `"../../catalog-data"`
and names its own id: `cardDef("classic-043")`, `cardDef("classicplus-012-1")`.

- **`def` always comes from `cardDef(id)`.** Never retype stats, cost, tags or rarity in a
  script file: `catalog.json` is the data and `test/catalog.test.ts` is what proves it against
  SPEC §8. A hand-written `def` is a second source of truth and will be rejected in review.
- **The catalog entry is the printed card.** `radiant.text` is the Radiant face written out in full
  (SPEC §8's cell read by its Conventions, R277), never shorthand, because the client prints it
  whole and marks what differs from `base.text`; every Radiant face meets R275's standard
  (`test/radiant-standard.test.ts`, `docs/radiant-audit.md`); and `refs` lists every card or token
  the entry's texts name, by id (R279) — `test/references.test.ts` proves it against the texts both
  ways, so a text that names a card must list it.
- **The fields patch v0.2.0 added to an entry** (SPEC §5), all of them data:
  - `printedRarity` — the rarity printed on a token (every Classic+ token but the ten AI cards):
    display only, for the card frame and the summon sting. A token's `rarity` stays `"Token"`, so no
    pool ever finds one by rarity.
  - `params` — the numbers Degrade, Upgrade and KY's Constant may change (R386), per face:
    `[{ key, base, radiant, better: "up" | "down", step?, min?, max? }]`. A face's text writes each
    one as `{key}`; the view carries the instance's current value and the client fills it in, as it
    prints `preview` (R280); R277's diff reads each face with its own values filled in. A script never
    writes a declared number as a literal: it reads `param(ctx, key)` (§1's read surface). With no
    `step` the step is 1 for a number up to 5, 2 for 6–12 and a quarter (rounded) above that, and an
    amount never drops below 1. Every Classic and Classic+ card declares its numbers (SPEC §8.6 and
    §8.7 end each Engine cell with them, "Tunes: …"); a Core card that declares none simply offers a
    Degrade fewer options.
  - `loc` — the lines of code of the card's own script file: its non-blank, non-comment lines,
    imports excluded. A `scripts/` generator writes it into `catalog.json` and a test holds it
    current, as `_generated.ts` is held, so a script edit that moves the count fails until the
    generator runs again. It is public (the inspect overlay prints it), C #48 Hired Shrimp, C+ #44
    Simplicity Audit and C+ #45 Complexity Audit compare it, and a fused card's `loc` is its
    ingredients' sum. It is card data, so a refactor that moves a card's `loc` is a balance change and
    lands in a patch (§8).
  - A face's own `type` (`CardFace.type`) — C+ #22 Blood Moon's Radiant face is a Field Trap. A card's
    type is its running face's (§5.2), so a Blood Moon made Radiant in hand is a Field Trap there, to
    pools and filters too.
  - `xStats` (`CardFace.xStats`) — stats printed in X, C+ #69 Buff Billy's 3X/3X → 7X/7X: the Unit is
    summoned with `statsOverride` from its played X, which is at least 1 (R348).
- **Tags.** Patch v0.2.0 adds `Book` (every "Book of …" card of both new sets), `Pancake` (C+ #12,
  #13 and the eight Pancake tokens) and `AI` (the ten AI cards), and every tag list is the designer's
  as written. `Jlockeed` is one tag on five cards, Core #13 and #14 and C+ #48, #51 and #52 (whose
  names keep the designer's "Jlockheed"), and since C+ #48 and #52 draw "a random Jlockheed card"
  from it, it is a pool as well as a filter (R278).
- **Names** (R381). No two cards share a name: Classic #55 is Book of Wildfire and Classic #72 Grand
  Counterspell, so "Book of Flame" and "Counterspell" each name one card. Cards named like rules words
  keep their names (C #10 Exile, #30 Recycle, #36 Burn, #57 Echo); `test/references.test.ts` holds a
  named list of rules words it never reads as a reference to a card unless that entry's `refs` lists
  the card, so "Exile" in a Core text is the verb and a card's `refs` stays curated.
- **House style** (R432, over R366). "(N) Cost" is the noun — "a (1) Cost or less card", "(4)+ Cost
  cards", "Discover a (2) Cost card", "Face-down trap, (2) Cost" — and "costs (N)" the verb — "costs
  (1) less", "costs (0)"; never "Cost (N)", "N-cost" or "costing N". Players read "Deck" and "Tribute"
  (R373). Every text in `catalog.json`, Core's included, follows it since patch v0.2.0.
- **`base` and `radiant` are both required**, even when they are the same object — a card whose
  Radiant face differs only in what the engine reads off the catalog or its config (its stats and
  keywords, or #38's Combo multiple, `QUICKSTRIKER_COMBO_MULTIPLE`) runs the same script on both
  faces, so write `export const radiant = base;` and say so in a comment.
- The `Script` shape is `{ cost?, cry?, death?, startOfGame?, resume?, delayed?, setStat?,
  startOfTurn?, endOfTurn?, aura?, triggers?, activate?, onPlayHook?, handTriggers?, staticFlags?,
  targets?, modes?, conditionMet?, preview? }` (`packages/engine/src/script.ts`, SPEC §10.9). `resume` is the named
  continuation a prompt answer re-enters (R113), `delayed` the hook a scheduled effect lands on
  (R126), and `setStat` layer 2's stat hook (R116) — 13 card files already export one of them. A spell's script hangs off `cry`: that is the
  on-resolve hook for a Spell as well as the Cry of a permanent. `activate` is a card's Activate
  ability (R384), of which #98 Heroic Power's power was the first.
- `conditionMet` is R195's yellow glow: a pure read of `{ state, self, controller, radiant, zone,
  yourTurn }` that returns whether the card's printed condition holds now (`zone: "hand"` as if
  played now, `"field"` as the card in play reads it), built on the same local predicate the card's
  own resolution uses so the two cannot disagree; it never writes, never draws from `rng` and never
  reads `state.active` (use `yourTurn`).
- `preview` is R280's number a formula comes to now: a pure read of the same context (the card's
  own controller, its running face, the zone, `yourTurn`) that returns `{ label, value }[]`. Each
  `label` is an exact substring of the running face's catalog text — the formula as printed,
  `"Fib(times played + 1)"` — because the client prints `{value}` right after its first occurrence; each
  `value` is what the formula comes to if the card resolved now, computed by the same local function
  the card's own hook deals or gains with. `viewFor` shows it wherever the viewer may read the card,
  the other seat's public cards included, so a hook reads only public facts and the card's own face
  and counters — a hero's health, a pile's size, the plays this turn, the active player's mana —
  never a library's contents or order or a hand's cards, and never `state.active`. An empty list is
  no preview. A value the text names by a word carries it as `display` beside the number (#93's
  grade letter, R372). The Core cards with one are #18, #31, #38, #40, #70, #91 and #93 (#93 on the
  field only); a fixed number already on the face (#92's stats, #100's cost, #89's hand stats) and
  an X chosen at play (#24, #74) have none. A declared number (`params`) is not a preview: the view
  carries its value as `{key}`.

**Purity (CLAUDE.md rules 4 and 5).** A hook is `(ctx: EffectContext) => Effect[]`. It reads
`ctx` and returns effects; it never assigns to `ctx.state`, never calls an engine mutator, never
calls `Math.random` or `Date`, and never awaits. Every effect comes from
`@jackioh/engine/effects` — that barrel is the entire vocabulary a card file has
(`packages/engine/src/effects/index.ts` lists all of it and says which SPEC §6.3 verb each name
implements). If the verb you need is not there, the effect is missing from the engine: report it,
do not reach into state. A card-specific system (C #57 Echo's copied text, C #90 In Too Deep's quests,
C+ #29's last boards, C+ #35 Rollback's snapshots, C+ #42 KY's Test's problems, C+ #62 KY's Papaya's
curve, C+ #27's perfect hand) is an engine module under `packages/engine/src/subsystems/` with its
own engine tests through fixture scripts, never logic in the card file.

**The read surface.** §10.9 lets a hook READ state to compute an effect's argument, and BUILD M3-T1
adds how: `grep -r "state.players[" packages/cards` must come back empty, so a card file names the
FACT it needs and never a field of `PlayerState` — otherwise the state model cannot change shape
without editing every card that reads it. Verbs come from `@jackioh/engine/effects`; facts come
from `@jackioh/engine`, except `instanceOf`, which the effects barrel exports because it resolves a
`TargetSpec` exactly as the verbs do. These are all of them:

| read | what it answers |
| --- | --- |
| `heroOf(state, player)` | `{ health, armor }`, copied (#68's threshold, #70's missing health) |
| `zoneCards(state, player, zone)` | one off-field pile as a `readonly` copy, `library[0]` first (#30, #51, #83) |
| `zoneCount(state, player, zone)` | how many cards are in it (#70's exile, #71's libraries, #76's hand) |
| `cardsPlayedThisTurn(state, player)` | §10.5 step 4's counter, already counting the card being played (#38) |
| `unspentManaOf(state, player)` | the mana a player holds now, which `turnEnded.unspentMana` is as the turn ends (#18's preview) |
| `playedIdsThisTurn(state, player)` | the instance ids played this turn, in order, copied (#39) |
| `playedEarlier(state, player, card)` | §6.2's Combo count: the plays before this card's play, at play time, so a card its own resolution casts is not one; for a card in hand, every play so far; for `null` (a `ctx.self` that has ceased to exist), every play but the latest (#10) |
| `wasPlayedThisTurn(state, player, card)` | the one-shot gate §5.1's "return to hand" spells need (#23, #24, #31) |
| `activeUnitsOf` / `dormantUnitsOf` / `cardAt` / `slotsOf` / `slotOf` | the field, by lane (§3.2, R13) |
| `faceOf` / `statsWithBuffs` / `unitView` | a unit through the §10.4 layers — never off the instance |
| `defOf` / `printedCost` / `effectiveCost` / `queryCost` | a definition and R65's two costs |
| `findInstance` | an instance id, wherever the card has since landed (R98) |
| `instanceOf(ctx, spec)` | the card a `TargetSpec` names on the stay the run aimed at — a chosen card on the stay its prompt or the play offered it on (R174) — or `null` for a hero, for nothing, or for a card buried under a Stack pile (§3.2, R13) (#22's meal) |
| `recalled(ctx, key)` | what the running card remembers under a key (`remember`'s write) — on a fused card, its own ingredient's (R102, #22) |
| `killerOf(state, card)` | the Unit that destroyed a card, read off the card as its Death hook sees it (R42's killer), while that Unit still acts on the field; else `null` (#86, R361) |
| `afterAttackOf(ctx)` | in an `afterAttack` hook, its combat's facts: `{ targetId, destroyedIds, survived, forced }` — the Units the attacker's own hits destroyed, and whether it is still on the stay it attacked from (#32, R426) |
| `param(ctx, key)` | the running card's current value of a number its entry declares (`params`, R386): its running face's `base` or `radiant` value as Degrade, Upgrade and KY's Constant have left it on the instance |
| `ownCost(state, card)` | the card's own cost — `costOverride` or printed, plus `costMod`, no player discount — which a Degrade, an Upgrade or KY's Constant moves; `null` for an X-cost card (R65, R386) |
| `costNow(state, card)` | what a card costs wherever a rule compares or counts costs: an X card on the field its played X, elsewhere 0; a hand card at its hand cost; a floor holds (R396, R455; C #10, #18, #25, #32, #39) |
| `maxManaOf(state, player)` | §2.3's max mana, which a refresh fills to (C #36's Radiant face) |
| `subsystems.activationPaid(ctx)` | in an Activate's effect list, what the activation paid: the ability's id and the Units its Tribute cost took, as they stood (R384, C #21) |

`zone` is `"hand" | "library" | "graveyard" | "exile"`; the field is not a pile, so read it by lane.
Every one of these but `findInstance`, `instanceOf` and `killerOf` returns a number, a boolean or a
fresh `readonly` array, so a card cannot write the game through a value it read; those three hand
back the card itself, which a card file reads and never writes (CLAUDE.md rule 5). Board facts live in
`packages/engine/src/query.ts` (the read half of the surface, next to `src/effects/index.ts`, the
write half); if the fact you need is not there, it is missing from the engine — extend that module
and test it, do not reach into `state.players`.

Randomness goes through `ctx.rng` (seeded, cursor in state) and every pool through
`catalog.query` — see §3 below. A player choice is either a play-time choice you *declare*
(next paragraph) or a `PendingChoice` opened by a `choose*`/`discover*` effect; a card file never
asks a question inline.

**Play-time choices (R81).** Zone, X, embiggen, Tribute, and the targets and modes a card needs for
its own play travel in the `play` action, not in a prompt. Declare them so `legalActions` and the
client can build the pickers without running the script:

```ts
export const base: Script = {
  targets: [{ kind: "target", min: 1, max: 1, filter: { side: "enemy", of: ["unit"], notTags: ["Human"] } }],
  cry: (ctx) => [destroy({ of: "chosen" })],   // ctx.targets[0] is the declared pick
};
```
`kind` is `"target" | "hand" | "zone" | "tribute"` for `targets` and `"mode" | "direction"` for
`modes`; a declared `hand` or `zone` pick arrives in `ctx.targets`, a `direction` in `ctx.modes`
(R81). An Activate ability declares its targets and modes the same way, and they travel in the
`activate` action as a play's travel in `play` (R384). Every choice made *during* resolution —
Discover, a chained step, an Echo repeat, a trigger — is a prompt instead, and prompts are state
(SPEC §10.6).

**Instance memory.** Anything a card must remember lives on the instance (`ctx.self.memory`, via
the `remember`/`rememberRandom` effects) so Fuse, copies and replay stay trivial (SPEC §10.1).

## 2. Registering a card — there is nothing to wire

`src/index.ts` is a contract file shared by every card, the harness and the server. **Do not edit
it to add a card.** Drop your file in `src/scripts/`, `src/scripts/classic/` or
`src/scripts/classic-plus/` and the registry picks it up: `src/scripts/_generated.ts` is regenerated
from the three folders by `scripts/gen-registry.ts`, and the cards project runs that automatically
before every test run (`test/globalSetup.ts`). To regenerate by hand:

```
pnpm --filter @jackioh/cards run gen            # rewrite src/scripts/_generated.ts
pnpm --filter @jackioh/cards run missing-tests   # catalog ids with no test file (silent = done)
```

`tsc` does **not** regenerate the barrel — a missing import is not a type error — so if you
typecheck without running the tests, run `gen` first or your card is silently unregistered.

What the registry exports, for the server, the client and the tests:

| export | meaning |
| --- | --- |
| `CATALOG`, `CATALOG_IDS`, `CATALOG_VERSION` | all 317 defs from `catalog.json`, script or no script, and the latest patch's version (R388) |
| `cardDef(id)`, `cardDefByIndex(set, index)` | one def, throwing rather than returning `undefined` |
| `CARDS` | `Record<catalogId, { def, base, radiant }>` — one entry per script file present |
| `registerAll()` | `registerCatalog(CATALOG, CATALOG_VERSION)` then `registerScripts(...)`; idempotent |
| `query`, `catalog` | SPEC §5.1's pool query, re-exported from `./query` |

A card with no script file yet is still in `CATALOG` and still playable as data: the engine falls
back to `EMPTY_SCRIPT`. `test/registry.test.ts` is the M4-T2 gate that every catalog id has a
script and every script a catalog entry; it flips to strict when M4-T4 lands the last file.

## 3. `catalog.query` — the only random pool (SPEC §5.1)

```ts
import { catalog } from "../query";        // or: import { query } from "../query"; "../../query" from a set folder
catalog.query({ type, cost, costRange, tags, notTags, rarity, set, excludeDefId, withTokens })
catalog.pool("core-057", { tags: ["KY"] }) // the same, with your own catalog id excluded (R387)
catalog.cost(def)                          // R65's out-of-play cost: X reads 0, embiggen reads base
catalog.trapTypes                          // ["Trap", "Field Trap"]
```

`src/query.ts` is a thin typed wrapper over the engine's one filter
(`packages/engine/src/catalog.ts`), so there is exactly one filter in the codebase. It also takes
`defId` and `token` for a pool a card names card by card. `set` takes one set or several, and a
pool that names no set draws from every set: there is one format (R380).

The pools the spec pins down, as the argument object to write:

| pool | write | result |
| --- | --- | --- |
| any random card | `catalog.query({})` | the 268 non-token cards of every set (R380) |
| a pool its card limits to sets | `catalog.query({ set: ["Classic", "Classic+"] })` | only those sets: C+ #27 Zephrys Zealotism, #73.1 Classic Golem and #75.1's Pack name Classic and Classic+, C+ #73 names Classic; #82 KY's Trial and #97 Zephyrs name Core (`set: "Core"`) |
| KY (#57 Conjure KY) | `catalog.pool("core-057", { tags: ["KY"] })` | Core #31, #51, #82 and C+ #41, #42, #62 |
| a (1) Cost Trap (#67 Zoomerbin Oomen) | `catalog.query({ type: catalog.trapTypes, cost: 1 })` | Core #18, #41, #60, #71, #96 and C+ #22 Blood Moon (#85 costs (2) since patch v0.1.1; radiant #67 drops the cost and reaches it) |
| Transmogulate (#83, R35) | `catalog.pool("core-083", { rarity: "Legendary" })` | every non-token Legendary but #83: Core's six (#52, #85, #87, #92, #93, #95), Classic's nine and Classic+'s thirteen |
| Call to Chaos (#95) | `catalog.query({ tags: ["Call to Chaos"] })` — **no** `excludeDefId` | #95 and C+ #73, the Classic+ Edition: the §5.1 exception, a text that names a pool holding itself (R28, R387) |
| Fruit (C+ #58 Fruit Basket) | `catalog.pool("classicplus-058", { tags: ["Fruit"] })` | the non-token Fruit cards but Fruit Basket, plus the five Grapes (C+ #65.1–#65.5), which no other pool holds but the next one (R382) |
| every card and token (C+ #23 Dropshipping) | `catalog.pool("classicplus-023", { withTokens: true })` | every card and every token of every set — a Grape, a Loser, an AI card — but Dropshipping (R382) |

**Footgun:** `type: "Trap"` matches the `type` field exactly and so drops the Field Traps (Core
#18, #71, and the Classic and Classic+ Field Traps, C+ #22's Radiant face among them). Everywhere
SPEC says "Field Trap counts as Trap" — #51's type choice, #85's type match, R35's same-type
replacement — pass `catalog.trapTypes`.

Guarantees a card file may rely on, all proved in `test/query.test.ts`:

- **No tokens** unless the query asks for them (`tags: ["Token"]`, `rarity: "Token"`,
  `token: true`, `withTokens: true`, or naming members through `defId`), with R382's one exception:
  a Fruit pool holds the five Grapes. #51.1 is "absent from every random pool" for exactly this
  reason.
- **Never the generating card** (R387): pass your own catalog id, which `pool(ownId, …)` does for
  you. It is the id, never the index, because an index repeats across sets. It holds on every path a
  card makes a card from a pool — added to a hand, shuffled in, summoned, Discovered, cast,
  transformed into, fused in, or replacing another card — and a fused card excludes every one of its
  ingredients. The one exception is a text that names a pool holding itself (the Call to Chaos
  family). A copy is not generation: "summon a copy of this", Echo repeats and C+ #14 Forever& are
  unaffected. A pool that can offer the card that made it is a bug, and a sweep test runs every
  generating script with a seeded rng and asserts its own id never comes out.
- **Costs read out of play (R65)**: an X-cost card queries as cost 0 and an embiggen card as its
  base price.
- **Deterministic order** (set by set, Core first, each by §5 index ascending), so
  `ctx.rng.pick`/`shuffle` over the result replays identically (§9.3, R60). Never sort, filter or
  de-duplicate a pool yourself afterwards: that is how two card files end up disagreeing about what
  "a random unit" means.

`test/query.test.ts` names the pools the spec pins down — the KY pool, the trap pool (Field Trap
counting as Trap), the Transmogulate Legendary pool (R35), the pools across sets (R380), the Fruit
pool (R382) and the cost brackets — with the exact argument object each one needs. Copy from there
rather than inventing a filter.

## 4. The test harness

Every card test builds its game with `scenario(...)` from `test/_harness.ts` and with nothing else:
it places real instances through the engine's own zone functions, so a test can never assert
against a state the engine could not have produced. Importing the harness also calls
`registerAll()`, so the real catalog and every registered script are live. A test in a set folder
imports it as `"../_harness"`.

```ts
import { describe, it } from "vitest";
import { scenario } from "./_harness";

describe("#43 Big Felinor", () => {
  it("destroys every non-Felinor on both sides and spares Felinors", () => {
    scenario({ p1: { hand: ["core-043"], field: ["core-056"] }, p2: { field: ["core-012", "core-025"] } })
      .play("core-043")
      .expectInZone("core-056", "graveyard")
      .expectEvents("cardPlayed", "destroyed");
  });
});
```

### `scenario(opts)`

```ts
scenario({ seed?, p1?: SideSetup, p2?: SideSetup, turn?, active? })

type DefRef = { def?: string; defId?: string };      // aliases; exactly one, both-and-differing throws

type CostSetup = { costMod?: number; costOverride?: number };   // R78: they persist in every zone

type SideSetup = {
  hand?:      readonly (string | (DefRef & CostSetup & { radiant?: boolean }))[];
  field?:     readonly (string | (DefRef & CostSetup & { radiant?: boolean; position?: "ATK" | "DEF"; damage?: number; lane?: number; stack?: boolean; counters?: { plague?: number; grade?: number } }))[];
  backrow?:   readonly (string | (DefRef & CostSetup & { radiant?: boolean; faceUp?: boolean; lane?: number }))[];
  library?:   readonly (string | (DefRef & CostSetup & { radiant?: boolean }))[];
  graveyard?: readonly (string | (DefRef & CostSetup & { radiant?: boolean }))[];
  exile?:     readonly (string | (DefRef & CostSetup & { radiant?: boolean }))[];
  health?: number; mana?: number; armor?: number;
};
```

A bare string is the same as `{ def }`, every array is `readonly` so an `as const` fixture is
assignable as it stands, and any pile entry may be an object when it needs `radiant: true` — which
is how a test puts a Radiant card on top of a library (#21, #23).

- Lanes are **1-based**; a `lane` given places there, omitted fills left to right (R64).
- `stack: true` (unit zones only) builds a **§3.2 Stack pile**: that entry buries the card already
  in its lane instead of taking a lane of its own, so its `lane` may repeat one an earlier entry
  pinned and, with no `lane` of its own, it lands on the entry before it. Later entries go on top,
  the way a play would put them, so the pile reads top-first and the list reads bottom-first:
  `field: ["core-043", { def: "core-092", stack: true }]` is a Felinor Fiender on top of a dormant
  Big Felinor in lane 1 (R13). A repeated lane without the flag is still the "two cards were given
  lane N" error, and `stack: true` with nothing under it throws rather than quietly making no pile.
- `counters` seeds §10.1's instance counters (`plague` for #91, `grade` for #93), and `costMod` /
  `costOverride` seed §2.3's cost layers in any zone (R78), which is what a ruling read at
  resolution rather than off the printed cost (R65, R66) needs in order to be set up at all.
- `faceUp` is written exactly as given, so `faceUp: false` reads back `false` rather than
  `undefined` — a face-down Trap is a state a test asserts (R33).
- `library[0]` is the **top** of the library: the next card drawn.
- The game starts in the **main phase** of `active` (default `p1`) on `turn` (player-turns, 1-based:
  turn 1 is p1's first, turn 2 is p2's first). The default `turn` is **9** — a mid-game board where
  both sides sit at `MAX_MANA` 4/4 — and the default `seed` is `"jackioh-harness"`. `turnsStarted`
  is derived from `turn` and `active`, so the mana refresh is the real one; `mana` overrides current
  mana without raising max (SPEC §2.3 allows current above max). No mulligan runs, no start-of-turn
  triggers fire and nothing is drawn during setup; the engine's state check runs once at the end of
  it. Units placed by `field` are not summoning sick and have both exertions unspent.
- `reduce` clones the state, so a `CardInstance` captured before a step is a stale object after it.
  Passing a stale instance to a harness method is fine (it re-resolves by id), but read values back
  with `s.card(inst)` / `s.unit(p, lane)` rather than off the object you are holding.
- SPEC §2.5: a turn with nothing meaningful left auto-ends itself, so `endTurn()` can cascade
  several turns forward when both hands are empty. Give each side a card in hand or a unit that
  could switch position when a test crosses a turn boundary.
- A card reference (`string`) may be a catalog id (`"core-043"`, `"classic-043"`), a §5 index
  (`"43"`, `"51.1"`, `"T-rush"`), a card name (`"Big Felinor"`) or an instance id (`"c7"`). An index
  is unique only within its set, so a bare index is for Core cards: name a Classic or Classic+ card
  by its id or its name. With several copies of one def in play, hold the `CardInstance`
  (`s.unit(...)`, `s.hand()[0]`) instead of the string.

### Steps and assertions

| member | what it does |
| --- | --- |
| `play(card, { zone, x, embiggen, targets, modes, tributes })` | the `play` action; `zone` is a lane number. Throws with the engine's message when the play is illegal — that is how a test asserts a refusal: `expect(() => s.play(x)).toThrow(/tribute/)` |
| `attack(attacker, target \| "hero")` | the `attack` action |
| `answer(selection)` | answers the open prompt; a string is an option key (or label/instance id). Call it again for the next prompt in a chain (SPEC §10.6) |
| `endTurn()` | the `endTurn` action: end-of-turn triggers, the trap window, delayed effects, cleanup — and the opponent's turn then starts, so two `endTurn()` calls come back to your own next turn |
| `startTurn()` | runs the engine's start of turn for the player who is active now: turn counter, mana refresh, start-of-turn triggers, one draw, back to main |
| `state`, `events`, `lastEvents` | the live state, every event so far, and the last step's events |
| `view(player?)` | `viewFor` — what that player is allowed to see (§10.8) |
| `unit(player, lane)`, `backrow(player, lane)`, `hand(player?)`, `pile(player, zone)`, `card(ref)` | board and zone readers, returning live `CardInstance`s |
| `stats(card)` | the §10.4-computed `{ attack, maxHealth, health, keywords, armor, position }`. A `CardInstance` has none of those: `health`, `armor` and `keywords` are layers, so read them here (or through `expectStats`), never off the instance |
| `expectInZone(card, "hand" \| "library" \| "graveyard" \| "exile" \| "field" \| "gone")` | `"gone"` is R11's token that ceased to exist |
| `expectStats(card, { attack, health, maxHealth })` | computed through the §10.4 layers, never the printed def |
| `expectEvents(...types)` | those event types appear in that order in `events` (a subsequence) |
| `expectHealth(player, n)`, `expectMana(player, n)` | hero health and current mana |

Everything is chainable and every assertion failure names the card, what was expected and what was
found. The harness is seeded and pure: same seed plus same steps equals the same event log, which
is what makes the fuzz and replay gates meaningful.

## 5. What a card's test file must cover

For **base and radiant separately**: one case per behaviour named in the card's SPEC §8 row, plus
every must-pass case in its set's BUILD table row for that card (M4-T4 for Core, M9's Classic and
Classic+ tables for the new sets). A Classic or Classic+ card also covers every reading its §8.6 or
§8.7 Engine cell makes (targets, timing, what happens at 0, hidden information, caps, what an empty
pool or board does), and reads each number its entry declares through `param(ctx, key)`, never a
literal. Name a test after the ruling it pins down when there is one —
`it("R64 the copy lands in the leftmost free zone", …)` — so a ruling change has a failing test with
its name on it (CLAUDE.md rule 3).

A card whose script declares `conditionMet` (R195's yellow glow) also proves both answers of its
hook against the branch its own resolution then takes (SPEC §10.9). Those proofs live together in
`test/condition-active.test.ts`, not in the card's own file, because they share one harness for
reading `conditionActive` off `s.view(...)`; the card's own test file names that file in its header.
The same file pins the set of cards that declare the hook to R195's list, so a card that adds one
fails `pnpm test` until its proof and the ruling's list are updated.

A card whose script declares `preview` (R280) proves, on both faces, that each value its view
carries is what its own resolution then deals or gains, that each label sits in its face's text,
and what the hook may read. Those proofs live together in `test/preview.test.ts`, which also pins
the set of cards that declare the hook to R280's six and R372's #93 and fences every library and
hand off from the hooks; the card's own test file names that file in its header (#93's values are
proved in its own test file, since its hook answers on the field only). A Classic or Classic+ card
that declares either hook joins that file's list and the ruling's in the same change.

A card is done when its tests are green, `pnpm lint` and `pnpm typecheck` are clean, and the fuzz
gate still passes with the card in the pool.

## 6. Wave order (BUILD M4-T4, and M9 for the new sets)

Within a wave, go in index order. A wave is done when every card in it passes its tests and the
M4 fuzz gate still passes with those cards in the fuzz deck pool; do not start the next wave first.

**Wave 1 — keywords and single primitives (45):** #1, #2, #5, #6, #7, #8, #10, #11, #13, #14, #15,
#16, #17, #19, #20, #25, #26, #28, #29, #34, #37, #40, #42, #43, #44, #47, #48, #49, #51.1, #53,
#54, #56, #57, #58, #62, #63, #68, #69, #70, #72, #74, #88, #90, #93.1, #95.1 — plus the four
shared tokens (`t-rush`, `t-sheep`, `t-felinor`, `t-bread`), The Coin (`t-coin`, which §2.1's
setup deals to the seat going second, R244) and the Ghoul Token (`t-ghoul`, patch v0.1.1, R353),
which have no §8 row of their own.

**Wave 2 — stored state, prompts, delayed and cross-turn effects, traps (49):** #3, #4, #9, #12,
#18, #21, #22, #23, #24, #27, #30, #31, #32, #33, #35, #36, #38, #39, #41, #45, #46, #50, #51, #55,
#59, #60, #61, #64, #65, #65.1, #66, #67, #71, #73, #75, #76, #77, #78, #79, #80, #81, #82, #84,
#86, #89, #90.1, #91, #94, #100.

**Wave 3 — subsystems (11):** #52 (rotation), #83 (Transmogulate), #85, #87 (Pocket Chaos), #92
(Felinor Fiender/Stack), #93 (Combo-Index), #95 (Call to Chaos), #96 (My Pawn/lethal), #97
(Zephyrs/scorer), #98 (Heroic Power), #99 (Craft a Card).

**Classic and Classic+ (BUILD M9)** come after the engine's new keywords and systems, each already
proved by engine tests through fixture scripts, and go in the same three waves: first the
keyword-only and one-primitive cards (C #3, #12, #16, #24, #26, #41, #73, #82; C+ #6, #10, #15, #16,
#20, #28, #53–#57, #59, #67, …), then the cards built on one or two of the engine's new systems, and
last the card-specific subsystems — C #48 Hired Shrimp, C #57 Echo, C #90 In Too Deep; C+ #19 League
of Losers and its Losers, #27, #29, #35, #42, #44 and #45 (the Audits), #62, #73 and #74 — each with
its subsystem's own tests.

## 7. Commands

```
pnpm exec vitest run --project cards                 # every card test
pnpm exec vitest run --project cards 043              # card 043 of every set that has one
pnpm exec vitest run --project cards test/classic/    # one set's cards (test/classic-plus/ for Classic+)
pnpm exec tsc -p packages/cards/tsconfig.json         # src + test + scripts
pnpm lint                                             # includes the Math.random / Date ban
pnpm --filter @jackioh/cards run gen                  # rebuild the script barrel
pnpm --filter @jackioh/cards run missing-tests        # M4-T3 gate: silence means covered
pnpm --filter @jackioh/cards run patches <version> <date> "<title>" # claim a patch (§8)
pnpm --filter @jackioh/cards run patches check            # prove every catalog change is claimed once
pnpm --filter @jackioh/cards run patches ship             # promote pending fragments in ship order
```

## 8. Patches and the catalog version (R388, R635)

Every change to card data is a patch, and every patch is kept, so an older version of any card can
still be read. Several patches are built at once, so a patch is claimed on its branch and shipped
after it merges — never by editing the history on the branch.

- **Patches are data.** `patches/patches.json` lists every shipped patch in ship order as
  `{ version, date, title, source, notes }`, `patches/<version>.json` is the whole catalog as that
  patch left it (snapshots, not diffs), and `patches/shipped.json` carries each patch's
  `{ version, commit, blob }`: the commit that shipped it and its snapshot file's git blob hash.
  A promoted entry also records that first-parent commit in `commits` and
  `reconstructed: false`; historical entries stay byte-identical.
  A generated index maps each card id to the versions in which it changed; the client's History
  section and Patch notes page read these files (`apps/web/README.md`). Shipped snapshots are
  never amended.
- **The history.** v0.1.0 (2026-09-18: Core as first built, 100 cards and 9 tokens); v0.1.0-r1
  (2026-09-22: the Core Set balance changes of issue #1); v0.1.0-r2 (2026-09-24: #95's text, and The
  Coin added); v0.1.0-r3 (2026-09-25: the Radiant pass, R275–R279: Radiant faces, the Jlockeed tag and
  `refs`); v0.1.1 (2026-09-27, issue #27: the Ghoul Token added and 105 entries changed); v0.2.0
  (issue #40: Classic, Classic+, the new keywords, Core's pools across sets and the Core card patches
  below). Everything before v0.1.1 was rebuilt from `git log --follow packages/cards/catalog.json` on a
  full clone (a shallow one stops early).
- **The version is the patch.** `CATALOG_VERSION` is the latest patch's version, `v0.2.0`, and a test
  holds `catalog.json` equal to the latest snapshot and `CATALOG_VERSION` equal to its version. A
  patch bumps it everywhere the string lives: `src/catalog-data.ts`; the server's env
  (`apps/server/.env.example`, `render.yaml`); the client's `VITE_CATALOG_VERSION`; and the database,
  where `db:seed-catalog` restamps every `cards` row and `app.settings` (`apps/server/README.md`). A
  version stays opaque (R105): nothing parses or orders one, and the order of patches is
  `patches.json`'s.
- **Claiming one.** `pnpm --filter @jackioh/cards run patches <version> <date> "<title>"` (in `scripts/`,
  where fs is allowed) writes `patches/pending/<version>.json` — `{ version, title, sources,
  notes, cards }`, where `cards` lists the catalog ids the patch creates, changes or removes
  (`--cards` lists them; otherwise they are diffed from the working catalog against the newest
  shipped snapshot). `patches check` (in CI, in the `validate:catalog` step) fails naming the card
  when a catalog change is unclaimed or claimed twice, when a claimed card did not change, or
  when a fragment's version is not a bare patch number. `patches ship` promotes each fragment in
  the order of the first-parent commit that added it — appending the patch (or `<version>b`,
  then `c`, … when that name already shipped), snapshotting that commit's catalog, recording
  `shipped.json`, deleting the fragment and bumping `CATALOG_VERSION`; with no fragments it
  changes nothing. A version's place is fixed by when it lands on `main`, whatever its name
  (R375): v0.2.0 landing after v0.2.5 reads after it.
- **Data, not code.** A snapshot keeps a card's data (its texts, numbers, `params` and `loc`), not its
  script. A patch that changes what a script does is recorded by the card's new text and its ruling,
  and an old log of that card's games replays exactly only under the code it was played with.

Patch v0.2.0's changes to Core, beside the two new sets:

| Card | v0.2.0 |
| --- | --- |
| #65 Masochism Mask | costs (1) |
| #43 Big Felinor, #88 Twisting Nether, #49 Snom Bunny Mind Control, #17 Flood, #34 Collateral Damage | cost (4) each |
| #16 Hit Job | costs (3) |
| #32 Prem Panther | draws only after **it attacks** (a declared or forced attack) **and survives that combat**: 2 for each Unit that attack destroyed, the attacked Unit and, on the Radiant face, its Cleave kills; never while defending and never when it dies in the combat (R426) |
| #41 Sheepish | fires after the played Unit resolves, so its Cry happens, and then transforms it into a Sheep Token; the Radiant face still adds the (0) Cost Lava Golem (R427, rewriting R17's Sheepish half) |
| #22 Carnivorous Cube | its Cry tributes one of your other Units, never a backrow card (R428, rewriting R41) |
| #31 KY's Math Equation | "Deal Fib(times played + 1) damage to a target. End of turn: Return this to your hand with +1 cost (maximum (4))."; Radiant Fib(times played + 3). *Times played* counts this card's plays, the current one included, kept on the instance in every zone as `costMod` is, so the 1st play deals Fib(2) = 1, the 2nd 2, the 3rd 3, the 4th 5, the 5th 8; the return never lifts its cost above (4) (R429, rewriting R25 and R67) |
| #60 Bear Honeypot | stays face-down, neither firing nor consumed, while its controller has no empty, unlocked, unreserved unit zone (R430) |
| #21 Hinder | base: "Cast on draw: Your opponent's next mana refresh is 1 lower. Discard 1." — the caster discards a card of their choice (R16), a hand prompt during the draw (R158), and nothing with an empty hand; the Radiant face is unchanged, with no discard (R431) |
| #95 Call to Chaos | Radiant: "Cast three different random effects" from its list, the shape C+ #73 has (R423) |

The same patch rewrote every text in the house style (R432) and opened every Core pool that names no
set to all three sets (R380): #7, #54, #57, #59, #67, #83, #95, #98 and #99 draw from every set, while
#82 KY's Trial and #97 Zephyrs name Core and keep to it.
