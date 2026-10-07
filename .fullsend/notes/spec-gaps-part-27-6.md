# Spec gaps: part 27, chunk 6 of 8
TESTS-IN: 64 (setup_and_mulligan 9, stacks_and_reborn 8, tributes 7, trigger_stays 13 — TS's 12 `it`s, one of them in a two-value loop — turn_clock_and_legality 14, turn_stages 13)

## SPEC GAPS
Every `it` of the six TS files has a Rust `#[test]` with its assertions. No file reads source text.
One assertion is moved to where Rust enforces it, and one is made stricter; both are recorded here so
part 31 can see them:

- `packages/cards/test/turn-clock-and-legality.test.ts:296-319` ("§9.3 a play naming a zone between
  two lanes is refused …"): the TS case hands `reduce` a play with `zone: { row: "units", lane: 2.5 }`.
  SURFACE §4.3 makes a lane an integer (`ZoneChoice.lane: i32`, part 1's `wire/actions.rs`), so that
  action cannot be built in Rust and `reduce` can never see it. The port keeps the `legalActions` half
  (no offered play has a lane equal to 2.5, compared as `f64`) and asserts the refusal where Rust makes
  it: the same JSON does not deserialise into an `Action` (`serde_json::from_value::<Action>` is an
  `Err`), and p1's mana is unchanged. If the server or the WASM layer ever parsed lanes leniently, this
  test would no longer cover the TS bug; it does today, because every action reaches `reduce` through
  that `Deserialize`.
- `packages/cards/test/turn-clock-and-legality.test.ts:423-428` (R123): `expect.soft(...)` is a plain
  `assert_eq!`. Stricter: the TS soft assertion let the `reduce` check below it run after a failure;
  here a failure stops the test. No assertion is weakened.
