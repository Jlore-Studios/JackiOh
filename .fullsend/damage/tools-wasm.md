# Damage: `crates/tools/**` and `crates/wasm/**` (part 31, tools and wasm reconciler)

## How it was measured

The real build cannot reach either crate: `cargo check -p jackioh-tools -p jackioh-wasm
--all-targets --keep-going` stops in `jackioh-engine (lib)` (188 errors at `9edbee2`/`98584a0`, the
first build; 45 at `1d56a25`, after the engine reconcilers' first commits). So the counts below are
from a **shadow build**, the ai reconciler's method (`.fullsend/damage/ai.md`): a scratch copy of the
workspace, never committed, in which every function body of `jackioh-engine`, `jackioh-cards` and
`jackioh-ai` is `loop {}` (signatures, types, consts and the testkit kept; card scripts left out),
with `crates/tools` and `crates/wasm` copied whole. That type-checks (and borrow-checks) both crates,
bins, libs and their tests, against the owners' **signatures as they stand on `staging`**.
`rust-analyzer diagnostics` over the real tree agreed (same errors, no others in scope).

## Before (first build: `crates/tools`, `crates/wasm` as Wave 1 left them at `98584a0`)

Real build: 0 errors reachable in scope; `jackioh-engine (lib)` 188 errors (E0308 100, E0609 26,
E0061 21, E0277 12, E0432 9, E0560 6, E0599 5, E0063 3, E0422 2, E0631 1, E0574 1, E0271 1).

Shadow build: **3** errors, 1 warning.

| File | Errors |
|---|---|
| `crates/tools/src/sweep.rs` | 2 |
| `crates/tools/src/gate.rs` | 1 |
| `crates/wasm/src/lib.rs` | 0 |

By code: E0308 3 (`sweep_card`/`sweep_at_risk` take `&SweepOptions`; `MatchConfig.max_actions` is
`Option<usize>`, the gate passed an `i32`). Type errors stop borrow checking, so this was a floor.

## After (this part's commits on `staging`)

Shadow build: **0** errors, **0** warnings in scope, `--all-targets` (the `jackioh` bin, its unit
tests, the wasm `rlib`/`cdylib`), on the host and on `wasm32-unknown-unknown`; borrowck clean.
`cargo clippy` (no `-D warnings`): 37 lint warnings, all local (Wave 3): fuzz.rs 18, gate.rs 11,
patches.rs 3, sweep.rs 2, golden.rs 2, stats.rs 1 (unnecessary casts 12, doc-list indentation 13,
`is_multiple_of` 3, large `Err` 2, `sort_by_key` 2, other 5).

What is left for Wave 3 in scope is therefore not type errors but whatever the engine's real bodies
do once they compile: the golden traces, `fuzz`, the gate and the arena tests are behaviour checks.

## Collisions (one concept, every location)

| Concept | Locations | Winner |
|---|---|---|
| The fuzz deal (pool, `decksForSeed`, `handicapForSeed`, handicapped decks, R265 actor order) | `tools/src/fuzz.rs`; `tools/src/golden.rs` (copies: `POOL_EXCLUSIONS`, `deck_legal_ids`, `fuzz_pool`, `handicap_pool`, `decks_for_seed`, `handicap_for_seed`, `handicap_decks_for_seed`, the inline actor choice) | fuzz.rs |
| The gate run's report | `ai/src/gate.rs` `GateReport`; `tools/src/gate.rs` `GateRun` | ai's `GateReport` |
| The matchup list | `ai/src/gate.rs` `Matchup::ALL`/`as_str`; `tools/src/gate.rs` `matchups()` over `MATCHUP_NAMES`, `matchup_name` via serde | ai's (`MATCHUP_NAMES` stays: a test reads it) |
| Unix days → `YYYY-MM-DD` | `tools/src/patches.rs` `utc_date_of`; `arena.rs` `civil_date`; `sweep.rs` inside `utc_date_today` (+4 constants) | patches.rs |
| Today's UTC date | `arena.rs` `utc_date`; `sweep.rs` `utc_date_today` | arena.rs |
| `git` runner | `patches.rs` `git`; `promote.rs` `git` | patches.rs |
| Repository root | `patches.rs` `repo_root` (TS `REPO_ROOT`); `promote.rs` `repo_root` (git toplevel of the cwd); `spec.rs` `default_root` (cwd ancestor search) | patches.rs |
| `is_ident_char` | `spec.rs`; `catalog.rs` | spec.rs |
| Panic payload → message | `agent.rs` `panic_message`; `fuzz.rs` `panic_message` | agent.rs |
| `MS_PER_SECOND` | `gate.rs`; `sweep.rs`; bare `1000.0` in `arena.rs`, `fuzz.rs` (×3) | gate.rs |
| Sorted shadow-ban ids | `ai/src/shadow_ban.rs` `SHADOW_BAN_IDS`; `tools/src/agent.rs` `own_shadow_ban`; `wasm` `constants()` sort | ai's |
| Refused-action record | `ai/src/match_.rs` `RejectedAction`; `arena.rs` `Rejected` | ai's |
| The random policy's skipped actions | engine `subsystems::AI_SKIPPED_ACTIONS`; `arena.rs` `SKIPPED_ACTIONS` copy | engine's |
| Newest patch in `patches.json` | `catalog.rs` `newest_version`; `stats.rs` `newest_patch` | not a collision: TS had both (catalog-version.mjs, stats.ts) |
| The referee loop (`playMatch`) | `ai/src/match_.rs` `play_match`; `arena.rs` `play_game` (+ `replacements_for`, `ARENA_MAX_ACTIONS`) | unresolved (decisions) |
| The golden check | `tools/src/golden.rs`; `engine/tests/golden.rs` | both mandated by SURFACE §13 |
| `naming.ts` | `tools/src/patches.rs` `naming`; `cards/tests/cross/registry.rs` private copy | out of scope; part 37 |

## Seams (calls to names that do not exist, or exist with another shape)

- `sweep.rs` → `jackioh_ai::{sweep_card, sweep_at_risk}`: options by value, owner takes `&SweepOptions`. Fixed.
- `gate.rs` → `MatchConfig.max_actions: Option<usize>`: `GREEDY_PROBE_ACTIONS` was `i32`. Fixed (now `usize`).
- `wasm` `validator("validateTrio")` → called `validate_loadout`; the owner exports `validate_trio`. Fixed.
- `wasm` `checkDeckDraft` → `DeckDraftInput.name_max_length` read through a `Value` hedge; the owner's type is `usize`. Fixed.
- Checked and matching the owners, no change: `FoldArgs` (camelCase serde, unknown keys ignored, `fold`/`hash_state`), `AiOptions`, `Decision` serde, `SearchBudget`, `AiDeckOptions` (`Default`, `banned: Option<Vec<String>>`, `mana_cap: Option<i32>`, `theme` three-state), `SweepResult { #[serde(flatten)] stats, tier, flags, unswept }`, `SweepOptions<'a>`, `MatchHooks<'a>` (boxed `after_action`/`time_decision`), `evaluate(state, seat, NextSwing, &EvalWeights)`, `AI_GATE` fields indexed by `Matchup` (`ByMatchup`), `game_config`/`run_gate_games`/`gate_shard_games`, `dev_game_record`/`DevRunOptions`, `card_stats`/`format_card_stats`, the testkit's `scenario`/`create_invariant_monitor`/`I6_GATE_STRIDE`, `subsystems::choose_action`, the validator's input/result types, `CHAOS_EFFECTS`/`CHAOS_PLUS_EFFECTS`/`HERO_POWERS` fields, `jackioh_cards::{register_all, catalog_version, CATALOG}`. `main.rs`'s dispatch matches every module's `Args`/`run` (and `catalog::{VersionArgs, run_version}`).

## Drift (deps, features, rules)

- No dependency or feature change needed: tools uses only its `Cargo.toml` rows; wasm likewise.
- Part 37's cull, noted not done: `patches::js::Json` (an order-keeping JSON value) would be
  `serde_json`'s `preserve_order`, which SURFACE §2 rules out for every crate; the hand-written SHA-1
  (`patches_io::git_blob_hash`) would be the `sha1` crate, which is not on §2's list. Both need a
  SURFACE §2 change first. `tools::sweep::to_fixed` and `ai::sweep`'s private `to_fixed_1` are the
  same JS `toFixed` (cross-crate; the ai copy is private).
- Test-side duplicates left as they are (tests are not edited): `replay.rs`'s `HOTSEAT`/`HOTSEAT_HASH`
  (= golden.rs's), `gate.rs`'s `MATCHUP_NAMES` (a test indexes it), the temp-directory helpers
  (`patches.rs`'s `TempDir`, ad-hoc dirs in gate/promote tests), promote's test `git_in` beside
  patches' test `git`.
- Rule 9: the bare `1000.0` milliseconds in arena.rs and fuzz.rs now name `MS_PER_SECOND`.

## Semantic conflicts (assumption keys in scope: parts 21, 22-1..3, 23-1, 28, 29-1)

| Key | Values | Settled |
|---|---|---|
| `error.style` | SURFACE default; part 21 `JsError` across WASM; part 22-2 `anyhow` in tools | No conflict: SURFACE §12 (`run -> anyhow::Result`, exit 1), §4.4.9 for engine refusals, §10.1's JS return types (part 21's `Result<String, JsError>`, see spec-gaps) |
| `paths.root` (22-2) / `spec.root` (28) | `CARGO_MANIFEST_DIR/../..`; cwd ancestor with `spec/`+`Cargo.toml`; (promote, no key) git toplevel of the cwd | SURFACE silent; the TS decides: `REPO_ROOT` and rulings-coverage.ts's `here` were source-relative, so `patches::repo_root` for every command (`spec --root` still overrides) |
| `tools.errors` (22-1) / `tools.error` (29-1) | both `anyhow`, a failed check is `Err` (exit 1) | Same value, two key names |
| `tools.parallel` | 22-1: rayon, one gate game per task; 29-1: rayon `par_iter`, game order | Compatible |
| `registry.style`, `rng.source`, `log.style` | part 21's and 22-2's refinements of the default | Compatible (WASM `init()` calls `register_all`; tools print to stdout, no tracing, SURFACE §12) |
