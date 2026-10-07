# Spec gaps: part 25, chunk 6 (engine tests 2: rulings-c, scorer, start-of-opponent-turn)

TESTS-IN: 65 of 65 (`rulings-c.test.ts` 55 `it`s in 15 `describe`s, `scorer.test.ts` 9 in 1,
`start-of-opponent-turn.test.ts` 1 in 1; one `#[test]` per `it`, one `mod` per `describe`). No test
dropped, skipped or `#[ignore]`d. No test reads a `.ts` source file, so none was dropped under #133's rule.

## SPEC GAPS

Assertions that could not be written exactly as TS wrote them, each kept as far as Rust can say it:

- `packages/engine/test/rulings-c.test.ts:1161` (R102 multi-target): `expect(shared.zone).toEqual({ z:
  "gone", player: "p1" })` reads the TS object the test still held after the fusion consumed it. A Rust
  test holds a copy; a card that is in no pile cannot be read back from the state, and `fuse` returns only
  the kept instance. The two assertions after it (`findInstance(...) === undefined`, the lane empty) are
  kept and say the same thing from the state's side. The copy handed to the second fusion has its zone set
  to `gone` by the test, as TS's shared object had been by the engine.
- `packages/engine/test/rulings-c.test.ts:1190` (R102 consumed): `expect(eaten.zone).toEqual({ z: "gone",
  player: "p1" })`, the same: ported as `find_instance(&state, &eaten.id).is_none()` beside the kept
  lane, graveyard and exile assertions.
- `packages/engine/test/rulings-c.test.ts:2154-2155` (R135): `expect(Object.keys(effects)).toContain(
  "exileMatching")` / `"drawFromLibrary"`. Rust has no runtime list of a module's exports; the port names
  `effects::exile_matching` and `effects::draw_from_library` as values, which compiles only if both verbs
  exist (a compile-time form of the same check).
- `packages/engine/test/rulings-c.test.ts:1854` (R126): `typeof scriptsFor(...).base.resume?.later ===
  "function"` is `resume.contains_key("later")`: every value of `Script.resume` is a `Hook`.
- `packages/engine/test/rulings-c.test.ts:2444` (R151): `expect(powerOf(card)).toBe(kept)` is an identity
  check on a `HERO_POWERS` table entry. Ported as the same power by name (compared as JSON) and `x`.
- `packages/engine/test/rulings-c.test.ts:2517, 2524` (R155): TS pushed one object into the graveyard (or
  exile) without taking it out of the hand, so one card sat in two piles and a flag written to it showed
  in both. Rust cannot alias it: the hand copy's zone is set and a copy pushed to the second pile; the
  assertion checks that no copy of that id in hand, graveyard or exile is flagged.
- `packages/engine/test/rulings-c.test.ts:1345-1346` (R113): `expect(() => runWorkItem(...)).toThrow(/…/)`
  is a `catch_unwind` around the call (SURFACE §4.4.9: a TS throw on an impossible state is a `panic!`
  with the same message), matched with `contains`. `not.toThrow()` (lines 1856, 1875, 1906) is the call
  itself: a panic fails the test.
