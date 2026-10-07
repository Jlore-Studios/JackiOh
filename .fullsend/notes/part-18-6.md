# Slice: part 18, chunk 6 of 6 (server 1: the settings, stats, tutorial, env and ranked tests)
BUILDS-RUN: 0

## FILES
- `crates/server/tests/api/settings.rs` (new) ← `apps/server/test/api/settings.test.ts`: 14 `it`s, 3 `describe`s.
- `crates/server/tests/api/stats.rs` (new) ← `apps/server/test/api/stats.test.ts`: 7 `it`s.
- `crates/server/tests/api/tutorial.rs` (new) ← `apps/server/test/api/tutorial.test.ts`: 10 `it`s, 3 `describe`s.
- `crates/server/tests/api/env_deployed_commit.rs` (new) ← `apps/server/test/env-deployed-commit.test.ts`: 4 `it`s.
- `crates/server/tests/api/glicko2.rs` (new) ← `apps/server/test/ranked/glicko2.test.ts`: 10 `it`s.
- `crates/server/tests/api/ladder.rs` (new) ← `apps/server/test/ranked/ladder.test.ts`: 16 `it`s.
- `crates/server/tests/api/season.rs` (new) ← `apps/server/test/ranked/season.test.ts`: 6 `it`s.
Every `it` is a `#[test]`/`#[tokio::test]` named by SURFACE §7.3 (R-ids leading), every `describe` a `mod`, in TS order;
the header comments and every comment citing a ruling are kept.

## SURFACE
- §11.2: HTTP tests go through `support::deps::{test_app, call}` and the store only through `Db::begin` / `Tx::<substore>_<method>` /
  `Tx::commit`. Store inputs are built from TS's object literals with a private `from(json!(…))` (serde), results read as JSON
  or by `.id`, so the tests depend on the TS JSON shape, not on Rust field spellings (as part 20 chunk 2 did).
- §5.1: wire answers (`VisibleRank`, `PeakBadge`, Grape tier names, every HTTP body) are compared as JSON.
- §7.3: names are the TS titles snake_cased with their R-ids leading.

## DEPENDS-ON (names I call that other parts write)
- `crate::support::deps` (part 18, `tests/support/deps.rs`): `test_app().await -> Arc<App>` with the E2E fixtures seeded
  (`e2e-p1`/`e2e-p2` active, `e2e-pending` pending, tokens `e2e-token-p1|p2|pending`), and
  `call(&Arc<App>, method: &str, path_and_query: &str, token: Option<&str>, body: Option<Value>).await -> (u16, HeaderMap, Value)`.
  One call site per test file (`get`/`put`/`request`), so a different shape is a one-line fix in each.
- `jackioh_server::app::App { db, .. }` (part 18), `db.begin(None)`, `Tx::commit` (part 20).
- `Tx` methods (part 20 chunk 4's names): `profiles_get_by_user_id(&str) -> Result<Option<Profile>, _>`, `profiles_create(&ProfileCreateInput)
  -> Result<Profile, _>`, `profiles_set_status(&str, ProfileStatus)`, `player_settings_get(&str) -> Result<Option<_>, _>`,
  `tutorial_get(&str) -> Result<Option<_>, _>`, `game_records_insert(&GameRecord)`, `player_stats_put(&str, &IndexMap<String, Value>,
  bool, i64)`; `Profile.id: String`; `ProfileStatus`, `ProfileCreateInput`, `GameRecord` deserialise from TS's JSON.
- `jackioh_server::config` (part 18): `PLAYER_SETTINGS_{BYTES_MAX, GROUPS_MAX, KEYS_MAX, NAME_MAX_LENGTH, TEXT_MAX_LENGTH}`,
  `TUTORIAL_LESSONS_MAX`, `TUTORIAL_LESSON_ID_MAX_LENGTH`, `CARD_STATS_{CACHE_TTL_SECONDS, MIN_SAMPLE}`, `PLAYER_STATS_{BYTES_MAX,
  CACHE_TTL_SECONDS}`, `PUBLIC_STATS_MIN_LIVE_GAMES` (any integer type: counts go through a generic `count()`), `RATING_START`,
  `RATING_DEVIATION_START` (any `Into<f64>`), `RATING_VOLATILITY_START: f64`, `GLICKO_TAU: f64`, `SEASON_RESET_STRENGTH`,
  `SEASON_RESET_DEVIATION_BOOST` (`Into<f64>`), `RANK_DIVISIONS_PER_TIER`, `RANK_PIPS_PER_DIVISION`, `RANK_PLACEMENT_GAMES`,
  `RANK_STREAK_LENGTH`, `RANK_CONVERGENCE_GAP_PIPS` (`i32`, the ladder's integer), `JLORIOUS_SIZE` (any integer),
  `RANK_TIER_PERCENTS.{rotten, normal, large, golden, mythic}`.
- `jackioh_server::env` (part 18): `load_env(&IndexMap<String, String>) -> Result<Env, E: Debug>` (as part 20 chunk 3 calls it),
  `Env.deployed_commit: Option<String>`, `SERVER_ONLY_ENV_VARS: &[&str]`.
- `jackioh_server::ranked::glicko2` (part 18): `Glicko { rating, deviation, volatility }` (f64s; `Debug + PartialEq`),
  `RatedOpponent { opponent, score }`, `Score = f64`, `START_GLICKO: Glicko` (a `const`), `glicko2_period(&Glicko, &[RatedOpponent],
  tau: f64) -> Glicko`, `rate_game(&Glicko, &Glicko, Score) -> <struct with fields a, b>`.
- `jackioh_server::ranked::season` (part 18): `season_id_of(&str) -> String` (panics with TS's "… is not a patch version …"),
  `soft_reset(&[ResetPlayer]) -> <struct with fields changes: Vec<ResetChange>, report: ResetReport>`, `reset_glicko(&Glicko, f64)
  -> Glicko`, `ResetPlayer { profile_id, glicko }`, `ResetChange { profile_id, before, after }`, `ResetReport { players, mean,
  spread_before, spread_after, deviation_before, deviation_after, lowest_before, highest_before, lowest_after, highest_after }`
  (`Debug + PartialEq` on both).
- `jackioh_server::ranked::ladder` (part 18): `GRAPE_TIERS` and `GrapeTier` (`Serialize` as TS's literals), `LADDER_TOP`, `PIPS_PER_TIER`
  (`i32`), `fresh_rank(&str, &str, i64)`, `tier_index_of(i32)` (same type as `SeasonRank.floor`), `tier_bottom(<index>) -> i32`,
  `place_of(i32) -> LadderPlace { tier, division, pips }`, `Percentile { numerator, denominator }`, `percentile_of(f64, &[f64])`,
  `target_ladder(&Percentile) -> i32`, `pip_delta(&PipDeltaInput { result, ladder, target, streak }) -> i32`,
  `apply_ranked_game(&SeasonRank, &ApplyRankedGameInput { result, target, at }) -> SeasonRank`, `jlorious_order(&[Standing])
  -> Vec<String>`, `Standing { profile_id, ladder: Option<i32>, rating: f64 }`, `with_jlorious_peak(&SeasonRank, i32) -> SeasonRank`,
  `visible_rank(Option<&SeasonRank>, Option<i32>) -> VisibleRank`, `peak_badge(&SeasonRank) -> Option<PeakBadge>` (both
  `Serialize` as TS), `GameResult::{Win, Loss, Draw}` (`Copy`), `SeasonRank { season_id, profile_id, games, wins, losses, draws,
  ladder: Option<i32>, floor, streak, peak_ladder: Option<i32>, peak_jlorious: Option<i32>, updated_at }` (`Clone`).
- `jackioh_cards::{catalog_version, catalog_json}` (part 1).

## GAPS
- Names for TS's anonymous types I had to guess (part 18's `ranked/*.rs` must use them, or part 31 renames at one call site each):
  `PipDeltaInput` (pipDelta's `input`), `ApplyRankedGameInput` (applyRankedGame's `input`); `rate_game`'s `{ a, b }` and
  `soft_reset`'s `{ changes, report }` read as fields `.a`/`.b` and `.changes`/`.report`.
- Argument forms guessed: struct arguments by reference (`&Glicko`, `&Percentile`, `&SeasonRank`), `glicko2_period`'s TS default
  `tau = GLICKO_TAU` passed explicitly as a plain `f64`. Each sits behind one private wrapper per file (`period`, `rate`, `reset`,
  `target`, `delta`, `apply`).
- `season_id_of` panics where TS threw (SURFACE §4.4.9); the test catches the unwind and reads the message. If part 18 returns
  `Result`, `refusal_of` in `season.rs` becomes a `match` on it.
- The server's clock under `test_app()` is assumed to be the wall clock (epoch ms from `SystemTime`): App has no clock field
  (SURFACE §11.2) and §11.3 maps `Timers` to tokio, which has no epoch time. If `deps.rs` gives the app a fake clock, the four
  R320/R634 time tests (`wall_ms()` call sites) must read it instead.
- `test_app()` is assumed to serve the real catalog (`jackioh_cards`); stats.rs's expectations are read off `catalog.json`.
- `db/mod.rs` re-exports nothing (part 20 chunk 4's gap); my files never name a store type, so they are unaffected.

## Decisions
- TS's per-suite deps (`createTestDeps`, `seedProfile`, `addUser`) do not exist for an `App`: the caller is the fixture `e2e-p1`, the
  other account `e2e-p2`, the pending one `e2e-pending`; a banned account is `e2e-p2` flipped with `profiles_set_status`.
  `store.tables.playerSettings`/`.tutorial` are read as `*_get` for each fixture profile (the only profiles there are).
- TS's "declares GET and PUT …, both `active`" read `createXRoutes()`; Rust has no per-file route list (ROUTES is app.rs's), so the
  two tests check it through the app: GET and PUT answer 200, POST/DELETE/PATCH the router's 404 `not_found`, and a pending
  account 403 on both (which an `user` route would let in).
- "A minute later" (`timers.advance(60_000)`) becomes "a moment later": the test waits until the wall clock is past the clamped
  time, then writes with the device time read just before; that time is ≤ the server's now, so it is stored as sent.
- stats.rs: the real catalog replaces TS's six defs (core-001 is "Big D-fender" in both); the filter test compares id sets against
  `catalog.json` filtered the way the test names (set, rarity, numeric cost with `X` = 0 and embiggen = base, cost ≥ 6), not the
  six hand-picked ids. The current patch is `catalog_version()`. TS's five `mode: "tutorial"` records are dropped: `GameMode` is
  `bo1 | bo3 | random` and cannot hold them; the totals they were excluded from are asserted unchanged. Listed players are made
  with `profiles_create` (+ `active`), so their ids are the store's.
- env_deployed_commit.rs: `CATALOG_VERSION` is `catalog_version()`, not `"core-1"`, because the Rust server refuses any other
  (SURFACE §11.3); the variables are an `IndexMap<String, String>` (TS's `Record<string, string | undefined>` minus the undefineds).
- Integer constants pass through a generic `count()` (`usize::try_from`) and numeric ones through `float()` (`Into<f64>`), so
  the tests compile whatever width `config.rs` picks and clippy's `useless_conversion` has nothing to flag; no `.clone()` on
  values that may be `Copy` (`clone_on_copy`).
- `toMatchObject` is a private recursive subset check (`is_match`); `toBeCloseTo(x, d)` is `|a − b| < 10^−d / 2`.
