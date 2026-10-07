# Slice: part 22 (the jackioh CLI), chunk 3 of 3: `cargo jackioh fuzz` and the cards data tests
BUILDS-RUN: 0

## FILES
All new, each the whole TS file (every `it`, every function, the comments that state a rule or cite
a ruling):
- `crates/tools/src/fuzz.rs` ← `packages/cards/test/fuzz.test.ts` + `fuzz-handicap.test.ts`:
  `Args { from, seeds, handicap }`, `run`, the gate and the handicapped wave, rayon over seeds,
  `#[cfg(test)]` seeds 1–20 of each.
- `crates/cards/tests/cross/{card_text, catalog, flavour, params, pools_and_randomness, query,
  radiant_standard, references, registry}.rs` ← `packages/cards/test/<x>.test.ts`.
Nothing left.

## SURFACE
Matched §4.1 paths, §4.2 names, §12's `fuzz [--from N] [--seeds N] [--handicap]`. Each TS
`describe` is a `mod`, each `it` a `#[test]` named by §7.3 (ruling ids lead: `r660_…`; a title that
opens with "§5.1" leads with `s5_1_`).

## GAPS (names I call that other parts provide; the signature I assumed)
Testkit (part 5, `jackioh_engine::testkit`):
- `scenario(Value) -> Scenario`; `Scenario::{state() -> &GameState, state_mut() -> &mut GameState
  (part-05-2's gap too), play(&str, Value), answer(Value), hand(PlayerId) -> Vec<CardInstance>
  (or of refs), unit(PlayerId, i32) / backrow(PlayerId, i32) -> Option<CardInstance or &CardInstance>,
  last_events() -> &[GameEvent]}`. A card ref is always passed as `&str` (an instance id for an
  instance), which TS's resolver reads too.
- `register_catalog(CardDefs)` and `register_scripts(IndexMap<String, CardScripts>)` (the
  thread-local override, one argument, SURFACE §8); `json_as`.
- `create_invariant_monitor(&GameState)` with `before(&mut, &GameState, PlayerId, &ActionBody)`,
  `after(&mut, &[GameEvent], &GameState)`, `hidden(&, &GameState)`, each `-> Vec<String>`;
  `I6_GATE_STRIDE: usize` (as part-05-4's notes give them).
Cards (part 9, `crates/cards/src/query.rs`):
- `jackioh_cards::query::{CardQuery: DeserializeOwned (TS's object literal as JSON), query(&CardQuery)
  -> Vec<&'static CardDef>, pool(&str, &CardQuery) -> Vec<&'static CardDef>, TRAP_TYPES: [CardType]
  slice or array (Serialize), query_cost(&CardDef) -> i32 (TS re-exported it)}` and the card-facing
  surface as a module: `pub mod catalog { query, pool, cost (= query_cost), TRAP_TYPES }` (TS's
  `catalog = { query, pool, cost, trapTypes }`). If part 9 shapes `catalog` otherwise, only
  `query.rs`'s `catalog::…` calls move.
Engine:
- `catalog::{registered_catalog() -> &'static CardDefs, catalog_version()}` (part 2);
  `scripts::scripts_for(&str) -> CardScripts` (TS `scriptsFor`, part 2 chunk 1).
- `subsystems::roll_chaos_effects(&mut Rng, bool)` with two arguments (TS's `table` default) whose
  items have `.name` (`&str` or `String`); `subsystems::score_def(&GameState, PlayerId, &CardDef)` with
  three (TS's `options`/`base` defaults) returning `.priority: subsystems::ScorePriority` with
  `Lethal`; `subsystems::choose_action(&GameState, PlayerId, &mut Rng) -> Option<ActionBody>` (part 8).
  If part 8 kept the TS defaults as trailing parameters, part 31 adds `None`/`&Default::default()`.
- `fold(&FoldArgs { seed, decks, log, catalog, handicaps, dealt, last_boards, glitch_boards })` (TS
  `ReplayInput`, every field written out) `-> { state, errors: [{ nonce, error }] }`; `hash_state`;
  `reduce(&GameState, &Action) -> ReduceResult`; `begin_game(&GameState)`; `mulligan_owed(&GameState)
  -> Vec<PlayerId>`; `seat_to_act(&GameState) -> Option<PlayerId>` (SURFACE §6.1; TS returns a seat
  always, so fuzz reads `None` as `state.active`, TS's own last fallback).
Not ported, with the reason:
- `card-text.test.ts`'s `readFragments()`: the pending fragments live in a directory, and a pure
  crate's tests read no files (SURFACE §3). The one test that read it (v0.2.7) does so only while
  v0.2.7 is pending; it has shipped, so the Rust test asserts that it has (and says why if it ever
  is not). The pending claims are `cargo jackioh patches check`'s to prove (chunk owning patches.rs).
- `registry.test.ts`'s `buildRegistry` error paths ("two card scripts claim", "not in"): build.rs is
  that check now (SURFACE §7.4). The tests prove the properties it guarantees over `REGISTRY`.
- `naming.ts` is the tools' (patches.rs, another chunk); registry.rs carries a private copy of the
  functions its tests need (`mod naming`, builder rule 5).

## Decisions
- fuzz: `--handicap` runs fuzz-handicap's wave instead of the gate (TS's `pnpm fuzz` ran both files;
  CI calls the command twice, part 30). `--from`/`--seeds` keep TS's env rules (a value under 1 is
  ignored; `--seeds` never widens past `WAVE_SEEDS`). The test build is TS's "whole-suite sweep"
  (`cfg!(test)`): `SWEEP_SEEDS` is 20 there (brief) and I6 runs on every state; the binary runs 1,000
  with `I6_GATE_STRIDE`. Seeds run on rayon and are read back in seed order. A panic is caught per seed
  (`catch_unwind`), and a once-installed panic hook (chained to the previous one) records its
  `file:line:col`, Rust's stand-in for TS's `originOf` stack frame. `signature_of`'s five regexes are
  hand-written char scanners with JS's `\b`. The report's reproduce line names
  `cargo jackioh fuzz --from <seed> --seeds 1`. fuzz-handicap's `decksForSeed` is
  `handicap_decks_for_seed` beside the gate's. Exit 1 (an `Err`) when any seed failed.
- No regex crate in a pure crate: every regex of card-text, params, references and catalog is a
  small matcher named after it (byte scans, ASCII `\b`, `/i` as ASCII lowering), and each file that
  has them adds one extra test pinning the matchers' near misses (not in TS).
- Patch snapshots and `patches.json` are `include_str!`ed into card_text.rs (22 files, ~4 MB in the
  test binary); a newer snapshot needs a row in its `SNAPSHOTS`. Snapshots parse as
  `IndexMap<String, IndexMap<String, Value>>` so top-level field order is the file's (v0.2.2's
  changed-field lists depend on it); `JSON.stringify` equality is `serde_json::to_string` equality
  (top level in file order, nested objects canonical), `toEqual` is `==`.
- `catalog.test.ts`'s one `it` per fixture row is one test per set that reports every failing row.
  The fixture tables were converted from the TS literals by a script (318 rows, the Glitch comment
  kept).
- `radiant_standard.rs` reads `docs/radiant-audit.md` with `include_str!` (index and name only).
- `flavour.rs` keeps `FLAVOUR_MAX_CHARS` (120) and `ARTIST_MAX_CHARS` (60) as private constants: their
  TS home, `packages/cards/src/flavour.ts`, moves to the web, and no Rust config holds them. Lengths are
  UTF-16 code units, as JS counted.
- `query.rs` registers the catalog per test through the testkit override (TS's `beforeAll`); the
  reversed-registry tests restore it before asserting (TS's `finally`).
- `registry.rs`: script identity (`toBe`) becomes a data fingerprint of each `Script` (hook presence,
  declarations, trigger/resume/check names), since scripts hold closures and each `script()` call
  builds new ones; the fixture-registration test runs its fixture on a scoped thread, since the
  testkit override is per thread and production registries never change.
