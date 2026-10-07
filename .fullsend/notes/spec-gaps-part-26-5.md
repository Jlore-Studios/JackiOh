# Spec gaps: part 26, chunk 5 of 7
TESTS-IN: 108 (replacements 28, replay_scripted 3, replay 2, restrictions 10, rng 7, rotation 12, rounds 4, self_tribute 4, setup_aside 17, setup 11, shuffle_random 3, state 7)

## SPEC GAPS
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
