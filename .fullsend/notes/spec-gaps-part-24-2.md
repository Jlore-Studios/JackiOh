# Spec gaps: part 24, chunk 2 (engine tests 1: effect verbs, files 11–18)

TESTS-IN: 112 of 115 TS `it`s ported (effects-combat 19, effects-core 21 of 24, effects-cost 9,
effects-counters 9, effects-cry 11, effects-damage 8, effects-datacenter 19, effects-delay 15); 3
dropped, below.

## SPEC GAPS

Dropped (part 24's brief, Risks: a test that reads source text is dropped, #133's rule):

- `packages/engine/test/effects-core.test.ts:189` — "M3-T1 every effect has its own test file, or is
  one this file owns": `readdirSync` of `packages/engine/src/effects` and `packages/engine/test`,
  checked against the file's `CORE_OWNED` and `TESTED_BY` tables (`:139-185`, not ported with it).
  The Rust equivalent is a structural check over `crates/engine/src/effects/*.rs` and
  `crates/engine/tests/rules/effects_*.rs`, which belongs to part 28's structural spec checks.
- `packages/engine/test/effects-core.test.ts:225` — "M3-T1 a card script never mutates state
  (CLAUDE.md rule 5, §10.9)": `readFileSync` of every `packages/cards/src/**/*.ts`, grepping for an
  assignment into `state.…`. In Rust the card files cannot reach `GameState` fields through the
  prelude's read helpers; a grep over `crates/cards/src/scripts/**` would be part 28's.
- `packages/engine/test/effects-core.test.ts:240` — "M3-T1 grep -r \"state.players[\" packages/cards
  returns nothing": the same `readFileSync` walk, grepping `state.players[`.

Ported against the nearest Rust observable (none weaker on what the engine does to the state):

- Every TS read through a live card object (`unit.damage`, `card.costMod`, `scribe.zone`) reads the
  card back by id (`find_instance`); every TS write through one (`card.x = 3`,
  `defending.position = "DEF"`, `unit.markedDestroyed = true`) goes through `find_instance_mut`.
  Every card these files read again is in a pile (none ceased to exist), so the lookups always
  find it.
- `effects-combat.test.ts:360` `expect(sink.state).toBe(state)` (identity: the AI played out the turn
  on the caller's own object) is `std::ptr::eq(&*sink.state, address)` against the address taken
  before the sink borrowed the state.
- `effects-combat.test.ts:591-592` `cloneState(...)` equality is trivial in Rust (`clone_state` is
  `clone()`); kept as written, and the test also round-trips the paused state through
  `serde_json` and compares `declaredAttack` and `work`, which is what "everything owed is plain
  JSON" (§10.1) means.
- `effects-combat.test.ts:576-583` `expect.any(String)`/`expect.any(Number)` inside `toEqual`: the
  declaration's fields are compared one by one, `exitsFrom` as `is_some()`.
- `effects-core.test.ts:775-777` `typeof effect.kind === "string"` and `typeof effect.apply ===
  "function"`: the Rust types say both; the test binds them to `&'static str` and `&EffectApply`
  (compile-time) and keeps the non-empty kind and the distinct-kinds assertions.
- `effects-delay.test.ts:399` `it.fails(...)` is `#[should_panic]` (green while the gap is open, red
  once `delay` takes a `defId`), with the TS comment carrying the fix kept.
- `toMatchObject` (effects-core `:277`, `:628`, `:635`; effects-delay `:155`, `:181`, `:198`, `:207`) is a local `matches_object` over the serialised JSON (objects by subset, arrays by length
  and element), or a field-by-field compare where the object is two numbers.
- `toEqual` on event lists, prompt options and views compares serialised JSON (absent optionals
  compare as TS's `undefined`); `indexOf` keeps TS's `-1` for a missing type.
- TS module counters (`nextIndex` in effects-core and effects-delay, `nonce` in effects-delay) are
  written out (`ec-castable` index 1701, `dl-bolt` index 1501) or a `thread_local!` `Cell` counter.
