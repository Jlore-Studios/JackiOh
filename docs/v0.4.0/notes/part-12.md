# v0.4.0 Part 12: cards to Luau, batch 4 (Classic+ Cry cards, first half)

Notes for issue #454 (v0.4.0 part 12 of 29): 26 Classic+ cards and tokens, Cry and declared
`targets` only, ported by `docs/v0.4.0/PORTING.md`. 24 run from Luau now; 2 stay Rust.

## 1. What changed

Each ported card has a `.luau` beside its `.rs`
(`crates/cards/src/scripts/classic_plus/<file>.luau`); the `.rs` keeps its header, `pub const ID`
and its tests, and lost `script()`, its helpers and its `use` lines.

Ported (24): C+ #4 Juhan Biggest Bat, #5 Guy Att, #6 Wrong-House Attacker, #8 Withering Storm,
#9 Silence, #10 New Wraps, #12.1 Devour, #12.2 Death Boil, #12.3 Fluffy Grip, #12.4 Powder Spray,
#12.7 Legion of the Hungry, #14 Forever&, #15, #16 and #17 Conjure Rush Token(+/++), #19.3 Mid
Loser, #20 Mushroom Power, #24 Crushing Walls, #25 Soul Shot, #27 Zephrys Zealotism, #32.1
Execute, #32.2 Brawl, #32.3 Blade Storm, #32 Otherworldly Removal.

Stayed Rust (2), each `.rs` as at the base commit:

- **C+ #23 Dropshipping** (reason 1, a missing API function): its Brittle goes to "the cards this
  list put in its controller's hand", read off the list's own `AddedToHand` events
  (`ctx.events` from `ctx.events_from`, R136). No reader hands a script the list's events
  (`summoned_so_far` is the only one built on them, and it reads `Summoned`). Its `each` also
  carries the Brittle count read as the Cry runs, which a Luau `each` cannot (section 3).
- **C+ #19 League of Losers** (reason 2, a divergence the batch cannot fix): Mid Loser's Cry is
  triggered through a `for_each_card` whose `cards` reads, after the summons, whether this list
  put a Mid Loser into lane 3. When it did not (lane 3 taken or reserved, a full board), that
  function answers an empty list, and a Luau `cards` function cannot (section 3): the hook panics.
  The hook cannot tell in advance whether the summon will land, so the empty case cannot be
  handed over outright either. The Luau was written, failed `a_full_board_summons_nothing` and
  `r411_with_mid_loser_s_zone_taken_there_is_no_mid_loser_and_no_cry` with "each_cards: a cards
  function answers ids, not a record", and was removed.

Tests: no assertion changed. The tests that called `script()` (17, one in each of 17 files) now read
the card's entry in the registry, `crate::scripts_of()[ID].clone()`, and `use super::{ID,
script}` became `use super::ID`. Three test modules (#19.3, #20, #25) used `Arc` through the
parent's `use jackioh_engine::prelude::*`, which went with `script()`, so they import
`std::sync::Arc` themselves; #14's top test module lost a `use super::*` that no longer imports
anything (it failed `-D warnings` as an unused import).

## 2. How each card was made equal

The Luau keeps each Rust effect list in its order. Where the Rust had one script for both faces
(`base.clone()`), the module says `radiant = "base"`, so the tests' `Arc::ptr_eq` checks still
hold; where it built two, the module has two face tables.

- Readers that answer an `Option` through serde (`live_self`, `slot_of`, `instance_of`,
  `resolve_target`, `card_at`, `recalled`, …) answer mlua's `null` (a light userdata, truthy in
  Luau) for `None`, not `nil`. The scripts test presence with `type(x) == "table"` (or
  `== "string"` for `recalled`), never truthiness.
- **#19.3 Mid Loser**: `Rng::lucky_coin(x)` draws `x + 1` coins with no short cut, each
  `next() < 0.5`; the Luau draws `ctx:rng_chance(1, 2)` (`next() < 1/2`) the same number of times.
  Lucky is read as `ctx:numbered_sum(me.id, "Lucky")`, the same `unit_view` keywords Rust sums.
- **#25 Soul Shot**: `Rng::lucky` (one roll, then X more, each kept against the best by R414's
  comparator) with `ctx:rng_pick`, which draws `int(len)` as `Rng::pick` does; Lucky is summed off
  `ctx:card_keywords(me.id)`, as Rust sums `card_keywords`.
- **#8 Withering Storm**: the picks are read as the list reaches them, in a `cards` function, as
  in Rust: a list that resumes after a pause re-enters its hook (`work.rs`), and the part's memo
  keeps the ids, so the shuffle stays out of the hook, where a re-entry would draw it again. It
  draws only when there are more changeable cards than the count, as in Rust.
- **#12.7 Legion of the Hungry**: Rust drew the picks in the hook and handed fixed lists to its two
  `for_each_card`s; the Luau does the same, passing the ids outright (`cards = { … }`).
- **#20 Mushroom Power**: Rust read `buff` in the hook and captured it in `for_each_card`'s
  `each`. A Luau `each` is handed only the id (section 3), so the Cry returns one `buff` per
  neighbour itself, the neighbours read in the hook. `for_each_card` was the list's only effect, so
  the neighbours it read when the list reached it are the ones the hook reads; a buff opens no
  prompt, so nothing re-runs the hook between them.

`for_each_card` lists that may be empty (section 3's first gap) follow two patterns, each equal to
the Rust it replaces because nothing runs between the hook and the list's first effect:

- the list's only effect, with no random draw: the ids are read in the hook and passed outright
  (#24 Crushing Walls, #32.1 Execute's Radiant face, #32.2 Brawl's Radiant face);
- otherwise the `cards` function stays lazy, as in Rust, and the hook passes `cards = {}` only in
  the case it can already see is empty: #8 an empty deck (`cards` is at least 1, so any other deck
  picks at least one card), #25 no enemy Unit a Spell may affect (no draw then, R129), #32.2's base
  face one Unit or none (the survivor drawn is the only one, so nothing is destroyed; the
  `remember_random` draw still happens first, as in Rust).

## 3. API gaps found (not fixed here: `crates/luau` is not this part's)

1. **A `cards` function cannot answer an empty list.** `numbers::walk_table` turns an empty table
   into `{}`, and `api/each.rs` refuses anything but an array from a `cards` function ("a cards
   function answers ids, not a record"), so the hook panics. Literal `cards = {}` in the verb's
   arguments works (`api::typed` retries with empty objects as arrays). The fix belongs in the
   host: read an empty record from a `cards` function as no ids. League of Losers waits on it.
2. **`for_each_card`'s `each` is handed only the id**, so a per-card effect cannot carry a value
   the hook read (a declared number). Mushroom Power works around it (section 2); Dropshipping
   needs it.
3. **No reader for the list's own events** (`ctx.events[ctx.events_from..]`, R136), beyond
   `summoned_so_far`. Dropshipping waits on it (its `AddedToHand` events).

## 4. Proofs

All on this box (linux x86_64, 4 threads, AMD EPYC 9V74), parent = `jackioh` built from the base
commit `c65d6d55c` (`git worktree add --detach ../base-454 HEAD`, removed afterwards).

- `cargo test -p jackioh-cards --lib -- classic_plus::<stem>::` for all 26 file stems: 271 passed.
- `cargo test -p jackioh-cards`: 5860 unit tests and 752 cross-card tests (`--test cards`) passed.
- `jackioh luau diff --parent-bin ../base-454/target/release/jackioh --cards <ids>`, the ids being
  the 26 cards with The Mother Pancake (`classicplus-012`) forced in place of its five Pancake
  tokens, which no card's `refs` name (the tool refuses them: "force the card that makes it"):
  "recorded 200 games (seeds 1–200, 23640 steps)", "golden check: 200 of 200 games replayed
  identically (23640 steps), 0 diverged", "luau diff: 0 mismatches". The same games recorded with
  `golden record` show each forced deckable card played 287 to 509 times, in 172 to 186 of the
  200 games (plays of tokens and created cards are not broken down by card).
- `jackioh golden check`: 240 of 240 games replayed identically (22713 steps), 0 diverged.
- `jackioh fuzz --seeds 200`: 200 passed, 0 failed, 0 panics, 0 replay mismatches, 0 invariant
  violations.
- `jackioh luau bench --parent-bin ../base-454/target/release/jackioh`: within every budget.

  | Number | Parent | This build | Ratio |
  |---|---|---|---|
  | `fuzz --seeds 200` wall time | 11,235.078 ms | 11,378.586 ms | ×1.01 |
  | Slowest `gate` decision (scaled) | 239.808 ms | 237.984 ms | ×0.99 |
  | Golden step | 1.913 ms | 1.926 ms | ×1.01 |

- `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `jackioh spec
  check`, `catalog check` and `patches check`: clean.
- `cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts`: ai, cards,
  engine (rules 2637, golden 19) and luau green, `apps/web/src/wire/` unchanged. The server's
  `api::bots::r640000_a_bot_in_mythic_shows_mythic_grape_while_jlorious_has_room` failed once in
  that run (a leaderboard order) and passed alone and in a second full server run (1268 of 1268).
  `jackioh-tools` has three failures that are not this part's: `luau::surface::tests::luau_core_002_bigot`
  and `luau_reports_print_as_markdown_and_json` (part 7's notes record both, on Core #2's port), and
  `acceptance::v2_the_pure_crates_link_only_serde_serde_json_indexmap_and_each_other`, which names
  `crates/cards/Cargo.toml`'s `jackioh-luau` dependency (part 4's, untouched here). Left for the
  reconcile.
- `luau-analyze --mode=strict` was **not run**: there is no Luau toolchain on this box and the
  network is closed to it, so the strict gate is the reconcile's. The files follow PORTING.md's
  conventions (annotated contexts, union-typed arguments hoisted into annotated locals:
  `J.TargetSpec` for `heal`, `J.Keyword`, `J.Enchantment`, `J.CardsSource`); the build's own lint
  and compile pass over every file.
