# v0.4.0 Part 14: cards to Luau, batch 6 (start-of-turn, end-of-turn and Death hooks)

Notes for issue #456 (v0.4.0 part 14 of 29), built by `docs/v0.4.0/PORTING.md`. Of the batch's 25
cards, 20 run from Luau now and 5 stay Rust (section 2). Every test in the 25 files is unchanged
except where a test called `script()` (section 3).

## 1. Cards ported

Each has a new `crates/cards/src/scripts/<set>/<module>.luau`. Its `.rs` lost `script()`, the
helpers and constants only `script()` used, and the `use` lines. It keeps its header comment,
`pub const ID` and `mod tests`.

| Card | Hooks | Notes |
|---|---|---|
| `core-009` Moths to the Flame | `startOfTurn` | `radiant = "base"` (Armor 1 is printed) |
| `core-013` Jlockeed Shredder-10 | `endOfTurn` | `ctx:param("damage")` |
| `core-019` Midrange Menace | `endOfTurn` | heal `toFull`, hoisted `J.TargetSpec` |
| `core-022` Carnivorous Cube | `targets`, `cry`, `death` | `memory.eaten` keeps the Rust key set; radiant's per-zone summons count `#ctx:slots_of(me, "units")` (the engine's `UNIT_ZONES`) |
| `core-059` Unbiased Immigration | `startOfTurn` | `ctx:embiggened()` gates `costOverride` |
| `core-081` Radiant Saintess | `death` | units, then (radiant) the hand, one `set_radiant` each |
| `classic-051` Back Breaker | `death` | |
| `classicplus-007` The House | `cry`, `startOfTurn` | `ctx:rng_chance(2, 3)` is the Rust `chance(2.0 / 3.0)` draw exactly |
| `classicplus-012` The Mother Pancake | `endOfTurn` | |
| `classicplus-013` Mommy Barker | `death` | |
| `classicplus-021` Whirlwind | `cry`; radiant `cry`, `endOfTurn` | |
| `classicplus-028` Nuestro hogar, nuestras tumbas | `death` | |
| `classicplus-038` Solarius | `death` | base passes `radiant = false`, as the Rust did |
| `classicplus-042-1` KY's Gift | `startOfTurn` | |
| `classicplus-049` Jay Fungus | `endOfTurn` | |
| `classicplus-061` Bauble Bubble | `death` | |
| `classicplus-063` Fruit Tree | `startOfTurn` | |
| `classicplus-076` Brother Lar | `death` | `crate::card_def("classicplus-076-1").id` is the id itself, so the gap `api-gaps.md` lists for this card (`card_def`) does not apply |
| `classicplus-078` Claude's Datacenter | `endOfTurn` | |
| `classicplus-t-ai-10` Fine-Tuning | `endOfTurn` | |

A reader with no answer (`live_self`, `first_free_zone`, `instance_of`, `recalled`, `self_`)
hands Luau mlua's `null`, a light userdata that is truthy and not `nil`, because `reads.rs`
serializes `None` with mlua's default options. The ports therefore test such a value with
`type(x) == "table"`, never `x ~= nil` or a bare `if x`.

## 2. Cards that stay Rust

Each has its `.rs` exactly as at the base commit and no `.luau`.

- `classicplus-019-2` Jungle Loser. Missing API (`api-gaps.md`): `with_kill_credit`'s `pairs` and
  `then` are not bound.
- `classicplus-019-4` Support Loser. Missing API: its `for_each_card` `each` closure captures the
  heal amount the hook read (`param("heal")`). A Luau `each_card` is called with the id alone (no
  `ctx`, no data), so it cannot know the amount. The rewrite I rejected was to read the units at
  hook time and return one `heal` per unit. That moves the read from after the hero's heal to
  before it and replaces the lazy part with plain effects, so it would not be the same card.
  Needs a `data` field for `for_each_card`, as `after_state_check` has.
- `core-086` "Miss" Mrow. Missing API: `ctx:killer_of(id)` resolves the id to the live instance. In
  a Death hook that instance is the R78-reset graveyard card, which has no `last_damaged_by`. Tried:
  a port on `ctx:killer_of(ctx:self_().id)` failed 4 of the card's 10 tests (every case where a Unit
  killed it). Recomputing R42/R361's killer from `lastDamagedBy` in the script would copy an engine
  rule into a card. Needs a `killer_of` that reads the hook's own snapshot (`ctx.self_`).
- `core-023` Reoccurring Dream. Missing API: its roll adds `given_lucky_on(live_self)` (R1438), which
  is not bound. `ctx:lucky_on` also counts the printed Lucky as tuned, which D14 keeps out of this
  roll. `luau surface` does not list `given_lucky_on` for this card, because the Rust passes it as
  a function value (`map_or(0, given_lucky_on)`), so `api-gaps.md` misses it too. The rest of the
  card (Lucky as `1 + max(x, 0)` `rng_chance(3, 10)` draws with no short circuit, the R68 return)
  ports with bound readers.
- `classicplus-070` Chaos Machine. The divergence can't be fixed from the card. Its own test
  `is_a_2_field_spell_whose_two_hooks_run_one_tick_on_both_faces` asserts
  `Arc::ptr_eq(start_of_turn, end_of_turn)`. `load_card` builds one adapter per hook, so two hooks
  are never one `Arc`, even when they name one Luau function. L16 keeps the assertion. Porting it
  needs the loader to share an adapter, or a person to relax that assertion.

## 3. Test edits

None of the 228 tests in the 20 ported files changed an assertion. Where a test called `script()`,
it now reads the registry: `crate::scripts_of()[ID].clone()` (`super::ID` or `super::super::ID` where
the test sits deeper). Tests that took the parent's `use jackioh_engine::prelude::*` through
`use super::*` lost it with the parent's `use` lines:
- three `use super::*;` lines that now import nothing are removed (Saintess, Mother Pancake, Mommy
  Barker);
- Jay Fungus's tests import `std::sync::Arc`;
- the tests of Cube, The House and Nuestro hogar import `super::ID` where they imported `script`.

## 4. Results

The parent is `jackioh` built in release from the base commit `c65d6d55c`, with every card still
Rust, in this worktree before any change, then copied to `/tmp`. PORTING step 1 builds a second
worktree instead; this is the same commit and needs nothing removed afterwards.

- `cargo test -p jackioh-cards --lib -- <the 20 modules>`: 228 passed, 0 failed. The base commit
  has 228 `#[test]`s in those files.
- `cargo jackioh luau diff --parent-bin <parent> --cards core-009,core-013,core-019,core-022,core-059,core-081,classic-051,classicplus-007,classicplus-012,classicplus-013,classicplus-021,classicplus-028,classicplus-038,classicplus-042,classicplus-049,classicplus-061,classicplus-063,classicplus-076,classicplus-078`:
  200 games (seeds 1–200, 25,932 steps), `golden check: 200 of 200 games replayed identically`,
  `luau diff: 0 mismatches`.
  - The two tokens can't be forced: no card's `refs` name `classicplus-042-1` or
    `classicplus-t-ai-10`, and the tool refuses both. KY's Gift is forced through
    `classicplus-042` KY's Test, the card that makes it (still Rust, L14). Fine-Tuning comes from
    the AI pool that Claude's Datacenter (forced) draws from.
  - The trace format holds hashes, so it doesn't show whether either token came into play in
    those games. Their own unchanged tests are what proves them.
- `cargo jackioh golden check`: 240 of 240 games replayed identically (22,713 steps).
- `cargo jackioh fuzz --seeds 200`: 0 panics, 0 over the bound or stalled, 0 replay mismatches, 0
  invariant violations; 200 hero deaths.
- `cargo jackioh luau bench --parent-bin <parent>`, on linux x86_64 with 4 threads (Intel Xeon
  6973P-C): "within every budget".

  | Number | Parent | This build | Ratio |
  |---|---|---|---|
  | `fuzz --seeds 200` | 7,852.0 ms | 7,941.2 ms | ×1.01 |
  | Slowest decision | 227.0 ms | 229.4 ms | ×1.01 |
  | Golden step | 1.340 ms | 1.351 ms | ×1.01 |

- `luau-analyze --mode=strict`: **not run**. This box has no Luau toolchain: `luau0-src` ships no
  `Analysis` sources to build it from, and nothing could be downloaded. The new files follow the
  conventions that kept the pilots and fixtures strict-clean: `--!strict`, annotated contexts, and
  `J.TargetSpec` hoisted into an annotated local wherever it is a whole argument or sits inside the
  union `HealArgs`. The first run with the toolchain should check them.

## 5. Decisions the plan did not settle

- **Tokens in `luau diff`**: forced through the card that makes them, as the tool's own error asks,
  rather than left out of the run.
- **Tests read the registry, not a shim.** Rejected: a `#[cfg(test)] fn script()` in each `.rs` that
  wraps the registry. It would leave every test byte-identical, but it would put a `script` back in a
  Luau card's file, which `build.rs`'s one-source rule exists to prevent.
- **No new helper in `lib.rs`.** Every batch edits these tests at the same time, and one shared
  helper would conflict on merge. `scripts_of()[ID]` already exists.
- **`api-gaps.md` is not edited here**, so the parallel batches don't conflict on it. The gaps in
  section 2 are for the reconcile to fold in.
