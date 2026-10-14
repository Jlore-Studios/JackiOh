# v0.4.0 Part 27: cards to Luau, batch 19 (Meditative Activate abilities, replacements and target checks)

Notes for issue #766 (v0.4.0 part 27 of 29, a fullsend part of #442), built by
`docs/v0.4.0/PORTING.md`. The batch has 24 cards: the plan's 14 Meditative cards and the 10 that
earlier batches held back (#442, L27). 17 run from Luau now and 7 stay Rust (section 3). First, the
two fixes the plan's §1c asks for (section 1). Base commit: `4af6da128` (`bot/issue-442` after #763
landed, part 28's API fixes in, every card of this batch still Rust).

## 1. The two fixes on the tree

- **Felinor Fuser (`classicplus-030`) is Rust again.** Batch 11's port (03771fb) diverges from the
  Rust script: its Cry's first Discover carries `data = { picks = {} }`, and an empty Luau table in
  an untyped `data` crosses into Rust as the JSON object `{}` where the Rust stored `[]`
  (`crates/luau/src/numbers.rs`, `walk_table`, coerces only typed arguments). The `.rs` is the one
  at `5e292a8` (`git checkout 5e292a8 -- …/c030_felinor_fuser.rs`) and the `.luau` is deleted.
  Marking a Luau table as a list for an untyped value is an API change in `crates/luau`, which is
  the reconcile's. With it, `cargo jackioh golden check` on the base replays 240 of 240 games
  (22,713 steps), 0 diverged.
- **`core/c078_fullsend.rs`** loses the `use super::*;` its tests no longer used, and nothing else.
  `cargo clippy -p jackioh-cards --all-targets -- -D warnings` is clean on the base with both fixes.

## 2. Ported to Luau

Each `.luau` sits beside its `.rs`. The `.rs` keeps its `//!` header, `pub const ID`, the comment
above its tests and its `#[cfg(test)] mod tests`. Its `script()`, the helpers and constants only
`script()` used, and its `use` lines are gone. Every effect keeps the Rust order. Every activation,
replacement and trigger id, label, `uses` count and target declaration is the Rust one, since
`legal_actions` and a replayed log name them, and every declared number is still a `ctx:param`
read (R386).

| Card | Id | What it declares |
|---|---|---|
| M #18 Expedition12 | `meditative-018` | `staticFlags.quickdraw` (Radiant `echo = 1`), a hand pick with `check = "notRadiant"` and its `targetChecks`, `cry`: `lose_refreshes` then `set_radiant` |
| M #49.2 Yile's Virus | `meditative-049-2` | `neverDefense`; Activate "destroy"; `death`: `discard_random`; `startOfTurn`: a `for_each_card` over `adjacent_to` self minus its own def, the amount in `data` (gap 2), then the hero; `radiant = "base"` |
| M #53 Prestige | `meditative-053` | base: a pick with `check = "radiant"` (`card_type_of`, `faceUp`, `revealed`) and `clear_radiant({})`; radiant: one `clear_radiant` per enemy Radiant permanent, hand and library card; both `add_random_from_catalog` of the AI tag |
| M #69 The Maestro | `meditative-069` | a `manaSpent` trigger, one `place_plague` per mana its controller spent; Activate "maestro-exile" with the Rust labels, `canActivate`: `plague_on(self) >= param("price")`, `run`: `consume_plague` then four `exile_random_from` |
| M #75 Ever Growing Tree | `meditative-075` | Activate "grow": `fuse_random_into` self, a Field Spell, Radiant on the Radiant face |
| M #80 Aluneth | `meditative-080` | `quickdraw`; `endOfTurn`: draw 3; radiant adds Activate "exile" |
| M #92 Unan | `meditative-092` | one replacement, data only: `lethalHit`, `instead = { redirect = "self" }`; `radiant = "base"` |
| M #97.2 Wishing Well | `meditative-097-2` | Activate "wish": `ctx:rng_chance(chance, 100)` once plus once per Lucky (`numbered_keywords_on`), every roll drawn, then a Radiant `add_random_from_catalog`; `radiant = "base"` |
| M #97.3 School | `meditative-097-3` | Activate "class": `summon_random` of a Unit at `param("cost")`, Radiant on the Radiant face |
| M #97.4 Mega Church | `meditative-097-4` | `staticFlags` `tribute = 5`, `tributeCheap = 1`; Activate "church-steal" with an enemy unit-or-backrow target: `steal`, then `set_radiant` on the Radiant face |
| M #97.6 University | `meditative-097-6` | Activate "study": `upgrade` of your field `param("times")` times; `radiant = "base"` |
| M #97.9 Jlockheed's Headquarters | `meditative-097-9` | `tribute = 5`; Activate "fill" (uses 1, Radiant 2): one `summon_random` of a Jlockeed Unit per `fill_board_zones` lane |
| M #101 Gachaholic | `meditative-101` | base `cry`, radiant Activate "pull": `add_random_from_catalog({ query = { luckBased = true }, lucky = param("lucky") })` |
| C+ #70 Chaos Machine | `classicplus-070` | one `tick` under `startOfTurn` and `endOfTurn`, so one adapter (gap 10): `upgrade` then `degrade`, `random = param("cards")`; `radiant = "base"` |
| C+ #47 Jogg's Box | `classicplus-047` | one `cry` on both faces, so one adapter (gap 14): `cast_random` of `param("casts")` Spells; radiant `echo = 1` |
| Core #46 Suppressive Aura | `core-046` | two generic auras (gap 8) with named predicates, section 6 |
| Core #58 Rush Token Farm | `core-058` | one `startOfTurn` on both faces (`summon` of the Rush Token); radiant a generic aura with a named predicate, section 6 |

The tests that assert one hook object (`Arc::ptr_eq`) pass because the Luau names one function
twice: Chaos Machine's start and end of turn across both faces, and Jogg's Box's two Cries.

## 3. Kept in Rust

Each keeps its `.rs` exactly as the base commit has it and has no `.luau`.

| Card | Id | Why |
|---|---|---|
| M #54 Money Machine | `meditative-054` | PORTING §5 reason 1. Its end of turn rolls the card it shuffles over `MONEY_MACHINE_ODDS` with `roll_weighted`, which `api-gaps.md` lists as deliberately unbound, and the odds are a `&[RollOdds]` table `J.config` does not carry (it holds `i32`s only). Spelling the roll in Luau would mean writing the odds into the script, restating an engine constant (CLAUDE.md rule 9) |
| C #82 Sheeople | `classic-082` | PORTING §5 reason 1, `crate::card_def` (`api-gaps.md`, still a true gap). Its `staticFlags.tributeWorth` is the catalog's `worth` param, base 2 and Radiant 3, read off the compiled catalog when the script is built. `tribute_value_of` (`play_choices.rs`) reads the flag's number wherever the card's def declares no `worth`, and a fused card's flags take the larger worth (`subsystems/fuse.rs`), so the number matters. A Luau face's `staticFlags` is data read at load with no context. `J.query({ defId = ID })` at module level reads the *registered* catalog, which is empty until `register_all()` runs, and `scripts_of()` is called without it (`tests/cross/registry.rs`, `has_no_duplicate_def_id`), so the load would fail there. A literal 2 and 3 would restate the catalog and drift from it under a tune. The shared Death adapter it waited for (gap 14) is not the obstacle any more |
| Core #14 Jlockeed's Weapons | `core-014` | PORTING §5 reason 3: it breaks `luau bench` together with the rest of the batch (section 5). Alone it takes `fuzz --seeds 200` to ×1.75 of the parent's wall time; on top of the 17 ports, to ×2.14 |
| Core #65.1 Spikey Pillow | `core-065-1` | PORTING §5 reason 3, the same: ×1.75 alone, ×2.04 on top of the 17 ports |
| C+ #12.5 Anti-Waffle Shell | `classicplus-012-5` | PORTING §5 reason 1. Its aura applies to the ids `active_units_of` answers as it is asked: the tops of the unit piles on its controller's side and the Units its carriers hold (R446), which stand in the **backrow** row. The generic aura's data filter (gap 8) has side, rows and def ids: `rows = { "units" }` misses a carried Unit, and adding `"backrow"` takes every backrow card with it. A named `aura_filter` is handed the candidate alone, with no state, so it cannot tell a carried Unit from a set one either |
| C+ #46 Felinor Flagbearer | `classicplus-046` | PORTING §5 reason 1. The base aura is "your **other** Felinors": it turns the source instance away (`unit.id != self_.id`) and reads `tags_of(state, unit)` for a granted Felinor tag (R923). The data filter has no instance or tag field, and a named `aura_filter` gets the candidate alone, so neither the source's id nor the state reaches it. The Radiant face (itself included) could be said, but a card is one script source (`build.rs`) |
| C+ #46.1 Felinor Flagbearer Prime | `classicplus-046-1` | the same "other Felinors" aura, on both faces |

These belong in `docs/v0.4.0/api-gaps.md`: a weighted-roll data shape (or `MONEY_MACHINE_ODDS` in
`J.config`) for Money Machine; a catalog-def reader that does not depend on the registered catalog
(or `staticFlags` read from a context) for Sheeople; and, for the three auras, data handed to a
named `aura_filter` (the source, as `after_state_check` hands its hook `data`) or an instance and
tag field on the filter. A cheaper Luau aura (section 5) would let Weapons and Spikey Pillow port.
This part does not edit `api-gaps.md` or `crates/luau`: the plan names only the card files and
these notes, and the batches build side by side. #442's reconcile folds them in.

## 4. The tests

No test asserts anything new or anything less: no added or removed line in a ported file holds
`assert`, the seven Rust cards' files are untouched (Felinor Fuser's is the `5e292a8` file, section
1), and the 17 ported files hold 188 `#[test]`s, as at the base. These lines changed, all because
`script()`, its constants and the module's `use` lines are gone:

- `script()` in a test is now `crate::scripts_of()[ID].clone()`, the registry `build.rs` generates
  (Expedition12, Aluneth, Wishing Well, School, Mega Church, University, Headquarters, Gachaholic,
  Chaos Machine, Jogg's Box, Suppressive Aura).
- `use super::{ID, script};` is `use super::ID;`, and so are Aluneth's and Headquarters' longer
  imports. The constants their tests named moved into the test module with their doc lines:
  `DRAWS` (Aluneth), `BASE_USES`, `RADIANT_USES` and `TRIBUTE_COST` (Headquarters).
- Chaos Machine's and Jogg's Box's tests import `std::sync::Arc` for their `Arc::ptr_eq` checks,
  which reached them through the removed `use jackioh_engine::prelude::*;`, as part 21 did.
- Prestige's, The Maestro's and Ever Growing Tree's top-level `use super::*;` imported nothing
  their tests use any more, so it is removed, as parts 14, 18 and 24 removed such lines.

## 5. Proofs

All run on this build, in this worktree, on linux x86_64, 4 threads (Intel Xeon Platinum 8573C).
The parent `jackioh` was release-built in this worktree from the base commit with section 1's two
fixes, before any card file changed, and copied outside it.

- **Card tests.** `cargo test -p jackioh-cards --lib --` over the 19 files first ported plus Felinor
  Fuser and /fullsend: 236 passed, 0 failed. With Weapons and Spikey Pillow back in Rust, the whole
  crate, `cargo test -p jackioh-cards`: unit tests 5,860 passed (the 188 of the 17 ported files
  among them), the cross-card tests (`tests/cards.rs`) 752 passed, 0 failed.
- **`luau diff`**, against the parent, 200 games each (seeds 1–200). The ids do not fit one
  20-card deck, so three runs:
  - `meditative-018, -049-2, -053, -069, -075, -080, -092, -097-2, -097-3, -097-4` (forced as 018,
    049, 053, 069, 075, 080, 092 and 097: the tokens go in through YileGPT Tamed and Jlockheed's
    Evil Blueprints, R279): 15,022 steps, **0 mismatches**;
  - `meditative-097-6, -097-9, -101, classicplus-070, classicplus-047, core-046, core-058`:
    19,606 steps, **0 mismatches**;
  - the Rust cards with ported ones they meet in play (`meditative-054, classic-082,
    classicplus-046, classicplus-046-1, core-014, core-065-1, classicplus-030, meditative-097-4,
    meditative-092, core-046, meditative-075`): 22,536 steps, **0 mismatches**. Anti-Waffle Shell
    cannot be forced: no deckable card's `refs` names it.
- **What those games ran.** The first two runs re-recorded with `golden record --seeds 1..200` and
  the same forced lists, then folded by a throwaway counter outside the repository (activations
  and resolutions by the face of the card, and `redirected` events whose `toId` is Unan):

  | Card | Activations base / radiant | Resolved base / radiant | Other |
  |---|---|---|---|
  | Expedition12 | | 362 / 5 | |
  | Yile's Virus | 114 / 5 | | |
  | Prestige | | 236 / 15 | |
  | The Maestro | 73 / 11 | 115 / 9 | |
  | Ever Growing Tree | 154 / 9 | 170 / 9 | |
  | Aluneth | 0 / 28 | 240 / 28 | |
  | Unan | | 153 / 9 | 351 / 14 hits redirected to it |
  | Wishing Well | 34 / 7 | | |
  | School | 57 / 7 | | |
  | Mega Church | 26 / 4 | | |
  | University | 118 / 1 | | |
  | Jlockheed's Headquarters | 22 / 0 | | the Radiant face never came up; its tests cover Activate 2 |
  | Gachaholic | 0 / 18 | 299 / 7 | |
  | Chaos Machine | | 231 / 9 | 19,054 / 818 steps on the field |
  | Jogg's Box | | 151 / 9 | |
  | Suppressive Aura | | 268 / 9 | 22,110 / 692 steps on the field |
  | Rush Token Farm | | 253 / 8 | 22,474 / 822 steps on the field |

- **`golden check`**: 240 of 240 games replayed identically (22,713 steps), 0 diverged.
- **`fuzz --seeds 200`**: 200 passed, 0 failed; 0 panics, 0 over the bound or stalled, 0 replay
  mismatches, 0 invariant violations; 200 hero deaths.
- **`luau bench --parent-bin`.** The first run, with all 19 first ported (Weapons and Spikey Pillow
  among them), was **over budget**:

  | Number | Parent | 19 ported | Ratio | Budget |
  |---|---|---|---|---|
  | `fuzz --seeds 200` wall time | 20,378.139 ms | 52,070.786 ms | ×2.56 | ≤ 2× |
  | Slowest `gate` decision | 103.716 ms | 14.374 ms | ×0.14 | ≤ 1.5× and < 1,500 ms |
  | Golden step | 2.555 ms | 4.671 ms | ×1.83 | ≤ 2× |

  The cost is the auras: a Luau aura hook runs its card's chunk afresh inside every stat read of
  every unit (L8), and a named predicate adds a second call and a serialised `CardInstance` per
  candidate. To find which, the fuzz number alone, `fuzz --seeds 200` wall time, was timed on one
  binary per aura with only that aura ported (all other 15 ports in), against the parent's 19.9 s
  to 20.5 s in the same sessions:

  | Build | `fuzz --seeds 200` | Ratio |
  |---|---|---|
  | the 15 ports, no aura | 21.4 s, 21.1 s | ×1.05 |
  | + Jlockeed's Weapons | 35.3 s | ×1.75 |
  | + Suppressive Aura | 26.7 s | ×1.32 |
  | + Rush Token Farm | 21.6 s | ×1.07 |
  | + Spikey Pillow | 35.4 s | ×1.75 |

  Taken cheapest first, Rush Token Farm and Suppressive Aura fit together. Then each of the other
  two on top of them, timed in one session against the parent's 20.2 s and 21.1 s:

  | Build | `fuzz --seeds 200` | Ratio |
  |---|---|---|
  | the 17 ports (the final build) | 26.4 s | ×1.28 |
  | + Jlockeed's Weapons | 44.2 s | ×2.14 |
  | + Spikey Pillow | 42.2 s | ×2.04 |

  Either takes the batch past 2×, so both stay Rust (section 3). The final build's bench:

  | Number | Parent (base) | This build | Ratio | Budget |
  |---|---|---|---|---|
  | `fuzz --seeds 200` wall time | 22,403.052 ms | 26,401.764 ms | ×1.18 | ≤ 2× |
  | Slowest `gate` decision | 102.302 ms | 39.699 ms | ×0.39 | ≤ 1.5× and < 1,500 ms |
  | Golden step | 2.602 ms | 2.967 ms | ×1.14 | ≤ 2× |

  `luau bench: within every budget`. The parent's slowest decision came out near 100 ms in both
  runs and this build's lower; the bench reports it as measured.

- **`luau-analyze --mode=strict`: not run.** This machine has no Luau toolchain, and the batch
  fetches nothing. `build.rs` lints and compiles every new file (`jackioh_luau::compile`), and each
  loads and runs in its card's tests. Every file starts with `--!strict`; hook contexts are
  annotated (`J.EffectContext`, `J.ConditionContext`, `J.HookArgs`); the union-typed `FuseInto` is
  hoisted into an annotated local. As in part 21, the target checks take an unannotated `args`
  (`jackioh.luau` exports no `TargetCheckArgs` type), and The Maestro's `canActivate` calls
  `plague_on`, a shared reader `ConditionContext`'s type does not list.
- `cargo jackioh catalog check`, `patches check` and `spec check` (805 rulings, 1710 proofs,
  27,218 citations): clean. `cargo fmt --check` fails on one file this part does not touch,
  `crates/luau/tests/api.rs:1101` (an `assert!` rustfmt would put on one line), as at the base
  commit and as part 21 found; the plan forbids `crates/luau`, so it is the reconcile's. Every
  file this part changed is formatted. `cargo clippy --workspace --all-targets -- -D warnings`:
  clean.

## 6. Decisions the plan did not settle

- **The auras that name no zone keep naming none.** Suppressive Aura's base aura applies to every
  unit (`|_unit| true`), its Radiant aura to every unit its controller does not control, and Rush
  Token Farm's to its controller's Rush Tokens: none checks the unit's zone, so they reach any
  instance whose layers are read. The data filter only matches a field row (`rows`), so the ports
  use named `aura_filter` predicates, which are exact. A predicate is handed the candidate alone,
  so where the Rust compares controllers the hook names the predicate for its own controller
  (`aura_filter` for `p1`, `aura_filter_2` for `p2`, chosen through a two-entry table). Rejected:
  `{ side = …, rows = { "units", "backrow" } }`, which is cheaper and differs from the Rust for
  any non-field instance a stat read is asked about.
- **Weapons and Spikey Pillow, not Suppressive Aura, stay Rust.** The plan keeps "a card in Rust
  if its aura breaks the budget"; no aura breaks it alone, so the batch kept the cheapest ones
  that fit together (section 5). Rejected: keeping all four auras in Rust (×1.05), which the plan
  gives no reason for, and porting the two most expensive ones, which leaves no room for a third.
- **The parent was built in this worktree** from the base with section 1's fixes, before any card
  file changed, and copied out, as parts 15, 21 and 24 did. Rejected: a second worktree and a
  second full release build of the same commit.
- **Three `luau diff` runs**, since one 20-card deck holds at most 20 forced cards. The third
  forces the Rust cards beside ported ones they meet (Mega Church's wide Tribute and Sheeople,
  Unan, the auras).
- **One function under both faces where the Rust built two closures** (Expedition12's `cry`,
  Aluneth's `endOfTurn`): the faces share one adapter (gap 14). The engine never compares hooks by
  pointer, and no test of either card does, as part 21 found for Devil's Pact and Reoccurring
  Dream.
- **Numbers the Rust wrote inline in `json!`** are named locals with the text they come from
  (`LOST_TURNS`, `ECHO`, `PER_MANA`, `AI_TAG`), CLAUDE.md rule 9; the `uses = 1` of a plain
  Activate stays inline, as in parts 21 and 24. The Maestro's two labels are the strings the Rust
  `format!` builds.
- **Wishing Well reads Lucky off the live card** (`ctx:numbered_keywords_on(self.id)`), where the
  Rust reads the context's `self_`. An Activate's run takes `self_` as `find_instance` clones it
  when the run begins (`prompts.rs`, `run_resume`; no self snapshot rides in its data), and the
  hook only reads, so the two are the same card.
- **Sheeople was not given a literal worth**, and the Felinors were not ported partly: a card is
  one script source, and a literal would restate the catalog.

## 7. Workspace tests

WORKSPACE
