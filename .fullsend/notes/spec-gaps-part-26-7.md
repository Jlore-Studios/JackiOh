# Spec gaps: part 26, chunk 7 of 7
TESTS-IN: 46 (view_for 21, windfury 6, zones 19)

## SPEC GAPS
- `packages/engine/test/viewFor.test.ts` l.512 (`it("§10.8 carries mana, health, armor, locks, the
  phase, the result and the server's clock (R79)")`): `viewFor(state, "p1", 75_000).clockMs` passes
  TS's optional third argument `clockMs`, which SURFACE §6.1's two-argument `view_for(state, player)`
  has no room for. The test is ported whole, with that one call written as
  `view_for_with_clock(&state, PlayerId::P1, Some(75_000)).clock_ms == Some(75_000)` (the name part
  5.2's notes give the clocked variant); part 31 fixes the call if the variant is named otherwise.
  No other test was left unported or weakened, and no file reads source text.
