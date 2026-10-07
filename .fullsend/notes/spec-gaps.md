# Spec gaps found in Wave 2 (part 31)

## engine tests

- **Testkit seams for `vi.mock` (part 26.6, `crates/engine/tests/rules/turn_wiring.rs`).** The file
  imports `jackioh_engine::testkit::{mock_brittle_tick, mock_animate_at_turn_start,
  mock_return_at_cleanup}`, each taking `impl Fn(&mut EngineSink<'_>, PlayerId)` and standing in for
  `brittle::brittle_tick`, `animated::animate_at_turn_start` and the end-of-turn cleanup return, the
  way TS's `vi.mock` of those modules did. The engine has none of them; SURFACE §8 names only the
  thread-local registry override. Decision taken here: the tests are kept as written (not deleted);
  the engine-src reconciler (or the orchestrator) decides whether these three thread-local hooks
  exist, `#[cfg(feature = "testkit")]` beside the registry override. Until then: 1 E0432 in
  turn_wiring.rs, and the `rules` binary does not build while it stands (its 9 tests).
- **A test-made work handler (part 25.3, `crates/engine/tests/rules/pauses.rs`).** Two TS tests
  ("§9.3 resumes a three-step sequence at the step after the one that asked", "§9.3 finishes a pause
  nested inside an owed sequence before the sequence's own tail") drain work owed under a hook name the
  test registers with `registerWorkHandler`; SURFACE §6.6 replaces registration with `work.rs`'s
  closed `match`, so a test cannot add one. Part 25.3 left an empty module in their place. Decision
  taken here: nothing deleted or added; the engine-src reconciler decides whether the testkit gets a
  thread-local `register_work_handler(hook, fn(&mut EngineSink, &WorkItem))` consulted by the
  dispatcher's fallthrough under `#[cfg(feature = "testkit")]` (the same exception SURFACE §8 makes
  for the registries); if it does, the two tests are ported back from `pauses.test.ts`.
- **The testkit exports no `Arc`** (SURFACE §8 lists `json!`, `Value`, `serde_json`, `IndexMap`,
  `IndexSet`, `json_as`), while effect argument structs that hold functions (`ForEachCardArgs`,
  `ChooseTargetWhereArgs`, `DrawWhileArgs`, `CastEachArgs`) need `Arc::new` at the call. Decision:
  four test files import `std::sync::Arc` themselves; no SURFACE change needed unless the orchestrator
  prefers the testkit to re-export it as the prelude does.

## ai (part 31)

- **SURFACE §4.3 / §9: the AI's node counts are not classified.** §4.3 sorts TS `number` into game
  quantities (`i32`), indexes (`usize`), cursors (`u32`) and fractions (`f64`), and part 1 added
  "bounds on Rust loops and collection lengths `usize`"; a search budget fits none of them cleanly,
  and part 17's chunks split on it (17.1 `i32` "every AI count", 17.2 `usize`). Decision taken:
  `SearchBudget`'s fields, `SearchStats.{nodes, determinizations, lines, sim_errors}`,
  `NodeCounter::{used, limit}`, `find_lethal`'s `limit` and `MatchRecord.nodes` are `usize`
  (`crates/ai/src/types.rs`); the AI's config counts (`AI_SEARCH`, `AI_REPLY`, `AI_MULLIGAN`) stay
  `i32` and are cast where they meet a node count. JSON is unchanged (both serialise as numbers),
  so `ai_decide`'s answer and `constants()`'s `AI_BUDGET` read the same in the web. Suggested line
  for §4.3: "`number` (an AI node budget or count) → `usize`".
- **SURFACE §9: `NodeCounter`'s receiver.** §9 names no counter; the decision (`&dyn
  NodeCounter`, `&self` methods, tallies in `Cell`) is `types.rs`'s. `Cell` is allowed by §3's
  `clippy.toml` (only `RefCell` is banned); if §3 means "no interior mutability" generally, the
  counter is the one exception and should be named there.
