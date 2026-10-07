# Spec gaps: part 26, chunk 6 of 7
TESTS-IN: 86 (targeting 23, temporary 5, transform_variants 7, tribute_zones 7, tribute 12, turn_cap 2, turn_wiring 8, turn 16, view_marks 6)

## SPEC GAPS
Every `it` of the nine TS files has a Rust `#[test]` (one `mod` per `describe`, TS order). No file reads
source text, so nothing was dropped under #133's rule. What could not be written exactly as TS wrote it:

- `packages/engine/test/turn-wiring.test.ts:46-55` (`vi.mock` of `brittleTick`, `animateAtTurnStart`,
  `returnAtCleanup`), used by all 8 tests (`:90-98`, `:167`, `:172`, `:209`, `:242`, `:266`, `:294`, `:299`,
  `:317`, `:320`). Rust has no module mocking and SURFACE.md names no seam for it. The file is ported whole
  against a seam it asks for, in the spirit of SURFACE §8's thread-local registries:
  `jackioh_engine::testkit::{mock_brittle_tick, mock_animate_at_turn_start, mock_return_at_cleanup}`, each
  taking `impl Fn(&mut EngineSink<'_>, PlayerId) + Send + Sync + 'static` and setting a per-thread double
  that `brittle::brittle_tick`, `animated::animate_at_turn_start` and `animated::return_at_cleanup` run in
  place of their bodies under `#[cfg(feature = "testkit")]` (set again replaces it; nothing else in the two
  modules changes). `vi.fn`'s `mock.calls` is a channel the recording doubles send each call's player
  down. Part 31: add the seam (in `testkit/scenario.rs`, which `testkit::*` globs) or delete
  `crates/engine/tests/rules/turn_wiring.rs` and its `pub mod` line; every assertion is otherwise TS's.
- `packages/engine/test/transform-variants.test.ts:137`: `expect(golem.zone).toEqual({ z: "gone", player: "p1" })`
  reads the TS object after the card was Replaced and ceased to exist (R35). A Rust test holds a copy, never
  the live object, so it is ported as `find_instance(&state, &golem.id).is_none()` (the same reading
  part 25.1 took for `callToChaosPlus.test.ts:275`), beside the test's other assertions.
- `packages/engine/test/turn.test.ts:116`: `expect(zero.state).toBe(state)` is object identity; ported as
  `assert_eq!(zero.state, state)` (SURFACE §6.1: a refused action hands back the input, unchanged).
- `packages/engine/test/view-marks.test.ts:67-75` (`beforeAll`/`afterAll` saving and restoring the
  registries): no assertion; each Rust test runs on its own thread with its own testkit override, so there
  is nothing to restore. Not ported (the file header says why).
