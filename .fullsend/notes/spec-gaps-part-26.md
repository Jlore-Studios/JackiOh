# Part 26: spec gaps (all chunks)

TESTS-IN: 696 (the sum of the chunks below)

## SPEC GAPS

### From `spec-gaps-part-26-1.md`

#### Spec gaps: part 26, chunk 1 of 7
TESTS-IN: 124 (after_attack 8, animated 22, auto_end_turn 7, backrow_death 3, backrow_piles 19, brittle 21, carried_damage 4, combat_positions 10, combat_resolution 12, combat_validation 18)

#### SPEC GAPS
None: every `it` of the ten TS files has a Rust `#[test]` with the same assertions. No file reads
source text. The names each test calls are listed under GAPS in `part-26-1.md`.

### From `spec-gaps-part-26-2.md`

#### Spec gaps: part 26, chunk 2 (engine tests 3: view, turn, setup, combat)

TESTS-IN: 99 of 103 TS `it`s ported (combat.property 6, conditionActive 29 of 33, config 5,
control-change.property 4, control-change 15, damage-pipeline 8, damage 17, destroyed-face 2, endgame 12,
faces 7); 3 dropped by the brief, 1 with no Rust form.

#### SPEC GAPS

Not ported:
- `conditionActive.test.ts:256`, `it("R195 B1: only an answer of exactly true lights the card")`: the
  test hands the `conditionMet` hook a truthy non-boolean (`1`, `"yes"`) and checks it lights nothing.
  Part 1's `ConditionHook` is `Arc<dyn Fn(ConditionContext) -> bool>`, so a Rust hook can answer only
  `true` or `false`: the rule holds by the type, and the test's remaining assertions (key absent, hook
  asked) are `r195_b1_when_the_hook_returns_false_the_key_is_absent_never_false`'s.
- `conditionActive.test.ts:765–825`, `describe("R195 in SPEC §10.8, §10.9 and §11, and in the rulings
  index (B10)")` and its three `it`s: dropped by the brief (#133; they read SPEC.md's prose and
  `rulings.test.ts`'s source). l.765–769 are the block's heading comment and its `textOf` helper.

Ported against the nearest Rust observable (no assertion on what the engine does is weaker):
- `conditionActive.test.ts:716`, "R196 each ingredient's hook is asked about the fused card itself, and
  only an answer of exactly true counts": TS answers `1` and `"yes"`; the port answers `false` both (a
  `bool` hook's only non-`true` answer), keeping every assertion: no glow, both ingredients asked, each
  about the fused card, as its controller's, in hand, on their turn.
- `control-change.property.test.ts`: fast-check's generator and shrinking are not available (SURFACE §2
  lists no such crate). The cases come from the engine's seeded `Rng` with the same arbitraries (shapes,
  value sets, `fc.option`'s one-in-five nil, one to three verbs) and TS's run counts (300, 300, 60, 150),
  one reproducible stream per run; the four properties are asserted unchanged on every case. The cases
  themselves differ from fast-check's.
- Every TS read or write through a live `CardInstance` reads or writes the card in the state by id
  (`find_instance`/`find_instance_mut`); `toBe(object)` on a hook's `ctx.self` is value equality with
  the live card, and `toBe(state)` compares the state's address.
- `toEqual` on events and views compares serde JSON (absent `Option`s are absent keys, as TS's
  `undefined`); `toMatchObject` is a local subset match over JSON.

### From `spec-gaps-part-26-3.md`

#### Spec gaps: part 26, chunk 3 (engine tests 3: game-summary, generation-replay, glow-facts, handicap, hotseat.smoke, instance-data, kill-credit, layers)

TESTS-IN: 121 of 121 (`it`s ported one `#[test]` each; 21 `describe`s, one `mod` each). No test dropped,
skipped or `#[ignore]`d. No test reads a `.ts` source file, so none was dropped under #133's rule.

#### SPEC GAPS

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

### From `spec-gaps-part-26-4.md`

#### Spec gaps: part 26, chunk 4 of 7 (engine tests 3: lethal, library-copies, mulligan-concurrent, ownLibrary, params, pools, preview-ids, preview, query, recruit-variants, reduce)

TESTS-IN: 112 of 112 (`it`s ported one `#[test]` each; 30 `describe`s, one `mod` each). No test dropped, skipped or `#[ignore]`d.

#### SPEC GAPS

No test was left unported. Assertions that could not be written exactly as TS wrote them, each kept as far as Rust can say it:

- `packages/engine/test/reduce.test.ts:56` (`dedupes a repeated nonce`): `expect(second.state).toBe(first.state)` is object identity. A Rust value has no identity; ported as `assert_eq!(second.state, first.state)` (`crates/engine/tests/rules/reduce.rs`).
- `packages/engine/test/pools.test.ts:156-157` (R387 `excludingDefId`): `expect(excludingDefId(asked, …)).toBe(asked)` is identity ("handed back as it was"). Ported as equality of the serialised queries (`crates/engine/tests/rules/pools.rs`).
- `packages/engine/test/preview.test.ts:530` (R280 "handed no rng and no event sink"): `Object.keys(ctx).sort()` equal to the six keys. A Rust struct has no runtime key list; the recording hook destructures `ConditionContext` exhaustively (no `..`), which compiles only while the context is exactly those six fields, and the test asserts the recorded key list on every call (`crates/engine/tests/rules/preview.rs`, `record`).
- `packages/engine/test/preview.test.ts:557-562`, `preview-ids.test.ts:90-96`, `query.test.ts:120-128, 169-180, 314-326` (the "hands back a copy" / "the view holds its own array" tests): TS mutated the returned array or object and checked the state or a later view. In Rust the returned values are owned, so these hold by construction; each test still mutates its copy and asserts the state and the next read unchanged.
- `packages/engine/test/params.test.ts:152-156, 165-168, 178-180` (`param(ctx, key)` on plain objects and its `toThrow`s): TS handed `param` object literals shaped like a context (`{ state, self: null, radiant, defId }`, `{ ...ctx, data }`). Ported as an `EffectContext` (`make_context` or `EffectContext::new`) with those fields set. The TS throws are panics with TS's message (SURFACE §4.4.9), caught with `std::panic::catch_unwind` and matched on the message text.
- `packages/engine/test/reduce.test.ts:122` (the walk's `30_000` ms timeout): a Rust test has no default timeout to raise; the comment is kept with a note, every assertion is ported.
- `packages/engine/test/preview.test.ts:153-164`, `preview-ids.test.ts:42-51` (`beforeAll`/`afterAll` save and restore of the registries, `beforeEach` `mockClear`): the registries are the testkit's per-thread override and each `#[test]` runs on its own thread, so there is nothing to restore; the mock logs are thread-locals that start empty. No assertion lived in those hooks.

No test reads a `.ts` source file, so none was dropped under #133's rule.

### From `spec-gaps-part-26-5.md`

#### Spec gaps: part 26, chunk 5 of 7
TESTS-IN: 108 (replacements 28, replay_scripted 3, replay 2, restrictions 10, rng 7, rotation 12, rounds 4, self_tribute 4, setup_aside 17, setup 11, shuffle_random 3, state 7)

#### SPEC GAPS
- `packages/engine/test/restrictions.test.ts` l.114–120 (in "can't attack or be attacked; a restriction
  from where a card stands is registered by the module that knows it"): the block registers a test-only
  attack bar with `registerAttackBar("test-carried", …)` and asserts, while it holds, that
  `attackTargets(state, attacker)` drops `other` (`["p1"]`) and `randomAttackTargets(state, other,
  "enemies")` is `[]`, then restores the bar. SURFACE §6.6 does not port `registerAttackBar` ("never
  registered"), so there is nothing to call. The test's other assertions are ported
  (`crates/engine/tests/rules/restrictions.rs`), including l.121's `attackTargets(...).length === 2`.
- `packages/engine/test/rng.test.ts` l.27–39 ("resumes from a serialized cursor in another process"): TS
  spawned `fixtures/rng-child.ts` in a second Node process. The Rust test calls the fixture's port
  (`fixtures::rng_child::run(&[seed, cursor, count])`, the JSON the script printed) in-process and parses
  that JSON, so the draws cross a serialised boundary built from the seed and the cursor alone; every
  assertion is kept. What no longer holds literally is "another process": a Rust `Rng` is a plain value
  (SURFACE §3: no statics), so there is no process state for a second process to rule out.

### From `spec-gaps-part-26-6.md`

#### Spec gaps: part 26, chunk 6 of 7
TESTS-IN: 86 (targeting 23, temporary 5, transform_variants 7, tribute_zones 7, tribute 12, turn_cap 2, turn_wiring 8, turn 16, view_marks 6)

#### SPEC GAPS
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

### From `spec-gaps-part-26-7.md`

#### Spec gaps: part 26, chunk 7 of 7
TESTS-IN: 46 (view_for 21, windfury 6, zones 19)

#### SPEC GAPS
- `packages/engine/test/viewFor.test.ts` l.512 (`it("§10.8 carries mana, health, armor, locks, the
  phase, the result and the server's clock (R79)")`): `viewFor(state, "p1", 75_000).clockMs` passes
  TS's optional third argument `clockMs`, which SURFACE §6.1's two-argument `view_for(state, player)`
  has no room for. The test is ported whole, with that one call written as
  `view_for_with_clock(&state, PlayerId::P1, Some(75_000)).clock_ms == Some(75_000)` (the name part
  5.2's notes give the clocked variant); part 31 fixes the call if the variant is named otherwise.
  No other test was left unported or weakened, and no file reads source text.

