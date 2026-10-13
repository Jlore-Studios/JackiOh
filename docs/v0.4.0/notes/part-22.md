# v0.4.0 Part 22: Meditative Cry cards, first half, and the data-only cards (24 cards and 3 held back)

Notes for issue #761 (v0.4.0 part 22 of 29, batch 14): the 24 Meditative cards of the plan and
the 3 Classic+ cards earlier batches held back (#442, L27) ported from Rust `script()` to Luau,
none staying in Rust, and the proofs. Base commit: `5e292a8fb` (`bot/issue-442` with part 28's
gap fixes merged; every card of this batch still Rust there).

## 1. Ported to Luau (27)

Each `.luau` sits beside its `.rs`. The `.rs` keeps its `//!` header, `pub const ID`, the comment
above its tests and its `#[cfg(test)] mod tests`. Its `script()`, the helpers and constants only
`script()` used, and its `use` lines are gone. Every effect keeps the Rust order, and every
declared number is still a `ctx:param` read (R386). Where the Rust built one script for both
faces (`base.clone()`, or two calls of one builder with no per-face argument), the module says
`radiant = "base"`.

| Card | Id | Luau shape |
|---|---|---|
| M #10 Double Counting | `meditative-010` | `cryDeathExtra` (`J.HookArgs`) answering `param("extra")`; `radiant = "base"` |
| M #66 Fiery Waraxe | `meditative-066` | `base = {}`, `radiant = "base"` (keywords only) |
| M #70.1 Ready… I'm | `meditative-070-1` | `base = {}`, `radiant = "base"` (keywords only) |
| M #71.1 The Cane | `meditative-071-1` | `base = {}`, `radiant = "base"` (see below) |
| M #100 Greaser | `meditative-100` | `base = {}`, `radiant = "base"` (no text) |
| M #5 Death by 1000 cuts | `meditative-005` | `targets` (any Unit), `echoX` (`param("echo") * ctx:max_mana_of(ctx:controller())`, R802), `cry`: `damage` of `param("damage")`; `radiant = "base"` |
| M #6 Me no Likey | `meditative-006` | `targets` (an ally Unit), `cry`: `ctx:instance_of({ of = "chosen" })`, `cost_now × param("multiple")` read before the move, `give_control`, then a `for_each_card` whose `each_cards` pays only a Unit now among the opponent's active Units (R803), `data = { moved, gain }`; `radiant = "base"` |
| M #12 Fear Mongerer | `meditative-012` | `cry`: `trigger_turn_hooks({ times = param("times") })`; `radiant = "base"` |
| M #14 Prime Time | `meditative-014` | base `cry`: `for_each_card` with `data = { take = param("draws") }`; radiant `cry`: the same with no take; `each_cards` reads the library's prime-indexed ids (R804), `each_card` is `draw_from_library` |
| M #15 Smelly Steven | `meditative-015` | `cry`: `add_cost_rule` (enemy, Spells, `param("surcharge")`, `theirNextTurn`); `radiant = "base"` |
| M #19.1 Temporal Rift | `meditative-019-1` | `cry`: `take_extra_turn({ rift = true })`, then on `ctx:radiant()` `gain_mana`, `next_turn_mana`, `draw`; `radiant = "base"` |
| M #23 Golly Bob Howdy | `meditative-023` | `cry`: `for_each_card` over `ctx:adjacent_to({ of = "self" }, {})`; `each_card` transforms into `core-t-sheep` with `radiant` false / true per face |
| M #24 Polymorph | `meditative-024` | shared `targets` (any Unit); base `cry`: `transform` of the chosen; radiant `cry`: `for_each_card` over `adjacent_to({ of = "chosen" }, {})` (base Sheep), then the chosen |
| M #28 Shade-iris | `meditative-028` | `cry`: `shuffle_into` of `meditative-028-1`, `param("curses")`, enemy; `radiant = "base"` |
| M #31 The Conductor | `meditative-031` | base `cry`: `for_each_card` over `ctx:created_in_library(ctx:controller())`, `draw_from_library` each; radiant `cry`: `shuffle_random_from_catalog` (Prime, 1) first |
| M #32 Spiritually 中国 | `meditative-032` | `cry`: one `ctx:rng_roll(3)`, then `translate` (scope side any / enemy), `transform_random` (hand, same, CN, cost 0) or `param("units")` `summon_random`s; the Radiant rider left off the base face's args, as the Rust left the key off |
| M #33 First Day of 学校 | `meditative-033` | `cry`: `add_random_from_catalog` (CN, `param("cards")`, `costMod = -param("discount")`, `chinese`), `radiant` on the Radiant face only |
| M #34 高考 | `meditative-034` | `cry`: `destroy_all` (side any / enemy, both rows, not CN or KY), then `upgrade` (field scope, side any / self, CN or KY, `param("times")`) |
| M #35 RCTA (CN) | `meditative-035` | `targets` (non-CN unit or backrow, `aim = "help"`, `required`), `cry`: the chosen, fizzling if `ctx:tags_of` now holds CN; `grant_tag`, `translate`, `buff`, `upgrade`; `radiant = "base"` |
| M #36 CN Peptides | `meditative-036` | `targets` (any Unit, `required`), `cry`: the chosen, then `Rng::lucky` over a coin written out (below), `upgrade` or `destroy`; `radiant = "base"` |
| M #39.1 Auspicious Rock | `meditative-039-1` | `cry`: `lose_health` (self, `param("life")`), `add_rolled({ table = "auspiciousRock" })`; `radiant = "base"` |
| M #39.2 Jade | `meditative-039-2` | `cry`: `add_jade`, `gain_mana`; `radiant = "base"` |
| M #39.3 Dud | `meditative-039-3` | `targets` (any unit or hero), `cry`: `damage` of `param("damage")`; `radiant = "base"` |
| M #39.4 Red Jade | `meditative-039-4` | `cry`: `add_jade`; `radiant = "base"` |
| C+ #19 League of Losers | `classicplus-019` | `cry`: per lane 1–5 not `ctx:is_locked`, `summon` of its Loser (`radiant` false / true per face); after Mid Loser a `for_each_card` with `data = { lane }` whose `each_cards` answers the Mid Loser at that lane if `summoned_so_far` holds it (R411), `trigger_cry` each |
| C+ #19.4 Support Loser | `classicplus-019-4` | `endOfTurn`: `heal` of your hero, then `for_each_card` over `cards_in_scope({ side = "self" })` with `data = { amount }` (gap 2); `radiant = "base"` |
| T-AI-4 Chain of Thought | `classicplus-t-ai-04` | `cry`: `draw_while_cheap({ maxCost, repeats = J.config.CHAIN_OF_THOUGHT_REPEATS })` (gap 7), max cost 1 / 2 per face |

Four translations are not word for word. Each gives the same answers:

- **The Cane needs no surface.** The plan expected it to stay Rust as glue to
  `subsystems::pareto`. Its own Rust script was two `Script::default()`s. The forced attack and
  the return home are Pareto Optimality's (`meditative-071`), which this part does not touch.
  So the card is catalog data, and it ports as the other empty cards do.
- **CN Peptides' Lucky coin is `Rng::lucky` written out.** `ctx.rng.lucky(x, |rng| rng.coin(),
  better)` draws one coin and then `max(x, 0)` more, folding each into the best so far. `coin` is
  `next() < 0.5`, and `ctx:rng_chance(1, 2)` is `next() < 1/2`, the same single draw. So the
  loop `heads = heads or next` (a friendly Unit) / `heads and next` (an enemy one) draws exactly
  what the Rust draws. The Lucky X is `ctx:lucky_on(self.id) + ctx:luck_of(controller)`. The
  Rust reads `lucky_on` off the `self_` snapshot. The reader looks the id up live, which finds
  the Spell in its controller's resolving zone (R98). Book of Tokens (part 13) reads its Lucky the
  same way.
- **Prime Time parses the index as Rust's `parse::<u32>()`**: an optional `+`, then decimal
  digits only, at most `u32::MAX` (`string.match(index, "^%+?(%d+)$")`). Luau's `tonumber`
  alone would also take hex, exponents and spaces. The trial division uses `%` on positive
  operands, where Luau's floored remainder is Rust's. The base face stops reading the library
  once it has `take` matches. The Rust collected every match and then truncated, which keeps the
  same first `take`.
- **Spiritually 中国's roll is `ctx:rng_roll(3)`**: one `int(3)` draw plus one, so the branches
  are 1, 2 and the rest where the Rust matched 0, 1 and the rest.

## 2. Kept in Rust

None. All three held-back cards port onto part 28's fixes. League of Losers' Mid Loser
`cards` function answers `{}` when the Loser is not there (gap 1). Support Loser carries its heal
in `data` (gap 2). Chain of Thought reads `J.config.CHAIN_OF_THOUGHT_REPEATS` (gap 7).

## 3. The tests

No assertion changed: the diff removes no line with `assert` and adds none. The test edits are
imports and the scripts' source, made because `script()` and the module's `use` lines are gone:

- `script()` in a test is now `crate::scripts_of()[ID].clone()` (c005, c010, c012, c015, c023,
  c024, c039_3, C+ c019, C+ c019_4, and `super::super::ID` in T-AI-4). Greaser's two assertions
  call `script()` inline, so its tests module declares a `fn script()` reading the registry, and
  `use super::{ID, script}` narrows to `use super::ID`.
- Some tests reached prelude names through `use super::*`. They now import those names directly:
  `cast_new` (c031), `bounce`, `destroy`, `summon_copy` and `transform` (c035), `destroy` (c071_1)
  and `std::sync::Arc` (c039_3, C+ c019_4, for their `Arc::ptr_eq` checks, which still hold under
  `radiant = "base"`).
- `use super::*;` is dropped where it no longer brings anything in: c006, c014, c019_1, c028,
  c031, c032, c033, c034, c035, c036.
- Two tests used a constant the removed script declared. Each tests module now declares it, with
  its doc comment: `CURSE` (c028) and `EFFECTS` (c032, the roll its seed search predicts).

## 4. Proofs

All of these ran on this build, in this worktree, on linux x86_64 with 4 threads (AMD EPYC 7763).
The parent oracle is `jackioh` release-built from the base commit in this same worktree before
any edit, then copied aside (sha256 `ca459b64…`; this build's is `499a01c4…`).

- **Card tests.** The 27 cards' own tests: 216 passed, 0 failed. The full `jackioh-cards` suite
  (`cargo test -p jackioh-cards --features jackioh-engine/testkit`): lib 5860 passed, cross-card
  (`--test cards`) 752 passed, 0 failed. `cargo clippy -p jackioh-cards --all-targets -- -D
  warnings` is clean. `cargo jackioh spec check` (805 rulings, 1710 proofs, 27331 citations: ok),
  `catalog check` and `patches check` are clean too. `cargo fmt --check` passes on every file
  this part touches. The one diff it reports, `crates/luau/tests/api.rs:1101`, is already there at
  the base commit and belongs to part 28. This part must not touch `crates/luau`.
- **`luau diff`.** The 27 ids resolve to more forced cards than one deck holds, so the diff ran
  as two full 200-game runs (seeds 1–200), each card forced into both decks of its run:
  - `meditative-010,-066,-100,-005,-006,-012,-014,-015,-023,-024,-028,-031,-032`: recorded 200
    games (22,825 steps), 0 mismatches.
  - `meditative-033,-034,-035,-036,-070-1,-071-1,-019-1,-039-1,-039-2,-039,classicplus-019,
    classicplus-019-4,classicplus-043` (forced as 033, 034, 035, 036, 070, 071, 019, 039, 095,
    C+ 019 and C+ 043): recorded 200 games (18,166 steps), 0 mismatches.
  - Dud (`meditative-039-3`), Red Jade (`meditative-039-4`) and Chain of Thought
    (`classicplus-t-ai-04`) cannot be forced: no card's `refs` names them (R279). They play
    through the cards that make them, which the second run forces: Dushi Addict
    (`meditative-039`), whose Auspicious Rock rolls Dud, Jade or Red Jade, and AI Slop
    (`classicplus-043`, the AI pool), as part 13 forced it for the other AI tokens.
- **`golden check`**: 240 of 240 games replayed identically (22,713 steps), 0 diverged.
  **`fuzz --seeds 200`**: 200 passed, 0 failed (0 panics, 0 replay mismatches, 0 invariant
  violations).
- **`luau bench --parent-bin`**, within every budget:

  | Number | Parent | This build | Ratio | Budget |
  |---|---|---|---|---|
  | `fuzz --seeds 200` wall | 17,813.373 ms | 17,824.134 ms | ×1.00 | ≤ 2× |
  | Slowest `gate` decision (scaled) | 114.692 ms | 116.290 ms | ×1.01 | ≤ 1.5× and < 1500 ms |
  | Golden step | 2.367 ms | 2.321 ms | ×0.98 | ≤ 2× |

- **Strict analysis was not run.** This machine has no `luau-analyze`, the vendored Luau sources
  (`luau0-src`) carry no Analysis library to build one from, and fetching the 0.740 release was
  not permitted in this run. The files follow the strict idioms of the landed batches: annotated
  contexts, hoisted `J.TargetSpec` and `J.ScopeSide` locals for union-typed arguments, and
  `boolean?` for a key left off.

## 5. Decisions the plan did not settle

- **The Cane is ported, not kept** (section 1). The plan's risk named it as glue to
  `subsystems::pareto`, but that glue is Pareto Optimality's. The Cane's own script binds nothing.
  Rejected: keeping a Rust file with an empty `script()`, which `build.rs` would allow but which
  PORTING.md section 5 gives no reason for.
- **The five data-only cards keep `local J = require("@jackioh")`**, as PORTING.md section 2
  says every file starts, and as batch 1's empty cards do. Rejected: dropping the line, which
  some later batches did to avoid luau-analyze's `ImportUnused` note.
- **Test constants live in the tests module** (`CURSE`, `EFFECTS`). Rejected: keeping them above
  the tests under `#[cfg(test)]`, which would put a test-only item in the card's script part.
- **Greaser's tests get a local `fn script()`**, so its two assertion lines stay byte for byte.
  Rejected: rewriting each assertion to read the registry inline.
- **`luau diff` ran as two runs of 13 ids** (part 9 split 11 + 11). Double Counting runs with
  the Cry Units it multiplies. Dud, Red Jade and Chain of Thought are proved through the cards
  that make them, since the tool refuses a token no card's `refs` names.
