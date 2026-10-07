# Slice: part 23 (golden traces recorded from the TypeScript engine), chunk 1 of 1
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt` or `clippy` was run. This part's exception (brief Steps 2–4): the
TypeScript engine was run to record the oracle. `CYPRESS_INSTALL_BINARY=0 pnpm install --frozen-lockfile`
(the Cypress binary download was reset by the proxy; nothing in the repo changed), then
`pnpm exec tsx scripts/golden/record.ts > crates/engine/tests/golden/games.jsonl` twice (the second to a
scratch file) and `cmp`: byte-identical. Also `pnpm exec eslint scripts/golden/record.ts` (clean) and
`tsc` over it with `tsconfig.base.json` (clean), and `--seed 201 --dump-step 5` / `--dump-step begin`,
whose hashes equal the recorded line's.

## FILES
All four Rust/data destinations were empty placeholders or absent on `staging`; all are complete.
No `todo!`, `unimplemented!` or `// TODO`.
- `scripts/golden/record.ts` (new): SURFACE §13.1–§13.3. Verbatim copies of fuzz.test.ts's
  `POOL_EXCLUSIONS`, `EXCLUDED_IDS`, `FUZZ_POOL`, `decksForSeed` and fuzz-handicap.test.ts's `POOL`,
  `handicapForSeed`, `decksForSeed` (renamed `handicapDecksForSeed`), and of replay.ts's `canonical` and
  FNV loop. `--seed k` prints one line; `--seed k --dump-step n|begin [--out dir]` writes TS's side.
- `crates/engine/tests/golden/games.jsonl` (new): 240 games (seeds 1–200 and 201–240), 22,765 steps,
  4,087,776 bytes (no fallback needed). Endings: 239 hero-death, 1 both-heroes-dead; none hit the
  3,000-step cap; longest game 224 steps. No non-integer number anywhere in a hashed value.
- `crates/engine/tests/golden/01-hotseat-full-game.json` (copy): `cmp`-identical to
  `packages/cards/test/fixtures/01-hotseat-full-game.json`.
- `crates/engine/tests/golden.rs` (new): the trace replay (12 `#[test]` shards plus a file-shape test)
  and the whole port of `packages/cards/test/hotseat-replay.test.ts` (its header, `Recording`, `read`,
  `EXPECTED_HASH` "a798906b" with its full doc comment, `EXPECTED_RESULT`, `EXPECTED_ACTIONS`, and all
  six `it`s in one `mod` named after its `describe`).
- `crates/tools/src/golden.rs` (new): `golden check [--file PATH] [--seed K]…` and
  `golden bless [--seeds N] [--file PATH]`.

## SURFACE
- `golden::Args` (`#[derive(clap::Args)]`, a private `#[command(subcommand)]` enum `Check | Bless`) and
  `pub fn run(args: Args) -> anyhow::Result<()>`, as part 1's `main.rs` calls them. `check` exits 1 (Err)
  on any divergence or a hotseat fold that is not "a798906b".
- Diff files: Rust writes `target/golden-diff/<seed>-<step>-<which>.json`, TS writes
  `target/golden-diff/<seed>-<step>-<which>.ts.json`; `<seed>` is the game seed string
  (`jackioh-fuzz-17`, `jackioh-fuzz-handicap-201`), `<step>` the 0-based index into the line's `steps`
  or `begin`, `<which>` one of `s` (canonical state minus `applied`/`opening`), `v-p1`, `v-p2`, `e`, `l`
  (one canonical action per line); TS adds `a` (the action), Rust `refused` (the state before an action
  `reduce` refused; compare it with TS's `s` of the previous step). Each file is the canonical text and
  a newline. `target/` is the workspace root's (`CARGO_MANIFEST_DIR/../../target`, and
  `scripts/golden/../../target` for TS).
- `record.ts --seed k` takes the trailing number of the seed string (1–200, 201–240).

## DEPENDS-ON
- `jackioh_engine::{create_game(&CreateGameArgs), begin_game(&GameState), reduce(&GameState, &Action)
  -> ReduceResult { state, events, error }, legal_actions(&GameState, PlayerId) -> Vec<ActionBody>,
  view_for(&GameState, PlayerId) -> PlayerView, hash_state(&GameState) -> String, fold(&FoldArgs) ->
  FoldResult, mulligan_owed(&GameState) -> Vec<PlayerId>, seat_to_act(&GameState) -> Option<PlayerId>}`
  through the root re-exports (SURFACE §6.1).
- `jackioh_engine::replay::{canonical(&Value) -> String, fnv1a32_utf16(&str) -> String}` (part 5, §5.2).
- `FoldArgs: Deserialize` (built from `{seed, decks, log}` JSON) and `FoldResult { state, errors }` whose
  items have `nonce` and `error` fields (part 5.1's `FoldError`).
- `jackioh_engine::subsystems::choose_action(&GameState, PlayerId, &mut Rng) -> Option<ActionBody>` (part 8).
- Frozen (part 1): `CreateGameArgs` (Deserialize, Default), `Action::new`, `Action.player_id`,
  `Action::action_type()` (Display), `GameResult { winner, reason }` (Copy, PartialEq), `Phase::Over`,
  `PerPlayerOpt::{default, slot}`, `Handicap.deck_size`, `AI_DIFFICULTY[Difficulty]`, `DECK_SIZE`,
  `Tag::Token`, `CardDef { token: bool, tags }`, `rng::Rng::{new, coin, shuffle}`,
  `jackioh_cards::{CATALOG, register_all}`.

## GAPS
- Nothing left unported. Every name above that is not part 1's is called as SURFACE and parts 5.1/5.2's
  notes give it; part 31 resolves any miss.
- SURFACE §13.1 leaves open which strings seeds 201–240 use (decided below). §13.3 says "on the first
  mismatch, fail" without saying whether the whole run or each game stops there (decided below).
- Run time in Wave 3: the replay is 22,765 steps, each `hash_state` + two `view_for` + `legal_actions` +
  four canonical texts. Under the dev profile that may be slow; the 12 shards run in parallel, and
  part 32/30 may want `[profile.test] opt-level = 1` (or more) for the engine, which this part cannot
  set (the manifests are part 1's).
- `crates/engine/tests/golden.rs` is `include_str!`ing a 4 MB file; fine for rustc, noted in case
  compile time matters.

## Decisions
- Seeds 201–240 are fuzz-handicap.test.ts's own seeds 201–240, played exactly as its `playSeed(k)`:
  game seed `jackioh-fuzz-handicap-<k>`, decks `jackioh-fuzz-handicap-decks-<k>`, policy
  `jackioh-fuzz-handicap-policy-<k>`, order `jackioh-fuzz-handicap-policy-order-<k>`, handicap
  `handicapForSeed(k)` (so 201 is Hard on p1). Each golden game is therefore the same game as that fuzz
  file's seed, nonces aside.
- Nonce `golden-<n>`, n the actions applied before it (the fuzz's `fuzz-<n>` renamed, §13.1).
- STEP_CAP 3000: a capped game would end `{"winner":null,"reason":null,"steps":3000}` (none is).
- `l` is a set: TS `[...new Set(texts)].sort()` (UTF-16 order), Rust sort by `encode_utf16` then
  `dedup`. Duplicates in either engine's list never move the hash.
- `args` is `{seed, decks, handicaps}` with `"handicaps": null` for a plain game (§13.3's example);
  Rust reads it as `CreateGameArgs` (null → `None`).
- The recorder refuses a non-integer number (not `Number.isSafeInteger`) anywhere in the state minus
  `applied`/`opening`, both views, the events and the legal actions, naming its JSON path (brief Risks).
  None occurred. It also checks its copied canonical/FNV against `hashState` at each game's begin and end.
- `s` is `hashState(state)` itself; `v`, `e`, `l` use the copied `canonical` and FNV (§13.2).
- The Rust test replays every game and reports each divergent game's first mismatch (writing its
  diff), rather than stopping the whole run at the first one: divergences cluster, and the list shows
  the earliest seed to fix first. `cargo jackioh golden check` does the same, in parallel with rayon.
- Twelve `#[test]` shards (a macro; `GOLDEN_SHARDS` is the macro's count) instead of threads: the
  engine's `clippy.toml` bans `std::thread::spawn` in its tests too.
- The engine's `clippy.toml` I/O bans: data comes in by `include_str!`; `#[allow(clippy::disallowed_methods)]`
  only on `write_diff` (`std::fs::write`) and on the Cypress-recording comparison (`std::fs::read_to_string`
  of `e2e/artifacts/01-hotseat-full-game.json`, which TS's test read the same way).
- Hotseat port: TS's "every action carries a nonce / names its seat" is checked on the raw JSON, since a
  typed `Action` cannot lack either. `COMMITTED`/`RECORDED` became an `include_str!` and a path
  constant. `EXPECTED_HASH`'s doc comment is TS's, with one paragraph added for v0.3.0.
- `bless --seeds N` (1–200) records plain seeds 1..=N and handicapped 201..=200+N/5 (`PLAIN_PER_HANDICAPPED`),
  so 200 gives §13.1's 240 games and 150 the brief's fallback (1–150, 201–230). It writes lines through
  `#[derive(Serialize)]` structs in §13.3's key order (serde_json without `preserve_order` would sort a
  `json!` object's keys). A blessed line's `a` follows Rust's `ActionBody` field order and its handicap
  `Handicap`'s, which may differ from TS's literal order; nothing reads the line's own text.
- `check` also folds the hotseat fixture (skipped under a `--seed` filter); `--file` points either
  command at another file.
- record.ts imports `../../packages/{shared,engine,cards}/src/index` by relative path; the engine's package
  export is `src/index.ts`, so the cards package's `@jackioh/engine` is the same module instance and
  `registerAll()` registers into the engine the recorder calls.
