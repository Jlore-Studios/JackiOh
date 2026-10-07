# Spec gaps: part 26, chunk 3 (engine tests 3: game-summary, generation-replay, glow-facts, handicap, hotseat.smoke, instance-data, kill-credit, layers)

TESTS-IN: 121 of 121 (`it`s ported one `#[test]` each; 21 `describe`s, one `mod` each). No test dropped,
skipped or `#[ignore]`d. No test reads a `.ts` source file, so none was dropped under #133's rule.

## SPEC GAPS

Assertions that could not be written exactly as TS wrote them, each kept as far as Rust can say it.

### Values TS's `Handicap` type does not allow (`crates/engine/tests/rules/handicap.rs`)

TS cast objects past its own type (`as unknown as Handicap`) to prove `validateHandicap` refuses them.
Rust's `config::Handicap` holds `i32`s and `hero_health: Option<i32>`, so such a value cannot reach
`validate_handicap`. Each is built as JSON and counts as refused when it does not deserialise into a
`Handicap` (`handicap_refused`), or, if it does, when `validate_handicap` refuses it:

- `packages/engine/test/handicap.test.ts:324-329` (R180, missing `manaCap`, `manaBonus: "1"`,
  `deckSize: null`): refused by serde. Kept.
- `handicap.test.ts:331-348` (R180, `manaCap: 4.5`, `deckSize: 20.5`, `extraDrawsPerTurn: NaN`,
  `manaBonus: Infinity`): the fractions are refused by serde. JSON has no `NaN` or `Infinity`;
  `json!(f64::NAN)` and `json!(f64::INFINITY)` are `null`, which no `i32` field reads, so those two are
  refused as `null`, not as non-finite numbers. The negative and out-of-range cases go through
  `validate_handicap` as TS's did.
- `handicap.test.ts:350-358` (R180, `0.5` in each field): refused by serde.
- `handicap.test.ts:364` (R180, `createGame` with `manaCap: 1.5`): the options are written as JSON and
  refused when they do not deserialise into `CreateGameOptions` (`create_refusal_json`); no game is made.
- `handicap.test.ts:1167-1178` (R290, `heroHealth` of `0, -1, 2.5, NaN, Infinity, "20", null`, each with
  the message "p2: handicap heroHealth must be a positive integer (R290)"): `0` and `-1` keep the message
  through `validate_handicap`. `2.5` and `"20"` are refused by serde, without TS's message. **`NaN`,
  `Infinity` and `null` cannot be ported**: as JSON all three are `null`, and `Option<i32>` reads `null` as
  an absent `heroHealth`, which is the valid "starts at HERO_HEALTH" handicap. The type makes the three
  unrepresentable rather than refused; no assertion is written for them.
- `handicap.test.ts:1180-1201` (R290, `createGame` with `heroHealth` `0, -1, 2.5`): `0` and `-1` keep
  the message through `create_game`'s panic; `2.5` is refused as JSON options (no game, no TS message).

### Identity checks (`toBe` / `includes` on objects)

- `handicap.test.ts:308` and `:1065` (`expect(state.players.p2.handicap).not.toBe(bonus)` / `AI_TUTORIAL`):
  "stored as a copy" is ownership in Rust. Ported as `!std::ptr::eq(stored, &original)`, which holds by
  construction; the value equality on the line before is asserted as in TS.
- `handicap.test.ts:1025` (`Object.values(AI_DIFFICULTY)).not.toContain(AI_TUTORIAL)`): JS `includes`
  compares objects by identity, so TS's assertion could not fail. Ported by value
  (`![easy, medium, hard].contains(&AI_TUTORIAL)`), which is what the test means and what the
  `not.toEqual` loop below it asserts anyway.
- `layers.test.ts:680` (`expect(first).toMatchObject(...)` after the board changed): a Rust `UnitView` is
  a value, so "the first view is untouched" holds by construction. Kept as written.

### Live objects

TS tests held the live `CardInstance` that `put`/`inHand`/`setLibrary` returned and read or wrote it after
the engine moved it. The ports write by id (`find_instance_mut`) and read the card back from the state by
id (`find_instance`) before each engine call and assertion. Every assertion is kept; where TS read a
card's fields after `moveToZone` (`instance-data.test.ts:119-124`) or after a draw
(`instance-data.test.ts:104-106`), the port reads the card where it landed.

### Harness that has no Rust counterpart

- `glow-facts.test.ts:57-65` (`beforeAll`/`afterAll` saving and restoring the registries): the testkit's
  registries are thread-local and each `#[test]` runs on its own thread, so nothing outlives a test; not
  ported (noted in the file's header).
- `{ timeout: … }` on ten `it`s (game-summary:132, generation-replay:106, handicap:370, 386, 400, 416,
  1227, 1248, hotseat.smoke:10, instance-data:251): cargo test has no per-test timeout; dropped.
