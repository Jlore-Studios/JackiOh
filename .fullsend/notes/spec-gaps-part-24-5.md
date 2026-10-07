# Spec gaps: part 24, chunk 5 (engine tests 1: effect verbs, files 43–50)

TESTS-IN: 148 of 148 TS `it`s ported (effects-summon-copies 15, effects-summon 25, effects-summonThis 11,
effects-swap 11, effects-targets 22, effects-transform 16, effects-tune 37, effects-turnEnd 11), in 26
`mod`s for the 26 `describe`s; 0 dropped.

## SPEC GAPS

No test was dropped. These assertions read something TS can reach and Rust cannot, and are written
against the nearest observable instead (none is weaker on what the engine does to the state):

- Every TS read through a live object a helper handed back (`copy.buffs`, `unit.tuning`, `spell.costMod`,
  `held.vanilla`, `mine.controller`, …) reads the card back by id (`find_instance`), and every TS write
  through one (`source.buffs = …`, `unit.damage = 1`, `card.radiant = radiant`, …) goes through
  `find_instance_mut`. Same assertions, same values.
- `effects-summon-copies.test.ts:192-194`: `expect(copy.buffs).not.toBe(source.buffs)` (and the same for
  `statsOverride`, `grantedKeywords`) pin object identity, which Rust values never share. Ported as the
  comment states it: the source's three fields are written after the copy is made, and the copy is read
  back unchanged.
- `effects-targets.test.ts:74,85`, `effects-summonThis.test.ts:192`: `toBe(card)` (identity) on an
  instance is equality of the whole card (compared as JSON, so an owned or a lent answer both compile).
- `effects-tune.test.ts:245`: `printedEcho(echo)` is called without the state in TS; the Rust port
  (part 4.1's notes) takes it. A hand card copies no Echo, so the value pinned (2) is the same.
- `effects-turnEnd.test.ts:55`: the module-level `let nonce` is a `thread_local!` `Cell<u32>` (one per
  test thread); nonces only need to be unique within a game.
