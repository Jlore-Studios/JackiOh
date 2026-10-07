# Spec gaps: part 27, chunk 7 of 8 (engine tests 4: play pipeline and cross-card rules)

TESTS-IN: 114 of 114 TS `it`s ported (vanilla-and-positions 10, announce 21, cost-rules 12,
counterWarning 4, draw-complete 4, draw-limit 15, draw-pause 7, draw 14, echo 10, graveyard-play 14,
mana-before-play 3); 0 dropped. No file reads source text.

## SPEC GAPS

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
