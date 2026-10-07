# Part 24: spec gaps (all chunks)

TESTS-IN: 630 (the sum of the chunks below)

## SPEC GAPS

### From `spec-gaps-part-24-1.md`

#### Spec gaps: part 24, chunk 1 (engine tests 1: effect verbs, files 1–10)

TESTS-IN: 94 of 94 TS `it`s ported (effects-after-check 4, effects-animate 5, effects-boardwide 27,
effects-brittle 5, effects-buff 12, effects-cardScope 4, effects-cast-chaos 2, effects-cast 20,
effects-choose-where 4, effects-choose 11); 0 dropped.

#### SPEC GAPS

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

### From `spec-gaps-part-24-2.md`

#### Spec gaps: part 24, chunk 2 (engine tests 1: effect verbs, files 11–18)

TESTS-IN: 112 of 115 TS `it`s ported (effects-combat 19, effects-core 21 of 24, effects-cost 9,
effects-counters 9, effects-cry 11, effects-damage 8, effects-datacenter 19, effects-delay 15); 3
dropped, below.

#### SPEC GAPS

Dropped (part 24's brief, Risks: a test that reads source text is dropped, #133's rule):

- `packages/engine/test/effects-core.test.ts:189` — "M3-T1 every effect has its own test file, or is
  one this file owns": `readdirSync` of `packages/engine/src/effects` and `packages/engine/test`,
  checked against the file's `CORE_OWNED` and `TESTED_BY` tables (`:139-185`, not ported with it).
  The Rust equivalent is a structural check over `crates/engine/src/effects/*.rs` and
  `crates/engine/tests/rules/effects_*.rs`, which belongs to part 28's structural spec checks.
- `packages/engine/test/effects-core.test.ts:225` — "M3-T1 a card script never mutates state
  (CLAUDE.md rule 5, §10.9)": `readFileSync` of every `packages/cards/src/**/*.ts`, grepping for an
  assignment into `state.…`. In Rust the card files cannot reach `GameState` fields through the
  prelude's read helpers; a grep over `crates/cards/src/scripts/**` would be part 28's.
- `packages/engine/test/effects-core.test.ts:240` — "M3-T1 grep -r \"state.players[\" packages/cards
  returns nothing": the same `readFileSync` walk, grepping `state.players[`.

Ported against the nearest Rust observable (none weaker on what the engine does to the state):

- Every TS read through a live card object (`unit.damage`, `card.costMod`, `scribe.zone`) reads the
  card back by id (`find_instance`); every TS write through one (`card.x = 3`,
  `defending.position = "DEF"`, `unit.markedDestroyed = true`) goes through `find_instance_mut`.
  Every card these files read again is in a pile (none ceased to exist), so the lookups always
  find it.
- `effects-combat.test.ts:360` `expect(sink.state).toBe(state)` (identity: the AI played out the turn
  on the caller's own object) is `std::ptr::eq(&*sink.state, address)` against the address taken
  before the sink borrowed the state.
- `effects-combat.test.ts:591-592` `cloneState(...)` equality is trivial in Rust (`clone_state` is
  `clone()`); kept as written, and the test also round-trips the paused state through
  `serde_json` and compares `declaredAttack` and `work`, which is what "everything owed is plain
  JSON" (§10.1) means.
- `effects-combat.test.ts:576-583` `expect.any(String)`/`expect.any(Number)` inside `toEqual`: the
  declaration's fields are compared one by one, `exitsFrom` as `is_some()`.
- `effects-core.test.ts:775-777` `typeof effect.kind === "string"` and `typeof effect.apply ===
  "function"`: the Rust types say both; the test binds them to `&'static str` and `&EffectApply`
  (compile-time) and keeps the non-empty kind and the distinct-kinds assertions.
- `effects-delay.test.ts:399` `it.fails(...)` is `#[should_panic]` (green while the gap is open, red
  once `delay` takes a `defId`), with the TS comment carrying the fix kept.
- `toMatchObject` (effects-core `:277`, `:628`, `:635`; effects-delay `:155`, `:181`, `:198`, `:207`) is a local `matches_object` over the serialised JSON (objects by subset, arrays by length
  and element), or a field-by-field compare where the object is two numbers.
- `toEqual` on event lists, prompt options and views compares serialised JSON (absent optionals
  compare as TS's `undefined`); `indexOf` keeps TS's `-1` for a missing type.
- TS module counters (`nextIndex` in effects-core and effects-delay, `nonce` in effects-delay) are
  written out (`ec-castable` index 1701, `dl-bolt` index 1501) or a `thread_local!` `Cell` counter.

### From `spec-gaps-part-24-3.md`

#### Spec gaps: part 24, chunk 3 (engine tests 1: effect verbs, files 19–30)

TESTS-IN: 138 of 138 TS `it`s ported (effects-destroy 15, effects-drawWhile 6, effects-each 2,
effects-enchant 5, effects-flicker 11, effects-fruit 30, effects-give 17, effects-hand-exile 5,
effects-heal 8, effects-health 6, effects-library 21, effects-locks 12); 0 dropped. Every `describe`
is a `mod` (34 of 34).

#### SPEC GAPS

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

### From `spec-gaps-part-24-4.md`

#### Spec gaps: part 24, chunk 4 (engine tests 1: effect verbs, files 31–42)

TESTS-IN: 138 of 138 TS `it`s ported (effects-move 27, effects-perks 6, effects-plague-random-cast 3,
effects-plague 23, effects-plus-c 14, effects-radiant 13, effects-random 27, effects-reveal 3,
effects-shuffle-card 6, effects-split 4, effects-statuses 4, effects-steal 8); 0 dropped.

#### SPEC GAPS

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

### From `spec-gaps-part-24-5.md`

#### Spec gaps: part 24, chunk 5 (engine tests 1: effect verbs, files 43–50)

TESTS-IN: 148 of 148 TS `it`s ported (effects-summon-copies 15, effects-summon 25, effects-summonThis 11,
effects-swap 11, effects-targets 22, effects-transform 16, effects-tune 37, effects-turnEnd 11), in 26
`mod`s for the 26 `describe`s; 0 dropped.

#### SPEC GAPS

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

### From `spec-gaps-part-24-6.md`

#### Spec gaps: part 24, chunk 6 (engine test fixtures)
TESTS-IN: 0 (the fifteen files ported are `packages/engine/test/fixtures/*.ts` helpers: no `describe` or `it`)

#### SPEC GAPS
(none: no test was left unported)

