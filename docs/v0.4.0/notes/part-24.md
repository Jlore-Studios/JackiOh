# v0.4.0 Part 24: cards to Luau, batch 16 (Meditative start-of-turn, end-of-turn, Death and trigger hooks)

Notes for issue #763 (v0.4.0 part 24 of 29, a fullsend part of #442), built by
`docs/v0.4.0/PORTING.md`. Of the batch's 29 cards, 24 run from Luau now and 5 stay Rust (section
2). Base commit: `5e292a8fb` (`bot/issue-442` with part 28's API fixes, every card of this batch
still Rust).

## 1. Ported to Luau

Each has a new `crates/cards/src/scripts/meditative/<module>.luau`. Its `.rs` lost `script()`, the
helpers and constants only `script()` used, and the `use` lines. It keeps its header comment,
`pub const ID`, the comment above its tests and `mod tests`. Every effect is in the Rust order,
every trigger id is the Rust one, a `when` stands where the Rust has `with_when` and nowhere else,
and a key the Rust `json!` writes (`radiant = false` included) is written.

| Card | Id | Luau hooks | Notes |
|---|---|---|---|
| Introspection | `meditative-007` | `endOfTurn` | two `for_each_card`s, the heal and the discount carried in `data` (gap 2); `radiant = "base"` |
| Gatling Pea | `meditative-013` | `endOfTurn` | `set_number` on `param:damage`; `radiant = "base"` |
| Trenful Trickster | `meditative-016` | `endOfTurn` | `max(traps, 0)` `summon_random`s over `J.TRAP_TYPES`; `radiant = "base"` |
| Forbiddenous Factory | `meditative-029` | `cry`, `endOfTurn` | `ctx:plague_on(me.id)` gates the spend; `radiant = "base"` |
| Fickle E-Kitten | `meditative-030` | `startOfTurn` | `highest_permanent_cost` answers `nil` for a side with no permanent (gap 12) |
| H1B Printer | `meditative-038` | `startOfTurn` | `costOverride = 0` on the Radiant face only |
| Jade Beauty | `meditative-039-5` | `endOfTurn` | `allure_enemy_units`; `radiant = "base"` |
| CN Smuggler | `meditative-041` | `startOfTurn` | the Rock, then the CN pick |
| CN Jade Well | `meditative-044` | `startOfTurn` | |
| 饕餮 | `meditative-047` | `endOfTurn` | one `fuse_cards`, the Radiant face a second from the library |
| YileGPT Tamed | `meditative-049` | `cry`, `death` | the lane across read off `live_self().zone` |
| Joon Jorker | `meditative-061` | `endOfTurn` | the neighbour through `slot_of` and `card_at`, bounded by `J.config.UNIT_ZONES`; `radiant = "base"` |
| Skull of J'Nari | `meditative-063` | `startOfTurn` | |
| Sentient Cat Ears | `meditative-067` | `endOfTurn` | `radiant = "base"` |
| Medina Enforcer | `meditative-083` | `endOfTurn` | `radiant = "base"` |
| Rampaging Rhino | `meditative-002` | `triggers` (`damage`, with `when`) | `radiant = "base"` |
| Jlockwork Machine | `meditative-003` | `triggers` (`cardPlayed`) | the controller and the count reach the `cards` hook through `data` (gap 2); `radiant = "base"` |
| Joint Filing | `meditative-009` | `turnHookExtra` | `args:param("extra")`; `radiant = "base"` |
| Double Header | `meditative-011` | `triggers` (`cardResolved`) | the two faces' trigger ids differ, as in Rust |
| YileGPT Unleashed | `meditative-049-1` | `cry`, `startOfTurn`, `handTriggers`, `deckTriggers` | `do_random_entries` with the Rust labels (string interpolation); the Radiant Cry flattens all five entries |
| AI Girlfriend | `meditative-049-3` | `startOfTurn`, `afterAttacked` | `after_attacked_of` answers `nil` outside a combat (gap 12); `radiant = "base"` |
| Traitorous Blood | `meditative-064` | `triggers` (`attackDeclared`, with `when`) | `ctx:rng_pick` draws as `rng.pick` does: `int(len)`, nothing on an empty list |
| Catnip | `meditative-068` | `triggers` (`cardResolved`, with `when`) | the resolved face's type read through `ctx:def_of` (section 5); `radiant = "base"` |
| CN Golem | `meditative-095-1` | `triggers` (`destroyed`) | |

Absent answers are tested with `== nil`: since part 28 a reader that answers Rust's `None` hands
Luau `nil` (#464 gap 12), so the `type(x) == "table"` guard the earlier batches needed is gone.

## 2. Kept in Rust

Each keeps its `.rs` exactly as the base commit has it and has no `.luau`. All five are PORTING.md
§5's first reason, a missing API function. None of these gaps is in `docs/v0.4.0/api-gaps.md`, and
this part does not edit it (the plan names only the card files and these notes, and the batches
build side by side): #442's reconcile folds them in.

| Card | Id | Why |
|---|---|---|
| Clip-Farming Lawyer | `meditative-027` | it asks whether any zone of either side and **either row** is Locked; `ctx:is_locked(player, lane)` fixes the row to Units, and no reader answers a backrow zone's Lock (a Locked zone takes moves, R688, so it may hold a card, and `open_zones` cannot tell it either) |
| Eschews | `meditative-060` | its `tributeWhen` sums every `deaths@…` key of its memory (R102: a fused card's parts remember apart); Luau cannot list a record's keys (L7 refuses `pairs`, `next` and a generic `for` over anything but `ipairs`), and no reader answers a memory's keys or a sum over a key prefix |
| Keymaster Keenus | `meditative-065` | its Radiant Death grants `keywords_of` the card **as it died** (`ctx.self_`, R78's last-known state); `ctx:keywords_of(id)` resolves the id to the live instance, which in a Death hook is the reset graveyard card or the Reborn body. Tried: a port on `ctx:keywords_of(ctx:self_().id)` failed 1 of the card's 9 tests (`r1085_a_second_death_grants_again`: 20 keywords granted where it had 19 as it died). Needs a snapshot read, as part 28 gave `killer_of` (gap 9) |
| Tatches the Totem | `meditative-087` | its deck trigger tests the played card's tags against the engine's `TRIBAL_TAGS` (`crates/engine/src/config.rs`, R1424); `J.config` holds the `i32` constants only, and writing the five tags into the script would restate an engine constant |
| Shrinking Felinor | `meditative-094` | its Death reads `base_stats_of` the card as it died (R1181: the `statsOverride` R78 clears in the graveyard); `ctx:base_stats_of(id)` resolves the live instance, the graveyard copy, whose override is gone (the card's header: "Reading the graveyard copy instead would restart every chain at 9/11"). Tried: that port failed 3 of the card's 8 tests (both chain tests and the Radiant override test). Needs the same snapshot read as Keenus |

## 3. The tests

No test asserts anything new or anything less: no added or removed line holds `assert`, the five
Rust cards' files are untouched, and the 24 ported files hold 218 `#[test]`s, as at the base. Three
kinds of line changed, all because `script()`, its constants and the module's `use` lines are gone:

- `super::script()` is `crate::scripts_of()[ID].clone()`, the registry `build.rs` generates, so the
  test reads the Luau card as the engine registers it (Gatling Pea, Trenful Trickster, Rampaging
  Rhino, Joint Filing, Double Header).
- A constant the tests named through `use super::*` or `use super::{CN_VIRUS, ID}` moved into the
  test module with its doc line: `CURSE` (Factory), `LOVE_BOMB` (Kitten), `ROCK` (Smuggler, Jade
  Well), `BOMB` (Cat Ears), `CN_VIRUS` (Golem, whose import is `use super::ID;` now).
- A `use super::*;` that then imported nothing the tests use is removed, as part 14 removed three
  (Introspection, Factory, Kitten, Printer, Skull, Cat Ears, Catnip).

## 4. Proofs

All run on this build, in this worktree, on linux x86_64, 4 threads, AMD EPYC 7763. The parent is
`jackioh` release-built from the base commit in this worktree before any file changed (4 m 01 s,
`git status` clean), then copied outside it (`/tmp/parent-jackioh`, sha256 `b27fcbbd…`).

- **Card tests.** `cargo test -p jackioh-cards`: the unit tests 5,860 passed (the 218 of the 24
  ported files among them), the cross-card tests 752 passed, 0 failed.
- **`luau diff`**, against the parent. All 28 forceable ids at once are refused ("26 forced cards
  do not fit a deck of 20"), so the batch runs in two halves:
  - Introspection, Gatling Pea, Trickster, Factory, Kitten, Printer, Smuggler, Jade Well, 饕餮,
    Tamed, Unleashed (forced through Tamed), Joon Jorker, Skull: 200 games, seeds 1–200, 15,948
    steps, `golden check: 200 of 200 games replayed identically`, **0 mismatches**;
  - Cat Ears, Enforcer, Rhino, Jlockwork, Joint Filing, Double Header, Traitorous Blood, Catnip,
    Jade Beauty and CN Golem (both forced through M #95, the card whose `refs` name them), and the
    five Rust cards, which meet the ported ones in play: 200 games, 22,743 steps, **0 mismatches**.
  - AI Girlfriend can't be forced: the tool refuses it, since no deckable card's `refs` name it
    (YileGPT Unleashed, the token whose Cry summons it, does). It comes into those games through
    Unleashed.
- **What those games ran.** The same games, re-recorded with `golden record --seeds 1..200` and
  each half's forced list (the step counts match), folded by a throwaway counter outside the
  repository: games with the card / plays / arrivals on the field / its controller's turn starts
  and ends with it on their field. Introspection 184 / 120 / 122 / 297 + 328; Gatling Pea
  193 / 189 / 217 / 572 + 654; Trickster 182 / 174 / 208 / 481 + 589; Factory 189 / 173 / 173 /
  444 + 501; Kitten 186 / 216 / 248 / 828 + 744; Printer 188 / 234 / 234 / 804 + 871; Smuggler
  187 / 198 / 229 / 512 + 629; Jade Well 195 / 204 / 206 / 590 + 633; 饕餮 189 / 129 / 173 /
  11 + 162; Tamed 189 / 194 / 223 / 472 + 569; Joon Jorker 188 / 161 / 197 / 358 + 446; Skull
  189 / 165 / 167 / 575 + 630; Cat Ears 200 / 287 / 302 / 375 + 466; Enforcer 200 / 270 / 279 /
  485 + 632; Rhino 199 / 273 / 276 / 607 + 759; Jlockwork 200 / 278 / 287 / 553 + 715; Joint
  Filing 200 / 290 / 292 / 1,475 + 1,574; Double Header 200 / 253 / 256 / 1,119 + 1,224; Traitorous
  Blood 199 / 281 / 289, 207 firings; Catnip 200 / 304 / 313, 337 firings. The tokens: Unleashed
  played 7 and 13 times, AI Girlfriend summoned 5 and 7 times, Jade Beauty 42 times, CN Golem 32.
- **`golden check`**: 240 of 240 games replayed identically (22,713 steps), 0 diverged.
- **`fuzz --seeds 200`**: 200 passed, 0 failed; 0 panics, 0 over the bound or stalled, 0 replay
  mismatches, 0 invariant violations; 200 hero deaths.
- **`luau bench --parent-bin`**: within every budget.

| Number | Parent (base) | This build | Ratio | Budget |
|---|---|---|---|---|
| `fuzz --seeds 200` wall time | 17,707.2 ms | 18,751.5 ms | ×1.06 | ≤ 2× |
| Slowest `gate` decision | 118.5 ms | 116.6 ms | ×0.98 | ≤ 1.5× and < 1,500 ms |
| Golden step | 2.340 ms | 2.393 ms | ×1.02 | ≤ 2× |

- **`luau-analyze --mode=strict`: not run.** This box has no Luau toolchain and fetching the 0.740
  release was refused. `build.rs` lints and compiles all 24 files, and each loads and runs in its
  card's tests. Every hook context and trigger event is annotated, every event is narrowed on
  `event.type` before a field is read, every `TargetSpec` that is a whole argument or sits in
  `HealArgs` is a hoisted annotated local, `J.TRAP_TYPES` is cast as Zoomerbin Oomen casts it,
  and a `highest_permanent_cost` answer is held in a `number?` local before it is tested for `nil`.
- `cargo clippy -p jackioh-cards --all-targets -- -D warnings`: clean. `cargo jackioh catalog
  check`, `patches check` and `spec check` (805 rulings, 1,710 proofs): clean.
- `cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts`: the AI's 325,
  the cards' 5,860 and 752, the engine's 376, rules 2,637 and golden 19, the Luau host's 60 and
  the server's 1,268 (plus 10 unit and export tests) passed; `apps/web/src/wire/` unchanged.
  `jackioh-tools`: 171 passed and three fail, none on a file or card of this part, the same three
  part 15's notes report failing on its base: the surface tests `luau_core_002_bigot` and
  `luau_reports_print_as_markdown_and_json`, which survey `core-002` (a Luau pilot since part 6)
  and `classic-090`, and `acceptance.rs`'s
  `v2_the_pure_crates_link_only_serde_serde_json_indexmap_and_each_other`, which names
  `crates/cards/Cargo.toml`'s `jackioh-luau` dependency. The workspace run stops at the first
  test binary that fails, so the tools' integration tests and the wasm crate (no tests) were run
  after it with `--no-fail-fast`.
- `cargo fmt --check` fails on one file this part does not touch: `crates/luau/tests/api.rs`
  line 1101, an `assert!` rustfmt wants on one line, as the base commit has it (part 28's file;
  the plan keeps this part out of `crates/luau`).

## 5. Decisions the plan did not settle

- **Catnip reads the resolved face's type off its definition.** Its `when` asks the engine's
  `card_type_of_face(def_id, event.radiant)`. The bound `ctx:card_type_of_face(id)` passes the
  running card's own face (`ctx.radiant`), so it would ask Catnip's face, not the played card's.
  The port reads `ctx:def_of(event.defId)` and takes that face's `type`, else the definition's,
  which is that function's whole body (`crates/engine/src/faces.rs`). Rejected: keeping Catnip in
  Rust for a reader whose only gap is which face it is told.
- **Factory, Joon Jorker and Catnip re-read their own instance by id** (`ctx:plague_on(me.id)`,
  `ctx:slot_of(me.id)`, `ctx:is_animated(me.id)`), where the Rust hands `live_self()` to
  `plague_on`, `slot_of` and `is_animated`. Those readers resolve the id with `find_instance`, the
  same lookup `live_self` makes; the two differ only when the card is gone or a combat continuation
  carries its snapshot, and neither happens to an end-of-turn hook or a trigger, which a card
  answers only from the field (R153). Rejected: reading `me.counters.plague` and `me.zone` straight
  off the table, which would restate the readers' arithmetic in the card.
- **`radiant = "base"` where the Rust called a no-argument factory once per face** (Trickster,
  Factory, Cat Ears, Rhino, Jlockwork): one shared face instead of two equal ones. Nothing in the
  engine compares hook identity, and no test of these cards does.
- **Numbers the Rust wrote inline in `json!`** (a count of 1, a cost of 0) are named locals in the
  Luau, with the card text they come from (`SPENT`, `BOMBS`, `FREE`, `UNLEASHED_COUNT`; CLAUDE.md
  rule 9).
- **Test-only constants moved into the tests** rather than staying in the `.rs` under
  `#[cfg(test)]`, so the file's non-test half is its header and `ID`, like every ported card's.
- **Two trial ports were run, then reverted** (Keenus, Shrinking Felinor), to name the failing
  tests in section 2 rather than reason about them; both `.rs` files are back byte for byte from the
  base commit.
- **The parent was built in this worktree from the untouched base**, as part 15 did, not in a
  second worktree: the same commit, one release build saved. The `.luau` files were drafted outside
  the worktree until it had finished.
