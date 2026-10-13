# v0.4.0 Part 10: Core Cry cards, second half, and the Core tokens

Notes for issue #452 (v0.4.0 part 10 of 29, batch 2 of the Luau ports), built by the procedure in
`docs/v0.4.0/PORTING.md` from base `c65d6d55c` on `bot/issue-442`.

## 1. Cards

Twenty of the plan's 22 run from Luau. Each has its `.luau` beside its `.rs`; the `.rs` keeps its
header, `pub const ID` and its `mod tests`.

| Card | `.luau` |
|---|---|
| #51.1 KY's Empty Notebook (`core-051-1`) | `c051_1_kys_empty_notebook.luau` |
| #54 Straaza (`core-054`) | `c054_straaza.luau` |
| #56 Jilliax (`core-056`) | `c056_jilliax.luau` |
| #57 Conjure KY (`core-057`) | `c057_conjure_ky.luau` |
| #61 Prejudiced Postdoc (`core-061`) | `c061_prejudiced_postdoc.luau` |
| #62 Friend of Felinors (`core-062`) | `c062_friend_of_felinors.luau` |
| #63 Plastic Surgery (`core-063`) | `c063_plastic_surgery.luau` |
| #67 Zoomerbin Oomen (`core-067`) | `c067_zoomerbin_oomen.luau` |
| #74 Adaptive UI (`core-074`) | `c074_adaptive_ui.luau` |
| #76 Field of Dreams (`core-076`) | `c076_field_of_dreams.luau` |
| #80 Zao Gao (`core-080`) | `c080_zao_gao.luau` |
| #90 CN-Viral Injection (`core-090`) | `c090_cn_viral_injection.luau` |
| #93.1 Combo-Fodder (`core-093-1`) | `c093_1_combo_fodder.luau` |
| #94 Genn's Greed (`core-094`) | `c094_genns_greed.luau` |
| #95.1 Chaos Golem (`core-095-1`) | `c095_1_chaos_golem.luau` |
| T-bread Bread Token (`core-t-bread`) | `t_bread.luau` |
| T-coin The Coin (`core-t-coin`) | `t_coin.luau` |
| T-felinor Felinor Token (`core-t-felinor`) | `t_felinor.luau` |
| T-ghoul Ghoul Token (`core-t-ghoul`) | `t_ghoul.luau` |
| T-rush Rush Token (`core-t-rush`) | `t_rush.luau` |

### Staying in Rust (PORTING.md §5, reason 1: the API cannot say it)

- **#77 Professor Curvature (`core-077`).** Its Cry's modifier expires `{ until = "nextTurnOf",
  player, fromTurn = <the turn it is made on> }` (R48), and no reader gives a Luau script the turn
  number (`ctx.state.turn`): `EffectContext` binds `controller`, `x`, `radiant` and the rest, but not
  the turn, and `add_player_modifier` takes `fromTurn` as given. `docs/v0.4.0/api-gaps.md` does not
  list this gap; eight card scripts read `state.turn` outside their tests today (this one, Classic
  #23 Devil's Pact and #65 Ace in the Hole among them).
- **#83 Transmogulate (`core-083`).** R223 walks the library in `numbering_order`, and
  `ctx:numbering_order(cards)` refuses the instances `ctx:zone_cards` hands out: it reads them back
  as `Vec<CardInstance>`, and a card with no granted keywords crosses back with
  `grantedKeywords = {}`, which the walk turns into an empty record, so every call fails ("invalid
  type: map, expected a sequence", every one of the card's 15 tests). With the library walked top
  down instead the card's own tests pass, but each library replacement would take a different pick
  and id than the Rust's, so that is no port. The rest of the card translates: `card_at` is
  Units-only in Luau, but `ctx:permanents_on_field(me)` walks the controller's units and then their
  backrow, lane by lane, which is the Rust's board walk.

Both keep their `.rs` exactly as at the base commit and have no `.luau`.

## 2. What the port needed beyond the translation

- **#94's ids go to `for_each_card` outright.** Its Rust reads the 2-cost library cards in the
  `cards` closure. In Luau a named `each_cards` function's empty answer crosses as `{}`, and the host
  refuses it ("each_cards: a cards function answers ids, not a record"; three of the card's tests
  panicked), so the Cry reads the ids itself and passes `cards = ids` (an empty list is coerced to
  `[]` by the verb's argument read). This is the same read at the same moment: `for_each_card` is
  the Cry's first effect, nothing runs between the hook's return and its start, and from then on
  the clause keeps the ids in its memo either way (R113).
- **An absent reader answer is not `nil`.** `ctx:live_self()` and `ctx:slot_of(id)` hand back
  mlua's null, a userdata, when the Rust `Option` is `None`, so `if x == nil` never fires; #67
  checks `type(x) ~= "table"`. The first `luau diff` run found it (#67's Cry, in a game where its
  unit was gone when the Cry ran: "attempt to index userdata with 'id'").
- **`J.TRAP_TYPES` is cast** (`J.TRAP_TYPES :: any`, #67): the stub types it `{string}`, which a
  query's `type: CardType | {CardType}` does not accept under strict.
- **The pools are `J.query` with `excludeDefId`.** `crate::query::pool(own, args)` is
  `catalog::query` with `own` added to `excludeDefId`, and #54 and #57 already wrote it that way in
  Rust. The trial port of #83 did the same, since the stub types `J.pool` as taking one argument
  while the host takes two.
- **Faces mirror the Rust's sharing.** A Rust `radiant: base.clone()` is `radiant = "base"`; a Rust
  that builds each face anew (`coin()` twice, `zao_gao(…, false)` / `zao_gao(…, true)`) is a table
  per face, because the tests of #80 and The Coin assert that the two faces' Cry hooks are not the
  same `Arc`, and `radiant = "base"` clones base's.

## 3. Tests

No assertion changed. What did change, each the least that compiles:

- `script()` (or `super::super::script()`) is `crate::scripts_of()[ID].clone()` (or
  `[super::super::ID]`): The Coin, the four empty tokens, #61, #67, #80, #95.1. #61's test module
  imported `super::script` and now imports `super::ID`.
- The Coin's and #80's test modules import `jackioh_engine::prelude::*`: their `use super::*` had
  reached `Arc` through the file's own prelude import, which went with the script.
- #90's and #94's test modules lost a `use super::*` that no longer imports anything they use, which
  `-D warnings` would refuse.
- `c095_call_to_chaos.rs`, another card's file, had one test reading `c095_1_chaos_golem::script()`;
  it reads the Golem's scripts from the registry the same way. #95 stays Rust under L14, so no batch
  edits that file.

## 4. Results

Run on this box (linux x86_64, 4 threads), against the parent built from the base commit in its own
worktree (`cargo build --release -p jackioh-tools`).

- `cargo test -p jackioh-cards --lib -- <the 22 stems> c095_call_to_chaos`: 350 passed on the
  first run with every card ported, 18 failed: #83's 15 and three of #94's, both above; after #94's
  fix and #83's return to Rust, all pass.
- `cargo jackioh golden check` with only The Coin ported (it is in every game): 240 of 240 games
  replayed identically (22,713 steps).
- `cargo jackioh luau diff --parent-bin ../base-452/target/release/jackioh --cards <the 20 ids>`:
  200 games, seeds 1–200, 23,957 steps, with core-051, core-054, core-056, core-057, core-061,
  core-062, core-063, core-067, core-074, core-076, core-080, core-090, core-093, core-094, core-095,
  core-018, meditative-027, classic-032 and classicplus-015 forced (the tokens through the cards
  whose `refs` name them): **0 mismatches**. The first run stopped on #67's null (above).
- `cargo jackioh golden check`: 240 of 240, 0 diverged. `cargo jackioh fuzz --seeds 200`: 200
  passed, 0 panics, 0 replay mismatches, 0 invariant violations.
- `cargo jackioh luau bench --parent-bin ../base-452/target/release/jackioh` (Intel Xeon Platinum
  8370C, 4 threads), within every budget:

  | Number | Parent | This build | Ratio | Budget |
  |---|---|---|---|---|
  | `fuzz --seeds 200` wall time | 12,156.0 ms | 12,339.2 ms | ×1.02 | ≤ 2× |
  | Slowest `gate` perf decision (scaled) | 241.8 ms | 243.5 ms | ×1.01 | ≤ 1.5× and < 1500 ms |
  | Golden step | 1.818 ms | 1.816 ms | ×1.00 | ≤ 2× |
- `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings`: clean.
  `catalog check`, `patches check` and `spec check` (805 rulings, 1710 proofs): ok.
- `cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts`: the cards' 5,860
  unit tests and 752 cross-card tests, the engine's (rules and golden included), the AI's, the
  Luau host's and the server's all pass, and `apps/web/src/wire` is unchanged. Three tools tests
  fail, none of them this part's: `luau::surface::tests::luau_core_002_bigot` and
  `luau_reports_print_as_markdown_and_json` (part 7's notes record both, failing since part 4
  ported #2 Bigot, whose `.rs` the surface scanner reads), and `acceptance.rs`'s
  `v2_the_pure_crates_link_only_serde_serde_json_indexmap_and_each_other` (`crates/cards/Cargo.toml`
  depends on `jackioh-luau` since part 4). This part touches neither #2 nor a manifest; they were
  not re-run at the base commit.
- `luau-analyze --mode=strict`: **not run**. No Luau toolchain is on this box and the release
  download was refused here, as in part 7; `cargo build` compiled and linted every new file through
  `jackioh_luau::compile`. The cast above is written for strict; the reconcile's strict run is
  the check.
