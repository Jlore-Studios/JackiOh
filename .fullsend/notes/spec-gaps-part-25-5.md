# Spec gaps: part 25, chunk 5 (engine tests 2: rulings-a, rulings-b)

TESTS-IN: 82 of 82 (`it`s ported one `#[test]` each: 40 from `rulings-a.test.ts`, 42 from
`rulings-b.test.ts`; 2 `describe`s, one `mod` each). No test dropped, skipped or `#[ignore]`d. Neither
file reads a `.ts` source, so nothing was dropped under #133's rule.

## SPEC GAPS

Assertions that could not be written exactly as TS wrote them, each kept as far as Rust can say it:

- `packages/engine/test/rulings-b.test.ts:1570` (R79, "leaves the clocks to the server"):
  `expect(Object.keys(engineConfig).filter((key) => SERVER_CONSTANTS.includes(key))).toEqual([])` —
  that `config.ts` exports none of the 27 server constant names. Rust has no run-time list of a module's
  items, and reading `crates/engine/src/config.rs` as text is the source-reading #133 drops. **Not
  ported**; the rest of R79 (both timeouts, the disconnect loss, the ceiling draw, `clockMs` with and
  without a clock) is. `SERVER_CONSTANTS` itself, which fed only this assertion, is not ported either;
  `rulings_b.rs` says why where it stood. A structural check (`cargo jackioh spec check`, part 28, or a
  grep in CI) could hold the same line.
- `rulings-b.test.ts:1292` (R73, "health and armor are separate fields"):
  `expect(Object.keys(state.players.p1.hero)).toEqual(["health", "armor"])`. The key *order* is a JS
  object's declaration order; serde_json without `preserve_order` hands back sorted keys. Ported as an
  exhaustive destructuring `let HeroState { health: _, armor: _ } = …` (exactly these two fields, checked
  by the compiler) plus the serialised hero's sorted key list `["armor", "health"]`. Field order is not
  asserted.
- `rulings-b.test.ts:1370` (R75): `typeof def.set === "string" && def.set.length > 0` — the
  `typeof` half is the type itself in Rust (`set: SetName`); ported as `!def.set.as_str().is_empty()`.
