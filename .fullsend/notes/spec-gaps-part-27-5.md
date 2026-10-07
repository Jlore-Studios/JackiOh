# Spec gaps: part 27, chunk 5 (preview, re-entry, resolving-face, self-generation)

TESTS-IN: 131 — `crates/cards/tests/cross/preview.rs` 95 (73 TS `it`s, 22 more from the TS loops
expanded one `#[test]` per case), `re_entry.rs` 27, `resolving_face.rs` 7, `self_generation.rs` 2.
Every TS `it` has its Rust `#[test]`; none is dropped, skipped or `#[ignore]`d.

## SPEC GAPS

- `packages/cards/test/preview.test.ts:707` (`deepFreeze`), `:720` (`guarded`), `:677` and `:687`
  (Lizard's Breath's size-only `Proxy` and throwing `state.active`), `:1344`–`:1351` (Divine Favor's
  sealed hand and throwing library); used at `:507`, `:666`, `:788`, `:904`, `:981`, `:1009`,
  `:1079`, `:1185`, `:1279`, `:1336`. TS proves a preview hook is a pure read by handing it a
  deep-frozen copy whose libraries, hands and `state.active` throw on access. Rust has no throwing
  getter or `Proxy`, and SURFACE fixes `ConditionContext.state: &GameState`. Ported as follows, every
  assertion kept (the hook's answer on the copy equals the view's preview of the real state):
  - writes: impossible by the type (`&GameState`), so `deepFreeze` has nothing left to prove;
  - reads: the copy is perturbed instead of fenced (`preview.rs`'s `fenced`): a pile TS walled off
    whole is emptied, a pile TS let `length` through is rewritten card by card at the same length
    (other ids, `core-008`, reverse order), and `active` is the other seat where TS made it throw.
  - What is lost: a hook that reads a fenced pile or `state.active` but whose answer happens to be
    the same on the perturbed copy passes in Rust and threw in TS (e.g. both seats sit at 4 mana in
    the "each Core hook" placements, so a #18 hook reading `state.active` instead of `yourTurn`
    would not be caught there). Part 31 may want a testkit fence (a `GameState` copy with poisoned
    piles the engine panics on) if it judges this worth closing.
- `packages/cards/test/self-generation.test.ts:149`: the `it`'s `{ timeout: 120_000 }` option has no
  Rust counterpart (the runner has no per-test timeout); dropped, the test itself is unchanged.
