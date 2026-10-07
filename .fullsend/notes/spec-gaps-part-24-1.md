# Spec gaps: part 24, chunk 1 (engine tests 1: effect verbs, files 1–10)

TESTS-IN: 94 of 94 TS `it`s ported (effects-after-check 4, effects-animate 5, effects-boardwide 27,
effects-brittle 5, effects-buff 12, effects-cardScope 4, effects-cast-chaos 2, effects-cast 20,
effects-choose-where 4, effects-choose 11); 0 dropped.

## SPEC GAPS

No test was dropped. These assertions read something TS can reach and Rust cannot, and are written
against the nearest observable instead (none is weaker on what the engine does to the state):

- `effects-boardwide.test.ts:468`, `:535`, `:645`, `:681` and `effects-brittle.test.ts:83-85`: TS
  reads a token or a detached instance's own object after it ceased to exist (`token.zone.z ===
  "gone"`, `held.brittle` undefined). A Rust test holds a clone, and a card that ceased to exist is
  in no pile, so `find_instance` cannot return it. Ported as "the card is in no zone of the state"
  (`find_instance(&state, id).is_none()`), plus the local clone's `brittle` for brittle:85.
  SURFACE says nothing about whether `zone: Gone` cards stay findable; if part 31 decides they do,
  these become `zone.z() == ZoneName::Gone`.
- Every TS test that reads a card through the live object (`unit.buffs`, `card.position`,
  `warded.markedDestroyed`, …) reads it back by id from the state (`find_instance`) in Rust; every TS
  write through one (`unit.position = "DEF"`, `mine.damage = 1`, `trap.faceUp = true`) goes through
  `find_instance_mut`. Same assertions, same values.
- `effects-animate.test.ts:61`: TS hands the second `makeContext` the same live object, which the
  first `animate` had moved; Rust re-reads it from the state before building the second context.
- `effects-cast-chaos.test.ts:63`, `:73`: the TS hook pushes into an outer array. A Rust `Hook` is
  `Fn + Send + Sync` and clippy.toml bans `Mutex`/`RefCell`, so the depths go through an
  `std::sync::mpsc` channel and are collected after the cast. The hook reads `ctx.live_self()` (TS's
  live `ctx.self`).
- `effects-cast.test.ts:358`: the TS title interpolates `${RANDOM_CAST_CHAIN_CAP}`; the Rust name
  is the title without the number, which is kept in a comment.
- Jest `toMatchObject` (effects-choose) is a local `matches_object` over the serialised JSON (objects
  by subset, arrays by length and element), and `toEqual` on events and views compares serialised
  JSON where TS compared object literals, so absent optionals compare as TS's `undefined` did.
