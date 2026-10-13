# v0.4.0 Part 11: cards to Luau, batch 3 (Classic Cry cards and Glitch)

Notes for issue #453 (v0.4.0 part 11 of 29), started on `bot/issue-442` at `c65d6d55c` (parts 4–7
landed, every Classic card still Rust) and finished on `385516a75`, after parts 9, 12, 14 and 15
had landed (none of them touches a card of this batch, `crates/luau` or `crates/engine`). The
procedure is `docs/v0.4.0/PORTING.md`; the parent (oracle) is a release `jackioh` built from
`385516a75`, where this batch's cards are still Rust, and section 5's proofs were run against it.

## 1. Ported: 20 of 25

Each card's `.luau` sits beside its `.rs`, which keeps its header, `pub const ID` and its tests. In
the tests only the lookups changed: `script()` became `crate::scripts_of()[ID].clone()`, and the
imports followed (`use super::ID;`, `use std::sync::Arc;` where `Arc` had come in through the card's
removed `use jackioh_engine::prelude::*;`, and a top-level `use super::*;` dropped where nothing was
left to import). No assertion changed.

| Card | Luau |
|---|---|
| `classic-002` The Trickster | `add_cost_rule`: base `amount = -discount`, radiant `setTo = setCost`, both `lasts = "used"` |
| `classic-003` Book of Heal | `targets` (any unit or hero, `aim = "help"`), `heal` the chosen one `heal` |
| `classic-012` Book of Blood | `targets` (any unit), `damage` with `lifesteal = true` |
| `classic-016` Book of Flame | `targets` (any unit or hero), `damage` |
| `classic-024` Book of Knowledge | `draw` |
| `classic-025` Lag in the System | `for_each_card` exile over the reachable cards at cost now ≤ `threshold` (base: the field; radiant: the enemy's field, hand and deck) |
| `classic-029` Book of Vital Kill | `targets` (any hero), `set_health` 13; radiant adds `add_to_hand` `classic-016` |
| `classic-031` Cookie Guild | `units` × `recruit` (Unit, `costRange.max = costLimit`) |
| `classic-034` Ancient Acquisition | `add_random_from_graveyard` (`exile = true` on radiant) |
| `classic-037` Last Hurrah | `draw` the library's count, `discard_hand_at_turn_end` `this` / `next` |
| `classic-041` State of the Game | `base = {}` (keywords only) |
| `classic-067` Felinor Feeler | `for_each_card` `switch_position_of` DEF over the enemy's active units |
| `classic-070` Book of Plague | `place_plague_tokens` |
| `classic-071` Lane Eater | `destroy` the card in each other zone of its lane, then `lock_lane` (radiant: enemy side) |
| `classic-073` Nurse Cleaver | `base = {}` (keywords only) |
| `classic-076` Plague Bringer | `place_plague_tokens`, `draw` |
| `classic-083` Flame Lance | `targets` (any unit), `damage` with `trample = true` |
| `classic-085` King Wagtoggle | `swap_library`; radiant then `recruit` `recruits` |
| `classic-086` Genn | `base = {}` (vanilla) |
| `classic-t-glitch` Glitch | `glitch` |

The two Plague cards (#70, #76) only place counters (`place_plague_tokens`); nothing in them reads
Pestilent Slime's multiplier hook, so they do not wait on part 20.

## 2. Kept in Rust: 5 (PORTING §5)

- `classic-026` Rapid Draw — reason 2. Its `forEachCard` set is read after the draws (which can
  pause) and is empty in most games (more cards in hand than the discard); an empty Luau table
  crosses as `{}`, a record, and `crates/luau/src/api/each.rs` refuses any `cards` answer that is
  not a list, so the faithful port panics. Ports once the host takes `{}` as an empty list.
- `classic-030` Recycle — reason 1. Its `each` needs data the Cry read (the radiant discount, a
  tuned number) and its second set needs the Cry's graveyard snapshot; `for_each_card`'s named
  `cards` and `each` functions take no data.
- `classic-035` Prep — reason 1. Its modifier's `expiry` is `{ until = "thisTurn", turn = … }` and
  no reader answers the turn number (`ctx.state.turn`).
- `classic-039` Outbreak — reason 1. Its draw half reads this run's own `controlChanged` in
  `ctx.events[ctx.events_from..]` (R136) to tell a card it just stole from its own permanent; no
  reader answers the run's events, and no state field tells the two apart (a steal is not a field
  departure, R171, so `stands_since_script_began` stays true).
- `classic-079` Risky Die — reason 1. Its first `each` sets `costMod` to `-discount`, a tuned number
  read in the Cry; `each` takes the id alone.

## 3. Host findings for the reconcile (not fixed here: `crates/luau` is not this part's)

- **An empty list cannot answer a `cards` function.** `numbers::walk_table` turns an empty table
  into `{}`, and `build_for_each_card` panics on anything but an array ("a cards function answers
  ids, not a record"); `effect_calls` already takes an empty record as an empty list, and the same
  arm in `each.rs` would fix it. Every batch whose `cards` function can come up empty hits it.
  Here #25 and #67, whose `forEachCard` is the Cry's first and only effect (so the Cry and the set
  read the same board), read the set in the Cry and pass it outright (`cards = ids`), leaving
  `cards` out when it is empty: the engine then builds the same empty part the Rust closure did.
- **A reader's `None` arrives as mlua's null, not `nil`.** `to_lua` serialises `Option::None` as
  `Value::NULL` (light userdata), so `ctx:card_at(…) ~= nil` is true for an empty zone and indexing
  it fails. #71 tests `type(card) == "table"`.
- **`api-gaps.md` does not list the three gaps of section 2**: no turn reader, no reader of the
  run's events, and `for_each_card`'s named functions taking no data. The batches landed beside
  this one found the same four: part 9 keeps Core #35 Lunar Eclipse in Rust for the turn reader,
  and part 12 keeps C+ #19 League of Losers for the empty list and C+ #23 Dropshipping for the
  `each` data, and lists the missing events reader.
- `card_at` binds only the units row (`ZoneRef { row: Units }`); #71 finds a backrow card through
  `permanents_on_field` (the same `card_at` walk) filtered by its zone.

## 4. Decisions the plan did not settle

- **#25 and #67 read their set in the Cry**, not in a `cards` function (section 3). Rejected: the
  named-function form, which panics on an empty board; keeping both in Rust, which section 3's
  equivalence makes unnecessary.
- **#26 stays Rust** rather than answering a sentinel id whose `each` is a no-op. Rejected: a
  sentinel puts a made-up id into the part's memo and a no-op effect into the list, to dodge a host
  bug the reconcile can fix in one line.
- **Empty cards (#41, #73, #86) do not `require("@jackioh")`.** They call nothing, and an unused
  `J` is a `LocalUnused` warning under `luau-analyze`. Rejected: the require PORTING's opening line
  suggests, which a strict run would flag.
- **The tests read the registry with `crate::scripts_of()[ID]`**, as Classic+ #40 and #47's tests
  already read it through `crate::scripts_of()`. Rejected: `registered_entry`, which depends on
  `register_all()` having run in the same test.
- **Glitch is proved by its own tests and the golden traces.** `luau diff --cards classic-t-glitch`
  refuses it ("no deckable card's refs name this token"): it reaches a hand only through R673's
  roll, never a creator's `refs`.
- **The parent was rebuilt once the branch moved.** The first run's parent was this worktree's
  release build at `c65d6d55c` before the first edit; after parts 9, 12, 14 and 15 were merged in
  underneath this part, every proof of section 5 was run again against a parent built from
  `385516a75` in a detached worktree (PORTING step 1), so the parent differs from this build by
  this batch alone. Rejected: keeping the first parent, which would also differ by the other parts'
  cards.

## 5. Proofs

Machine: linux x86_64, 4 threads, AMD EPYC 9V45. Date: 2026-10-13. Parent: `385516a75`.

- `cargo test -p jackioh-cards --lib -- <the 25 file stems>`: 321 passed, 0 failed.
- `cargo test -p jackioh-cards`: 5,860 unit tests and 752 cross-card tests passed.
- `cargo jackioh luau diff --parent-bin <parent> --cards classic-002,classic-003,classic-012,classic-016,classic-024,classic-025,classic-029,classic-031,classic-034,classic-037,classic-041,classic-067,classic-070,classic-071,classic-073,classic-076,classic-083,classic-085,classic-086`:
  200 games (seeds 1–200, 12,046 steps), 0 mismatches. As a check that the forced cards are
  played, the first run built Book of Knowledge drawing one too many and saw 14 of 20 games diverge
  from its parent; the card was then restored.
- `cargo jackioh golden check`: 240 of 240 games replayed identically (22,713 steps).
- `cargo jackioh fuzz --seeds 200`: 200 passed, 0 panics, 0 replay mismatches, 0 invariant
  violations.
- `cargo jackioh luau bench --parent-bin <parent>`: within every budget.

  | Number | Parent | This build | Ratio |
  |---|---|---|---|
  | `fuzz --seeds 200` | 7,085.287 ms | 6,906.936 ms | ×0.97 |
  | Slowest decision | 245.571 ms | 247.857 ms | ×1.01 |
  | Golden step | 1.189 ms | 1.170 ms | ×0.98 |

- `cargo jackioh spec check`, `catalog check`, `patches check`: ok. `cargo fmt --check` and
  `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts --no-fail-fast`:
  every test passes except three in `jackioh-tools` whose causes predate this part:
  `luau::surface::tests::luau_core_002_bigot` and `luau_reports_print_as_markdown_and_json` (part
  7's notes report both on the base: surface reads `script()` out of the `.rs` files part 4
  emptied), and
  `acceptance::v2_the_pure_crates_link_only_serde_serde_json_indexmap_and_each_other` (part 4 made
  `jackioh-luau` a dependency of `jackioh-cards`); parts 12 and 15 report the same three.
  `apps/web/src/wire/` is unchanged.
- `luau-analyze --mode=strict`: not run. Neither run's box had a `luau-analyze`, and the
  `luau0-src` crate `mlua` builds from ships no Analysis library to build one from. The files follow PORTING §2 (annotated contexts,
  `ipairs` only, no `/`, a hoisted `J.TargetSpec` for `heal`'s union argument); the build's lint and
  compile pass on all twenty.
