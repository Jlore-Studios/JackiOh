# Spec gaps: part 25 (engine tests 2), chunk 2 of 7
TESTS-IN: 109 (copied_text 20, core_patches 15, death_pause 8, delayed_kinds 11, fuse_registry 5,
fuse_variants 17, fuse 18, glitch 15), every TS `it` ported; none dropped.

## SPEC GAPS
Assertions TS states that SURFACE.md makes inexpressible as written. Each test is ported whole; only
the assertion below is restated in Rust's terms, and why.

- `packages/engine/test/fuse-registry.test.ts:103` (`markers`): reads `registeredScripts()[defId]`
  directly. SURFACE §6.6: a fused id is never registered; its scripts are composed on lookup from the
  state (`scripts::script_of`). Rust's `markers` reads `script_of(state, def_id)`.
- `fuse-registry.test.ts:156` `expect(markers(copy, id)).toEqual([])` and `:157`
  `syncFusedScripts(copy)`: `syncFusedScripts` is not ported (SURFACE §6.6), and `script_of` composes
  the fusion at every lookup, so "nothing before the rebuild" cannot happen. Restated as
  `registered_scripts().get(id).is_none()` (the precondition TS's `[]` showed: the registry holds
  nothing for the id); the post-sync assertion runs unchanged off `script_of`.
- `fuse-registry.test.ts:165` `expect(markers(first, id)).toEqual([])` after `drop(id)`: the same,
  restated as `registered_scripts().get(id).is_none()`.
- `fuse-variants.test.ts:135` and `:140` `fusedIdSpecs(id)`: TS's answer changed from `null` to the
  specs once `legalActions` had taught the process's digest table the id. No digest table in Rust
  (SURFACE §6.6; catalog readers take `Option<&GameState>`, part 2.2): `:135` is
  `fused_id_specs(None, &id)` (no state: a digest is no fused id) and `:140` is
  `fused_id_specs(Some(&round), &id)`. `:142`/`:154` `scriptsFor(id)` is `script_of(&round, id)`.
- `fuse.test.ts:455`, `:488`, `:591`, `:592` and `fuse-variants.test.ts:223`
  `expect(<ingredient>.zone).toEqual({ z: "gone", player })`: TS read the live object `fuse` set to
  `gone`. Rust's `FuseArgs` takes owned copies and a gone card is in no pile, so nothing holds its
  zone after the call. Restated as `find_instance(state, id).is_none()` (in no pile at all, R86); the
  `player` of the gone zone is not observable.
- `glitch.test.ts:68`, `:69`, `:83`, `:95` `pickGenerated(rng, pool, { systemPlays })` and `:177`
  `seatPlayedBy({ seatSwaps: 2 }, "p1")`: TS passed bare object literals; part 2.2 made `GlitchOdds`
  the state and SURFACE §6.1 gives `seat_played_by(&GameState, ..)`. Each passes a clone of the test's
  state with only that field set (`system_plays`, `seat_swaps`).
