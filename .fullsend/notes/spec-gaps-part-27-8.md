# Spec gaps: part 27, chunk 8 of 8
TESTS-IN: 68 of 68 (`mana` 11, `overflow-events` 13, `play-pipeline-b-replay` 1, `play-step3` 12,
`playChoices-filters` 14, `playChoices` 8, `playCounts` 9)

## SPEC GAPS
None: every `it` is ported with its assertions.

Assertions written against the nearest Rust observable (same values, different handle):
- `overflow-events.test.ts:154` `expect(token.zone.z).toBe("gone")`: TS read the live object. The port
  reads the copy handed to `shuffle_into_library(&mut CardInstance, …)`, which part 2.1's `move_to_zone`
  contract leaves "as it landed (zone included)". If the Rust call does not update its argument, the
  card has ceased to exist and is in no zone `find_instance` reaches; part 31 picks the handle.
- `playChoices.test.ts:238` `expect(buried.damage).toBe(0)`: read back by id from the state.
- `playChoices-filters.test.ts:250`, `:464`; `playChoices.test.ts:237`, `:437` `expect(x.state).toBe(state)`
  (object identity): value equality on `GameState`.
- `play-step3.test.ts:70`, `:86` `toMatchObject`: a local subset matcher over the events' JSON.
- `playChoices.test.ts:358` `new Set(…)` equality: a `BTreeSet<String>`.
- `play-pipeline-b-replay.test.ts:140` the 120 s vitest timeout: dropped (Rust tests take none).
