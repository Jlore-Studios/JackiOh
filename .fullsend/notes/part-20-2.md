# Slice: part 20, chunk 2 of 5 (server 3: the store contract suite, grants.sql, two SQL suite copies)
BUILDS-RUN: 0

## FILES
- `crates/server/tests/store/contract.rs` (new): `apps/server/test/db/contract.ts` whole (101 `it`s → 101 cases, in TS
  order, one module per `describe`), with `test/db/harness.ts` folded in (`StoreHarness`, `fixture_catalog`,
  `playable_ids`, `token_ids`, `CATALOG_VERSION`, `TRUNCATE`, `REDEMPTION_ENABLED_SQL`, `database_url`, `admin_client`,
  `seed_cards`) and `contract.memory.test.ts` / `contract.postgres.spec.ts` as the `both_stores!` runner: every case
  runs as `<describe>::memory::<case>` always and `<describe>::postgres::<case>` when `DATABASE_URL` is set (skips,
  passing, when unset).
- `crates/server/tests/db/grants.sql` (new): `test/db/grants.sql`, statements unchanged, header path now
  `crates/server/tests/db/run.sh`.
- `crates/server/tests/sql/08_game_records.sql`, `crates/server/tests/sql/12_ranked.sql`: `cp`, byte for byte (`cmp`).

## SURFACE
- §11.2: cases take the harness (which holds `db: Db`) rather than a bare `&Db`, because TS's harness also reaches
  state no port method exposes (email verification, the redemption switch, auth.users). Every TS call on `store`
  is `let mut t = db.begin(None).await?; t.<substore>_<method>(…).await; t.commit()` (the `call!`/`q!` macros:
  commit on `Ok`, drop = rollback on `Err`). TS's `store.tx(async t => …)` is one begin … commit; a nested `tx`
  is the outer `&mut Tx` passed down.
- §5.1: every port value is built from TS's own object literal with `serde_json::from_value(json!({…}))` and read
  back as `serde_json::to_value(&x)`, so the cases depend on the JSON shape (TS's keys, camelCase), not on Rust field
  spellings or integer widths. Direct field access only where §4.2 makes it certain: `.id`, `.code_hash`, `.status`,
  `.card_id`, `.quantity`, `.uses`.

## DEPENDS-ON (names I call that other parts write)
- `jackioh_server::db::store` (part 20 chunk 4): `Db::{Pg(PgPool), Fake(Arc<tokio::sync::Mutex<FakeData>>)}`,
  `Db::begin(&self, Option<&str>) -> Result<Tx<'_>, StoreError>`, `Tx::commit(self)`, `StoreError: Display + Debug`
  with `StoreError::Duplicate { .. }` (chunk 3 says `{ match_id }`), and the `Tx` methods
  `<substore>_<method>` with chunk 3's argument shapes (`&str`, `&T`, `&[T]`, `Option<&str>`, caps `usize`, epoch ms
  `i64`): profiles_{get_by_id, get_by_user_id, get_many(&[String]), create(&ProfileCreateInput),
  set_status(&str, ProfileStatus), set_glicko(&str, &Glicko), set_display_name, set_in_match, remove},
  codes_{insert, find_by_hash, claim, log_attempt, count_attempts_by_profile, oldest_attempt_at_by_profile,
  count_attempts_by_ip, count_failures}, redeem(&RedeemInviteCodeInput), purge_expired(&RetentionPurgeInput),
  collection_{get, upsert_quantities(&str, &[CollectionEntry]), append_grants(&[CollectionGrant])},
  decks_{list, get, upsert(&SavedDeck, usize), remove}, trios_{list, get, upsert(&SavedTrio, usize), remove},
  tutorial_{get, merge(&TutorialMergeInput, usize)}, player_settings_{get, merge(&PlayerSettingsMergeInput,
  &PlayerSettingsLimits)}, last_boards_{get(&str, LastBoardKind), put(&str, LastBoardKind, &[LastBoardEntry], i64),
  sample_others(&[String], usize)}, player_stats_{get, put(&str, &IndexMap<String, Value>, bool, i64),
  list_public(&ListPublicOptions)}, series_{create, get, update, by_match, with_game, active_for, active},
  matches_{create(&MatchRow), get, append_actions(&[MatchActionRow]), actions, set_clocks(&str, &MatchClocks), finish,
  live, mode_of, discard_open, forget_voided}, rooms_{create(&Room), get, claim(&str, &str, &str, i64)},
  tickets_{insert(&Ticket), get, open_for_profile, list_open, count_open, count_open_by_mode, claim_pair, cancel},
  game_records_{insert(&GameRecord), list(&GameRecordQuery)}, results_{insert(&ResultRow), get_by_match, record_for},
  ranked_{lock_seasons, seasons, create_season(&Season), rated_players, reset_ratings(&[ResetChange]), standings, rank,
  ranks_of, put_rank(&SeasonRank), note_peak_jlorious(&str, &str, <int>), bot, put_bot(&BotRating),
  record_game(&RatedGameRow), game}.
- Port types imported from `db::store` (chunk 4), each needing `Serialize + Deserialize` with TS's camelCase JSON,
  plus `Clone + Debug + PartialEq` (the cases compare whole rows): `BotRating, CodeAttempt, CollectionEntry,
  CollectionGrant, FrozenTrio, GameRecordQuery, InviteCode, LastBoardKind, ListPublicOptions, MatchActionRow,
  MatchClocks, MatchRow, PlayerSettingsLimits, PlayerSettingsMergeInput, PlayerSettingsRow, Profile,
  ProfileCreateInput, ProfileStatus, RatedGameRow, RedeemInviteCodeInput, ResultRow, RetentionPurgeInput, Room,
  SavedDeck, SavedTrio, Season, SeriesRow, Ticket, TutorialMergeInput, TutorialProgressRow`. Result types read only
  through `serde_json::to_value`, so they need `Serialize` with TS's literals: `RedeemResult` ("ok", "not_pending", …),
  `UpsertOutcome`, `TrioUpsertOutcome`, `QueueMode`, `ProfileStatus`, `TutorialMergeOutcome` and
  `PlayerSettingsMergeOutcome` (`#[serde(tag = "kind", rename_all = "camelCase")]`: `{kind:"merged", progress|settings}`
  / `{kind:"limit"}`), `RetentionPurgeResult`, `ProfileRecord`, `SeasonStanding` (flattened rank + `rating`),
  `PublicPlayerSummary`, `ResetPlayer`, the count-by-mode map.
- `jackioh_server::db::fake` (part 20 chunk 3, as its notes name them): `create_e2e_store(E2eStoreOptions { catalog:
  FakeCatalog { card_ids: Vec<String>, is_token, is_banned }, now: Arc<dyn Fn() -> i64 + Send + Sync>, redemption:
  Option<RedemptionSettings> }) -> Db` (I take the `Arc<Mutex<FakeData>>` back out of `Db::Fake`; if it answers
  `FakeData`, wrap it instead), `FakeData::reset(&mut self)`, `RedemptionSettings { email_verified: Arc<dyn Fn(&str) ->
  bool + Send + Sync>, enabled: Arc<dyn Fn() -> bool + Send + Sync>, .. }`, `default_redemption_settings()`.
- `jackioh_server::ranked::{glicko2::Glicko, ladder::{SeasonRank, fresh_rank(&str, &str, i64) -> SeasonRank},
  season::ResetChange}` (part 18), all serde with TS's JSON.
- `jackioh_engine::{Action, GameRecord, LastBoardEntry}`: part 1's `Action` and `LastBoardEntry`, part 5's
  `wire::stats::GameRecord` (serde, PartialEq, Debug, Clone).

## GAPS
- `db/store.rs` must use the engine's `jackioh_engine::LastBoardEntry` (state.rs) for `last_boards_*` and
  `MatchRow.last_boards`/`glitch_boards`, not a second copy (ports.ts defined its own; it is the same `{defId, radiant}`).
- Every port type above must derive `Deserialize` too (inputs and results included), or `from(json!(…))` fails to
  compile; the alternative is rewriting those literals as struct literals (part 35).
- Test-binary parallelism: my Postgres runs serialize on `store::contract::PG_SERIAL` (a `tokio::sync::Mutex`), but
  the other Postgres suites in this binary (`postgres.rs`, `redeem_race.rs`, `seed_catalog.rs`, chunks 1/4/5) truncate
  the same database. TS ran them with `fileParallelism: false`; `crates/server/tests/db/run.sh` should run the Pg half
  with `cargo test -p jackioh-server --test server store:: -- --test-threads=1`, or every suite should take
  `PG_SERIAL` (part 31).
- Lines over rustfmt's 110 columns (about 100): formatting only; part 31/35 run `cargo fmt`.

## Decisions
- `ProfileStore.setRating` is not ported (SURFACE §11.2), but three TS cases used it: "moves the rating and the
  in-match pointer" and "tx joins a nested transaction" write the same rating through `profiles_set_glicko` at a new
  profile's deviation 350 and volatility 0.06 (the identical row); R603's case drops its final block, which only
  checked that `setRating` left deviation and volatility alone.
- `expect(x).rejects.toThrow()` is `assert!(call!(…).is_err())`; `toThrow(DuplicateResultError)` is
  `matches!(e, StoreError::Duplicate { .. })` (a braced pattern matches a tuple or struct variant); R611's message
  check is `e.to_string().contains("rated_games already holds a row for <id>")`.
- `toBeUndefined()` on an optional row field is "absent from the JSON, or null" (`absent`), so either serde
  convention for `Option` passes.
- The Postgres harness casts the binds TS left to `pg`'s inference (`$1::uuid` in the auth.users delete), since sqlx
  declares a `String` bind as `text`; `newUserId` reads `returning id::text`. `TRUNCATE` runs through `sqlx::raw_sql`.
- The memory harness holds the two values `app.redeem_invite_code` reads and the port cannot (verification and the
  switch) in an `Arc<std::sync::Mutex<IndexSet>>` and an `Arc<AtomicBool>` behind the fake's `RedemptionSettings`
  closures, exactly as TS held them behind its hooks.
- Each Rust test builds its own harness (its own pools: a sqlx pool cannot outlive the per-test tokio runtime), then
  calls `reset` first (TS's `beforeEach`) and `close` last (TS's `afterAll`).
- TS's `harness.name` survives as the store name in every failure message (`q!` panics with it and the call's text).
