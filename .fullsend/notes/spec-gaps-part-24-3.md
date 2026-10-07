# Spec gaps: part 24, chunk 3 (engine tests 1: effect verbs, files 19–30)

TESTS-IN: 138 of 138 TS `it`s ported (effects-destroy 15, effects-drawWhile 6, effects-each 2,
effects-enchant 5, effects-flicker 11, effects-fruit 30, effects-give 17, effects-hand-exile 5,
effects-heal 8, effects-health 6, effects-library 21, effects-locks 12); 0 dropped. Every `describe`
is a `mod` (34 of 34).

## SPEC GAPS

No test was dropped. These assertions read something TS can reach and Rust cannot, and are written
against the nearest observable instead (none is weaker on what the engine does to the state):

- `effects-library.test.ts:389` and `:444`: `expect(token.zone).toEqual({ z: "gone", player: "p1" })`
  reads a unit-token card's own object after it ceased to exist (R11, R86). A Rust test holds a clone,
  and a card that is gone is in no pile, so `find_instance` cannot return it. Ported as "the card is in
  no zone of the state" (`find_instance(&state, &token.id).is_none()`), which is the comment's first
  half ("it is in no pile"); the zone half is not observable. If part 31 decides gone cards stay
  findable, these become `zone == Zone::Gone { player: PlayerId::P1 }` (same as part 24.1's gap).
- `effects-fruit.test.ts:345`: `expect(token?.zone.z).not.toBe("graveyard")` on a gone token: ported as
  `zone_of(&state, id) != Some(ZoneName::Graveyard)` (None for a card in no pile), which holds as TS's.
- `effects-give.test.ts:304`: `expect(card.owner).toBe("p2")` reads the object handed to
  `takeIntoHand`/`changeOwner`. Rust hands them `&CardInstance`, so the local copy cannot change; the
  assertion is kept on the copy, plus `find_instance(&state, &card.id).is_none()` (nothing in the state
  became the card either).
- `effects-flicker.test.ts:138-139`: TS's live `trap` takes the fresh face-down id (R227), so
  `cardAt(...)?.id === trap.id` compares the zone's card with that object. Ported as: the card now in
  p2's backrow lane 2 has an id other than the old one, is a Watcher (`def_id`), and the old id is in
  no zone; every later TS read of `trap` reads that card.
- `effects-enchant.test.ts:74`: TS empties p1's exile and places the live object it still holds; Rust
  takes the card's copy from the state just before emptying the pile and places that.
- `effects-enchant.test.ts:98-99`: `unitedEnchantments([{ enchantments: [CAST] }, {}])` takes
  `Pick<CardInstance, "enchantments">` literals; Rust builds two `CardInstance`s with `new_instance`
  over a local `u32` counter (`NextId`), so the game's own ids are untouched, and hands the slice.
- `effects-each.test.ts:86`: the script's `asksAfter` closes over a variable the test sets after the
  library is dealt. A `Hook` is `Fn + Send + Sync` and clippy.toml bans `Mutex`/`RefCell`, so it is an
  `Arc<OnceLock<String>>` set once (unset is TS's `undefined`).
- Every TS test that reads a card through the live object reads it back by id from the state
  (`find_instance`), every TS write through one goes through `find_instance_mut`, and every engine call
  that took the live object is handed the card's copy read from the state right before the call.
