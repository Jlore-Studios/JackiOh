# Slice: part 21, chunk 2 (#409): the Done-when grep, emptied
BUILDS-RUN: 0 (no cargo, rustc, pnpm, tsc, vitest or Cypress. grep and Python only: Python to make
the edits and to simulate the messages test's new extraction over the real files.)

`grep -rnE "packages/|apps/server/src" apps/web/src e2e --include=*.ts --include=*.tsx` printed 77
lines in 44 files; it prints nothing now. The same 44 files changed, nothing else (plus this note).

## FILES
The three tests that read TypeScript source:
- `apps/web/src/game/v020-real.test.tsx`: no `packages/cards/test/_harness`. The five cases (C #23
  Devil's Pact, C #21 Turtinator, C #18 Glitch in the System, C #44 Back from the GY, C #5 Tesla) are
  kept with their bodies and assertions unchanged; a local `scenario()` builds the same boards
  through the wasm wrapper (`../wasm/index.ts`: `createGame`, `reduce`, `legalActions`, `viewFor`,
  `registeredCatalog`). No case dropped.
- `apps/web/src/net/env-production.test.ts`: reads `crates/server/src/env.rs` (its `PUBLIC_ENV_VARS`
  spells each name as `"VITE_…"`, so the `toContain(`"${key}"`)` assertion is unchanged).
- `apps/web/src/game/deckbuilder/messages.test.ts`: reads `crates/engine/src/validator.rs`.
The other 41 files: comments and one message string (`game/decks.ts`'s empty-catalog error) reworded
to name `crates/engine`, `crates/cards`, `crates/server`, the wire layer or the wasm module. Every
R-id and §-id kept.

## Decisions
- v020's board builder is a port of the harness's `buildState` for the setup these five cases use
  (hand, field/backrow with `lane` and `faceUp`, library, graveyard, `mana`, `health`, `active`):
  the same seed (`jackioh-harness`), turn 9 in the main phase, turnsStarted 5/4, filler decks (the
  first DECK_SIZE non-token Core cards by §5 index, as `query({})` gave), libraries emptied, `nextId`
  back to 1, cards laid out p1 then p2 in the harness's order so ids follow the literal, lanes per
  row with `field` first, units in ATK, library cards with `knownAs` on the base face (R311), mana
  refreshed as `refreshMana` does. The state is plain JSON (SURFACE §5.1), so the layout writes it;
  each new card is a JSON copy of a card `createGame` dealt with its id, def, owner, controller and
  zone replaced (and `knownAs` removed), so it carries every field the Rust engine gives a card.
  Plays go through `reduce` with nonces `h1`, `h2`, … as the harness's did.
- Left out of that port, because none of the five boards reaches them: the harness's closing
  `stateCheck` (no unit at lethal damage, no Twinspell echo grant, no carrier, no quest, no
  `tributeWhen`, no hero at 0; and every `reduce` runs the state check anyway), `startBrittleOnField`
  (no card here prints Brittle), Stack piles, `damage`/`counters`/`statsOverride`/cost seeds, exile,
  `armor`, `turn`, `seed`, last boards, and the harness's `attack`/`answer`/`endTurn`/`activate`
  steps. The builder takes only the setup keys it implements, so a future case that needs one fails
  to type-check rather than silently ignoring it.
- messages.test.ts, the extraction: Rust's messages are `format!` strings and plain literals, so it
  lexes the source in one regex pass (comments, char literals, string literals; only string bodies
  kept, `\"`/`\\`/`\'` unescaped) and splits on `{…}` holes instead of backtick templates and `${…}`.
  One change to the filter: the punctuation that joins a piece to a hole (`;:,."()`) is not counted
  towards the 9-character minimum, though it stays in the fragment. A Rust message is one literal
  where TS's were joined templates, so `"{} has … of {}; at most {MAX_COPIES} …"` yields `; at most`,
  which `game/decks.ts:120` and `wire/codes.ts:45` contain without being validator sentences;
  stripping the punctuation from the fragment itself instead would make `this one has`, which two
  client comments contain. Simulated in Python on the real files: 32 fragments (TS's extraction found
  28), no hit in `apps/web/src`, and the control's first fragment (`" is not a card a deck can
  hold.`) is in the unescaped source. One fragment is not a sentence a player sees (`at least two
  labels`, an `expect` message); it is harmless to scan for.
- Rust names in the reworded comments follow SURFACE §4.2 (TS names snake_cased: `target_id`,
  `validate_deck`, `check_deck_draft`, `push_view`) and the paths follow `docs/v0.3.0/port-map.tsv`;
  they were not checked against the Rust (rule 2), except `env.rs`'s `PUBLIC_ENV_VARS` and
  `validator.rs`'s literals.
- A few comments next to a reworded line were stale in the same breath and were corrected with it:
  `routes/match.tsx` (net.ts no longer takes a standalone `legal` frame), `routes/dev/hotseat.tsx`
  (why `loadEnginePort()` rejects now), `cards/quest.test.tsx` (`game/engine.real.ts` is gone; the
  real-engine block runs through `audio/test/realGame.ts`), `e2e/cypress/e2e/11-radiant.cy.ts` (the
  wasm `init` registers the catalog, not `registerAll()`), `game/deckbuilder/deckSize.ts` and
  `game/decks.ts` (what `@jackioh/engine/config` is now).

## GAPS
- Nothing here has run. v020-real.test.tsx assumes the Rust engine deserialises a `GameState` laid
  out in TS's JSON shape (SURFACE §5.1) mid-game (phase `main`, no mulligan, hand-placed piles) and
  that the five boards behave as they did on the TS engine; part 36 runs it.
- `scenario()` lives in the test file, not a shared web test helper: no other web test builds a
  hand-laid board today. If a second one needs it, move it to `src/test/`.
