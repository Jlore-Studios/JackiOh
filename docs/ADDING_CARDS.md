# Adding a card

Read this before any card work. It is the procedure for a **standard** card: one whose text is made of
verbs, keywords and filters the engine already has. Anything else is [§6](#6-a-card-the-engine-cannot-express-yet).

It was written by reading C+ #5 Guy Att, #6 Wrong-House Attacker, #77 Anti-Softlock and the Rush Token, grepping every file that names
a card of each kind, and then adding a dummy Classic+ unit (a vanilla (1) 1/1 Human) to a scratch worktree and running every gate. The
files below are what that run named. It could not reach a fully green run, for the reason in [§7](#7-what-cannot-be-done-on-linux).

## 1. The files, in the order to touch them

A Classic+ card is shown. Core cards live at the top of `src/scripts/` and `test/`, Classic in `classic/`.
`NNN-slug` is the card's SPEC index in three digits plus a slug ([cards README §1](../packages/cards/README.md)).

| # | File | What changes | A gate fails without it |
|---|---|---|---|
| 1 | `SPEC.md` §8.7 (§8.1 Core, §8.6 Classic, §7 tokens) | the card's row: the only source of card text (CLAUDE.md) | none |
| 2 | `packages/cards/catalog.json` | the entry, after the set's highest card index and before its shared tokens (`T-AI-…`); a token follows its card | `validate:catalog`, `catalog.test.ts` |
| 3 | `packages/cards/src/scripts/classic-plus/NNN-slug.ts` | the script ([§2](#2-templates)) | `registry.test.ts`, `missing-tests` |
| 4 | `packages/cards/test/classic-plus/NNN-slug.test.ts` | the test ([§2](#2-templates), [cards README §5](../packages/cards/README.md)) | `missing-tests` |
| 5 | `packages/cards/src/scripts/_generated.ts` | **generated**: run `pnpm typecheck`, commit its diff, never edit it | typecheck |
| 6 | `packages/cards/test/catalog.test.ts` | the card's fixture row in `CLASSIC_PLUS`; `RARITY_COUNTS` and `SET_SIZES` (the totals and row counts are their sums) | itself |
| 7 | `packages/cards/scripts/validate-catalog.ts` | the set's `cards` and `rarities`, and `EXPECTED_TAG_COUNTS` for each tag the card has | `validate:catalog` |
| 8 | `docs/radiant-audit.md` | one row `\| <index> \| <name> \| …`; the Radiant face must meet R275 (about twice the base face; doubling stats alone is not enough for a unit with text) | `radiant-standard.test.ts` |
| 9 | `BUILD.md` | the card's must-pass row in the M9 (or M4-T4) table | none |
| 10 | `apps/web/src/audio/voice-lines.json` | the card's lines, in catalog order | `voiceData.test.ts` |
| 11 | the count assertions below | the totals change by one | the tests named |
| 12 | `packages/cards/patches/pending/<version>.json` | the fragment, `run patches …` ([§3](#3-the-patch-and-its-order)); the shipped history and the four version sites move at promotion, not here | `patches check`, `patches.test.ts`, `loc.test.ts` |

**Count assertions** (`grep -rn "\b317\b\|\b268\b" --include=*.ts --include=*.tsx` finds most): `test/query.test.ts` (the
non-token total, the set sizes, `317 - 1`), `test/registry.test.ts` (`CATALOG_SIZE`),
`test/059-unbiased-immigration.test.ts` (the pool without #59), `apps/server/test/api/catalog.test.ts`,
`apps/server/test/db/seed-catalog.test.ts` and `.spec.ts`, `apps/web/src/game/deckbuilder/filters.test.ts` (the pool and a set's
size), and `e2e/cypress/component/deckbuilder-layout.cy.tsx` (`DECKABLE_COUNT`). Since #108 the catalog and pool counts
and the web's patch tests (`patches/source.test.ts`, `routes/patch-notes.test.tsx`, `patches/PatchNotes.test.tsx`) count
`catalog.json` and `patches.json` themselves and need no edit for a new card. The patch-list tests
(`patches.test.ts`, `PatchNotes.test.tsx`, `source.test.ts`, `patch-notes.test.tsx`) pin only the history shipped before
yours — a pending-claimed card needs no edits there (R646). What stays hand-kept is the proof:
`catalog.test.ts`'s `RARITY_COUNTS` and `SET_SIZES` (its totals and row counts are the sums) and
`validate-catalog.ts`'s `SETS` and `EXPECTED_TAG_COUNTS` (`patches.test.ts`'s `VERSIONS` is derived from
`patches.json`, with the shipped prefix pinned).

Also grep the Markdown for the stated totals (`268 cards`, `317`) and update them: `README`s, `BUILD.md`, `REVIEW.md`,
`CLAUDE.md`, `SPEC.md`, `docs/architecture.md`.

## 2. Templates

**Catalog entry** (C+ #6). `loc` is rewritten by `gen-loc` ([§3](#3-the-patch-and-its-order)); text is the printed text, with
`{key}` where a number is declared in `params`. A token has `"token": true`, `"rarity": "Token"` and the `Token` tag.

```json
"classicplus-006": {
  "id": "classicplus-006", "index": "6", "name": "Wrong-House Attacker", "set": "Classic+",
  "type": "Unit", "tags": ["Human"], "rarity": "Common", "token": false, "cost": 1, "loc": 3,
  "base":    { "attack": 1, "health": 1, "keywords": [{ "kind": "Rush" }, { "kind": "Lifesteal" }, { "kind": "Poisonous" }],
               "text": "Rush, Lifesteal, Poisonous" },
  "radiant": { "attack": 2, "health": 2, "keywords": [{ "kind": "Rush" }, { "kind": "Lifesteal" }, { "kind": "Poisonous" }, { "kind": "Reborn" }],
               "text": "Rush, Lifesteal, Poisonous, Reborn" }
}
```

**Script, keywords only** (`src/scripts/classic-plus/006-wrong-house-attacker.ts`, comments left out). Keywords are catalog data, so there is nothing to run:

```ts
import type { Script } from "@jackioh/engine";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-006");
export const base: Script = {};
export const radiant: Script = base;
```

**Script with an effect** (`005-guy-att.ts`). Hooks return `Effect[]` from `@jackioh/engine/effects` and never touch state:

```ts
import type { Script } from "@jackioh/engine";
import { destroyAll } from "@jackioh/engine/effects";
import { cardDef } from "../../catalog-data";

export const def = cardDef("classicplus-005");
export const base: Script = { cry: () => [destroyAll({ side: "self", rows: ["backrow"] })] };
export const radiant: Script = { cry: () => [destroyAll({ side: "any", rows: ["backrow"] })] };
```

Read a declared number with `param(ctx, "draw")`, never a literal. Declared targets and modes go in `targets` and `modes`
([cards README §1](../packages/cards/README.md)). A card with no script of its own for one face reuses `base`.

**Test** (`test/classic-plus/006-wrong-house-attacker.test.ts`, first case only). Build every game with `scenario()` from `../_harness`, and name a
test after the ruling it pins (`it("R64 …")`):

```ts
import { describe, expect, it } from "vitest";
import { scenario } from "../_harness";
import { base, def, radiant } from "../../src/scripts/classic-plus/006-wrong-house-attacker";

describe("C+ #6 Wrong-House Attacker", () => {
  it("is a (1) 1/1 Human Unit with Rush, Lifesteal, Poisonous (Radiant 2/2 plus Reborn); no script on either face", () => {
    expect([def.cost, def.base.attack, def.base.health, def.radiant.attack, def.radiant.health]).toEqual([1, 1, 1, 2, 2]);
    expect(def.base.keywords.map((k) => k.kind)).toEqual(["Rush", "Lifesteal", "Poisonous"]);
    expect(def.radiant.keywords.map((k) => k.kind)).toEqual(["Rush", "Lifesteal", "Poisonous", "Reborn"]);
    expect(base).toEqual({});
    expect(radiant).toBe(base);
  });
  // …one case per behaviour in the SPEC row, for base and radiant separately.
});
```

## 3. The patch, and its order

A change to card data is a patch (R388, R646). The designer picks the version name on the issue (`Patch v0.2.X: …`, or `Patch v0.2.Y: …`
for a micro patch, which keeps its `Y` until promotion names it, R650); never reopen a
shipped one. A branch claims its card changes with a pending fragment:

```
pnpm --filter @jackioh/cards run patches <version> <date> "<title>" --source "<issue or PR>" --notes "<what changed>"
```

It writes or updates `packages/cards/patches/pending/<version>.json`, containing the version, title,
sources, notes and every catalog id the patch changes. It never changes the shipped history, its
snapshots or `CATALOG_VERSION`. `pnpm --filter @jackioh/cards run patches check` must pass before the
branch merges. The post-merge promotion runs `patches ship`: it snapshots the catalog at the
fragment's first-parent commit, appends the shipped patch and bumps `CATALOG_VERSION` everywhere.

`typecheck` runs `gen`, whose `gen-loc` may update the card's `loc` in `catalog.json`; include that
card in the fragment before merging. It never amends a shipped snapshot. In a patch split over
several PRs, only the part that changes card data adds the fragment
([issues-and-patches.md](issues-and-patches.md)).

## 4. Gates, and what a new card breaks that is not your card's fault

Run, from the repo root: `pnpm typecheck` (generates, then compiles), `pnpm lint`, `pnpm validate:catalog`,
`pnpm --filter @jackioh/cards missing-tests`, `pnpm rulings:coverage`, then `pnpm test` (or one project:
`pnpm vitest run --project cards`; the cards project takes about ten minutes, almost all of it the two 1000-game fuzz suites).
Replay one fuzz seed with `JACKIOH_FUZZ_FROM=<seed> JACKIOH_FUZZ_SEEDS=1 pnpm fuzz`.

One more card shifts every random draw from the pool (R380), so tests and games that were lucky stop being so:

- **Seed-pinned tests** ("the first Discover offers Bigot, the second Twisted Sorcerer"): `play-choices`, `resolving-face`, `fused-hooks`,
  `tributes`, `pools-and-randomness`, `098-heroic-power` (Stitching), and `ai/test/shadowBan.test.ts`. Re-pin by looping seeds in a
  throwaway test (`for n … scenario({ seed: \`craft-${n}\`, … })`, `g.play(…)`, `g.answer(…)` in a `try`) until the offers match, and
  update the comment beside the seed.
- **The fuzz gates** play 1000 seeded games each and may now draw a game nobody has played. Four latent bugs came out this way, all fixed: a fused
  card of two ingredients that define `targetChecks` threw from `legalActions` (#104), a client table made the animation runner replay a
  whole event window (#106), `determinize` put a Siphon Squad in a hidden slot the seat's own view rules out (#107), and a deeply nested
  fusion resumed Final Gambit's step against the wrong ingredient (#105). If a gate fails at a seed that has nothing to do with
  your card, print the failing game's state, find which card the throw or the diff names, and file it rather than editing the test.

## 5. Do not read

These look relevant and do not change for a standard card: `packages/cards/src/index.ts` (a contract shared by every card, README §2),
`_generated.ts` (generated; commit it), `catalog-data.ts` (only the post-merge promotion edits the version), the snapshots in `packages/cards/patches/`,
`packages/engine/**` (unless the text needs a new verb, [§6](#6-a-card-the-engine-cannot-express-yet)), `apps/web/src/cards/**` (faces
and art are drawn from the catalog; procedural art needs nothing), `apps/server/src/**` and `packages/ai/src/**` (they read the catalog),
`e2e/fixtures/decks/*.json`, `reviews/`, `docs/polish/`, and the designer's source notes (`JackiOh_*.md`).

## 6. A card the engine cannot express yet

When the text needs a verb, keyword, filter or board fact that no card uses: find where the most similar existing mechanic lives, add the new
one beside it in that pattern, and test it there. A verb is an effect in `packages/engine/src/effects/` (re-exported by its barrel), a new
board fact goes in `packages/engine/src/query.ts`, a new keyword also needs its row in SPEC §6.1 (the web glossary reads that table). Keep the card's script a list of those effects
(CLAUDE.md rules 4 and 5). A rule the SPEC does not decide is a new R-row in SPEC §11 with a proving test (CLAUDE.md rule 3). No escalation step.

## 7. What cannot be done on Linux

Every card needs its voice lines **rendered**: `apps/web/src/audio/voice-assets.test.ts` (B35, B36, B37, R501) expects an `.m4a` per line in
`apps/web/public/audio/voice/` and a manifest entry. `pnpm --filter @jackioh/web gen:voice` renders them with macOS `say` and `afconvert` or
Windows SAPI and ffmpeg, and has no Linux backend, so a card added from Linux or CI fails about twenty tests there until a person renders them.
`node apps/web/scripts/gen-voice.mjs --check` lists what is missing. Write the `voice-lines.json` entry (the text needs no tool) and say in the
PR that the audio is owed.
