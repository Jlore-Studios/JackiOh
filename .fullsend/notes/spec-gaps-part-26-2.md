# Spec gaps: part 26, chunk 2 (engine tests 3: view, turn, setup, combat)

TESTS-IN: 99 of 103 TS `it`s ported (combat.property 6, conditionActive 29 of 33, config 5,
control-change.property 4, control-change 15, damage-pipeline 8, damage 17, destroyed-face 2, endgame 12,
faces 7); 3 dropped by the brief, 1 with no Rust form.

## SPEC GAPS

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
