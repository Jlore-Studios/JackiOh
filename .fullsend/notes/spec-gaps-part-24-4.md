# Spec gaps: part 24, chunk 4 (engine tests 1: effect verbs, files 31–42)

TESTS-IN: 138 of 138 TS `it`s ported (effects-move 27, effects-perks 6, effects-plague-random-cast 3,
effects-plague 23, effects-plus-c 14, effects-radiant 13, effects-random 27, effects-reveal 3,
effects-shuffle-card 6, effects-split 4, effects-statuses 4, effects-steal 8); 0 dropped.

## SPEC GAPS

No test was dropped. These assertions read something TS can reach and Rust cannot, and are written
against the nearest observable instead (none is weaker on what the engine does to the state):

- Every TS read through a live object a fixture handed back (`victim.controller`, `unit.buffs`,
  `trap.revealed`, `plagueOn(base)`, `card.radiant`, …) reads the card back by id from the state
  (`find_instance`); every TS write through one before a run (`victim.damage = 1`, `book.radiant = true`,
  `fresh.summonedTurn = …`, `guard.position = "DEF"`, `base.vanilla = true`) goes through
  `find_instance_mut`. Same assertions, same values.
- `effects-plague.test.ts:141-147`: `expect(() => act(run, answer…)).toThrow(/not one of the options/)`
  and `/other player/`, and `:188` `expect(() => answer(run1, pick(theirs), "p1")).toThrow(/other
  player/)`, use `generation::refusal(&run, body)` — the error of the same `reduce` that `act`/`answer`
  throw on — and check it `contains` the text. `expect(run.state).toBe(refused)` (identity) is equality
  with a copy taken before (`act` takes `&Run` and cannot change it).
- `effects-plague.test.ts:110`: `round.pending?.resume.data` `toMatchObject` reads the data bag as
  serialised JSON.
- `effects-random.test.ts:524`: TS also set `victim.zone = { z: "gone" }` on the detached object after
  emptying its lane; no pile holds that object, so the state is the same without it (no Rust line).
- `effects-random.test.ts:571-578` (`:574`): `instanceIds: expect.any(Array)` is `is_array()` on that field, and
  the rest of the `fused` event is compared whole.
- `effects-plus-c.test.ts:88`: the module nonce counter (`pcv<n>`, shared by every test in the file) is
  `pcv<applied.len() + 1>`, unique within each game; no assertion reads a nonce.
- `effects-statuses.test.ts:82-90`: TS read `liveFresh` (a live object found before the effects ran)
  after `mayAttackAgain` wrote through it; the Rust test reads the card under that id after the run.
