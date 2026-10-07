# Part 27: spec gaps (all chunks)

TESTS-IN: 676 (the sum of the chunks below)

## SPEC GAPS

### From `spec-gaps-part-27-1.md`

#### Spec gaps: part 27, chunk 1 of 8
TESTS-IN: 129 (after_resolution 12, combat_windows 22, condition_active 66, control_change_carry 2, control_change 20, costs_and_mana 7)

#### SPEC GAPS
None: every `it` of the six TS files (and every iteration of the two `for`s in condition-active that
declare `it`s) has a Rust `#[test]` with the same assertions. No file reads source text, so none was
dropped under #133's rule. The names each test calls are listed under GAPS in `part-27-1.md`.

### From `spec-gaps-part-27-2.md`

#### Spec gaps: part 27, chunk 2 of 8
TESTS-IN: 54 (deaths_and_reborn 10, echo_and_exile 6, forced_attacks 4, fuse_registry 1, fused_hooks 18, fused_nested_resume 1, fused_target_checks 2, game_over 4, hand_returns 8)

#### SPEC GAPS
None: every `it` of the nine TS files has a Rust `#[test]` with the same assertions. No file reads
source text. The names each test calls are listed under GAPS in `part-27-2.md`.

### From `spec-gaps-part-27-3.md`

#### Spec gaps: part 27, chunk 3 of 8
TESTS-IN: 52 (hidden_information 37, lasting_effects 6, my_pawn 9)

#### SPEC GAPS
None: every `it` of the three TS files has a Rust `#[test]` with the same assertions. No file reads
source text. The names each test calls are listed under GAPS in `part-27-3.md`.

### From `spec-gaps-part-27-4.md`

#### Spec gaps: part 27, chunk 4 (cards cross tests: paused-sequences, play-choices, plays-and-casts)

TESTS-IN: 64 of 64 (`it`s ported one `#[test]` each: paused-sequences 45, play-choices 7,
plays-and-casts 12; 54 `describe`s, one `mod` each). No test dropped, skipped, `#[ignore]`d or weakened.

#### SPEC GAPS

Nothing was left out. Two TS constructs have no Rust counterpart and are ported as close as Rust says it:

- `packages/cards/test/plays-and-casts.test.ts:292` (`LONG_PLAY_TIMEOUT_MS = 60_000`, passed as vitest's
  per-test timeout at line 341): a Rust `#[test]` has no timeout, so the R58/R350 Call to Chaos test runs
  without one (`crates/cards/tests/cross/plays_and_casts.rs`). Its comment is kept.
- `packages/cards/test/plays-and-casts.test.ts:277` and `:331` (`expect(() => g.play(…)).not.toThrow()`):
  ported as the plain call, which panics, failing the test, on any refusal or engine panic — the same
  assertion, without a wrapper.

No test reads a `.ts` source file, so none was dropped under #133's rule.

### From `spec-gaps-part-27-5.md`

#### Spec gaps: part 27, chunk 5 (preview, re-entry, resolving-face, self-generation)

TESTS-IN: 131 — `crates/cards/tests/cross/preview.rs` 95 (73 TS `it`s, 22 more from the TS loops
expanded one `#[test]` per case), `re_entry.rs` 27, `resolving_face.rs` 7, `self_generation.rs` 2.
Every TS `it` has its Rust `#[test]`; none is dropped, skipped or `#[ignore]`d.

#### SPEC GAPS

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

### From `spec-gaps-part-27-6.md`

#### Spec gaps: part 27, chunk 6 of 8
TESTS-IN: 64 (setup_and_mulligan 9, stacks_and_reborn 8, tributes 7, trigger_stays 13 — TS's 12 `it`s, one of them in a two-value loop — turn_clock_and_legality 14, turn_stages 13)

#### SPEC GAPS
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

### From `spec-gaps-part-27-7.md`

#### Spec gaps: part 27, chunk 7 of 8 (engine tests 4: play pipeline and cross-card rules)

TESTS-IN: 114 of 114 TS `it`s ported (vanilla-and-positions 10, announce 21, cost-rules 12,
counterWarning 4, draw-complete 4, draw-limit 15, draw-pause 7, draw 14, echo 10, graveyard-play 14,
mana-before-play 3); 0 dropped. No file reads source text.

#### SPEC GAPS

No test was dropped. These assertions read something TS reached that Rust reaches another way, and are
written against the nearest observable (none is weaker on what the engine does to the state):

- Every TS read through a live object (`card.zone` after a cast in `echo.test.ts:342`, `big` after
  `big.embiggened = true` in `cost-rules.test.ts:312`, `g.card(rock)` after
  `rock.grantedKeywords.push(…)` in `vanilla-and-positions.test.ts:174`) reads the card back by id from
  the state; every TS write through one goes through `find_instance_mut`. Same assertions, same values.
- `draw-limit.test.ts:213-217` and `cost-rules.test.ts:262-264`: TS read `state` while a context built on
  it was alive; Rust reads `ctx.state` (the same state) until the context is dropped.
- `graveyard-play.test.ts:281-295`: TS's `try { … } finally { registerScripts(scripts) }`. The registry
  override is thread-local (SURFACE §8), so a failing assertion cannot leak it into another test; the
  restore runs after the assertions, as TS's `finally` does on success.
- `echo.test.ts:173-176`: TS reads `echoQueue` structurally because its `GameState` type lacked the
  field; part 1 froze `GameState.echo_queue`, so the port reads it directly and `Object.hasOwn` reads
  the state's JSON for the key.
- `graveyard-play.test.ts:94` (`/no card .* in p1's hand/`) is the only TS regex here that is not a
  literal; it is a hand check (the text holds `no card `, and after it ` in p1's hand`).
- Fixture shapes (`PA` as a static with snake_cased `CardDef` fields; `plays_of`'s element type; the
  harness's `put` options and `set_library`'s slice type) are guesses at part 24's fixtures, listed under
  GAPS in `part-27-7.md`; part 31 reconciles the call sites, not the assertions.

### From `spec-gaps-part-27-8.md`

#### Spec gaps: part 27, chunk 8 of 8
TESTS-IN: 68 of 68 (`mana` 11, `overflow-events` 13, `play-pipeline-b-replay` 1, `play-step3` 12,
`playChoices-filters` 14, `playChoices` 8, `playCounts` 9)

#### SPEC GAPS
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

