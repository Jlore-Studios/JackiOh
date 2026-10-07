# Slice: part 20, chunk 1 of 5 (#408): `crates/server/src/db/pg.rs`
BUILDS-RUN: 0

## FILES
- `crates/server/src/db/pg.rs` (new): the whole port of `apps/server/src/db/store.ts` (2979 lines),
  in TS order: the header's three rules, `DECISIVE_REASONS`, `ACTING_ROLE`, `SESSION_SQL`, the value
  conversions, the session (`begin`, `commit`, `run_as`), every row shape and its `to_*`, the
  ranked rows, `create_postgres_store`/`close`/`assert_postgres_url`, then one `pub async fn` per
  store method (`redeem`, `redeem_invite_code`, `purge_expired`, `profiles_*`, `codes_*`,
  `collection_*`, `decks_*`, `trios_*`, `matches_*`, `rooms_*`, `tickets_*`, `results_*`,
  `series_*`, `tutorial_*`, `player_settings_*`, `last_boards_*`, `game_records_*`, `ranked_*`,
  `player_stats_*`), the small helpers and the KNOWN DIVERGENCES block. Every `SET LOCAL`
  role/claim (`SESSION_SQL`) and every `app.*` call of the TS is kept, SQL text verbatim.

## SURFACE
- §11.2: `pg::<substore>_<method>(t: &mut PgTx<'_>, …TS args in TS order)`, `PgTx<'a> =
  sqlx::Transaction<'a, Postgres>` (what `Tx::Pg` holds). `Db::begin` is meant to call
  `pg::begin(&pool, claim_sub) -> Result<PgTx<'static>, StoreError>` (BEGIN via `pool.begin()`, then
  `SESSION_SQL` with `service_role` and the claim); `Tx::commit` calls `pg::commit(t)`.
- `pg::create_postgres_store(&PostgresStoreOptions { connection_string, max: Option<u32> }) ->
  Result<PgPool, StoreError>` (lazy pool, the TS URL check, 10 connections, 10 s idle and acquire
  timeouts) for `app.rs`; `pg::close(&pool)`.

## DEPENDS-ON
- `crate::db::store` (part 20, store.rs chunk): the row types and enums imported at the top of
  pg.rs, with fields named by SURFACE §4.2 from ports.ts. See GAPS for the exact shapes assumed.
- `crate::ranked::glicko2::Glicko`, `crate::ranked::ladder::SeasonRank` (Serialize),
  `crate::ranked::season::{ResetPlayer, ResetChange}` (part 18).
- `jackioh_engine::wire::{GameRecord, parse_game_record, portrait_or_default, sources_of}` (part 5),
  `jackioh_engine::{Action, LastBoardEntry}` (part 1).

## GAPS
Names called that another part provides (and the shape pg.rs assumed):
- `crate::db::store::StoreError` with variants `Db(sqlx::Error)`, `Other(String)` and
  `Duplicate(String)` (the match id, TS's `DuplicateResultError`). Only `db_error()` and `fail()`
  (top of pg.rs) and one line of `results_insert` name them.
- store.rs field types assumed (SURFACE §4.3; part 31 reconciles any compile error):
  epoch ms `i64` / `Option<i64>` everywhere (`created_at`, `updated_at`, `at`, `since`, `now`,
  `expires_at`, `enqueued_at`, `ended_at`, `pick_deadline`, `MatchClocks.*`, `Season.started_at`);
  ratings `f64` (`Profile.rating*`, `Ticket.rating`, `ResultRow.rating_before/after: (f64, f64)`,
  `Glicko`); counts and caps `i32` (`InviteCode.max_uses/uses`, `CollectionEntry.quantity`,
  `CollectionGrant.delta`, `ResultRow.turns`, `SeriesRow.version`, `SeasonRank` ints and
  `Option<i32>`s, `BotRating.games`, `ProfileRecord.*`, `max_decks`, `max_trios`, `max_lessons`,
  `PlayerSettingsLimits.{max_groups,max_bytes}`, `count`, `limit`, `offset`, `position`, the
  counters `codes_count_*`/`tickets_count_open` return); `MatchActionRow.seq: i64`;
  `RetentionPurgeResult.{code_attempts,match_actions}: i64`.
- Tuples for TS's fixed arrays: `MatchRow.players: (String, String)`, `decks: (Vec<String>,
  Vec<String>)`, `last_boards`/`glitch_boards: Option<(Vec<LastBoardEntry>, Vec<LastBoardEntry>)>`,
  `portraits: Option<(PortraitId, PortraitId)>`, `ResultRow.players`, `SavedTrio.deck_ids:
  (Option<String>, Option<String>, Option<String>)`, `FrozenTrio.decks: (FrozenDeck, FrozenDeck,
  FrozenDeck)`, `SeriesRow.sides: (SeriesSide, SeriesSide)`, `RatedGameRow.sides: (RatedSide,
  RatedSide)`. If store.rs used `[T; N]`, `.0`/`.1` become `[0]`/`[1]`.
- `MatchClocks.grace_deadline` has `.p1`/`.p2` (`PerPlayer<Option<i64>>` or a struct); built
  through serde, so either compiles.
- Option fields: `Profile.display_name: Option<String>`, `Ticket.portrait`, `Room.host_portrait`,
  `SavedDeck.portrait`, `FrozenDeck.portrait: Option<String>` (TS's absent and null both `None`),
  `MatchRow.{ranked: Option<bool>, mode: Option<QueueMode>}`, `SeriesRow.ranked: Option<bool>`.
- Built through serde (type-agnostic, so any Rust shape that serialises as TS does compiles):
  `MatchRow.stake`, `RatedGameRow.winner_side`, `SeriesRow.{games, winner, end_reason,
  rating_before, rating_after}`, `SeriesSide.{wins, pick}`, `RatedSide.{pilot, rank_before,
  rank_after}`, `RatedGameRow.{kind, reason}`, `SeasonStanding` (a `SeasonRank` plus `rating`),
  `PublicPlayerSummary` (with its anonymous `favouriteCards` and `funStats`), every string union
  (`QueueMode`, `ProfileStatus`, `TicketStatus`, `MatchStatus`, `UpsertOutcome`,
  `TrioUpsertOutcome`, `RedeemResult`, `LastBoardKind`, `GameOverReason`, `GameSource`, `GameMode`,
  `CodeAttemptResult`, `PortraitId`).
- Named directly: `TutorialMergeOutcome::{Merged { progress }, Limit}`,
  `PlayerSettingsMergeOutcome::{Merged { settings }, Limit}`, `TutorialHiddenChoice { hidden, at }`,
  `RedeemInviteCodeInput { profile_id, code_hash: Option<String>, ip_hash }`,
  `RetentionPurgeInput { code_attempts_before, match_actions_ended_before }`,
  `GameRecordQuery { source, mode: Option<_>, patch: Option<String> }`,
  `PlayerSettingsMergeInput { profile_id, groups: IndexMap<String, PlayerSettingsGroup>, at }`,
  `TutorialMergeInput { profile_id, completed: Vec<String>, hidden_choice, at }`.
- Method argument shapes store.rs's dispatch must match: `profiles_create(t, user_id, email,
  rating: f64, at: i64, display_name: Option<&str>)` (TS's anonymous input object spread into
  arguments); `player_stats_list_public(t, search: Option<&str>, limit: i32, offset: i32)` (same);
  `last_boards_get/put(t, profile_id, kind: LastBoardKind, …)`; `profiles_get_many(t,
  &[String])`; `last_boards_sample_others(t, &[String], count: i32)`;
  `tickets_count_open_by_mode -> IndexMap<QueueMode, i32>` (needs `QueueMode: Hash + Eq`).
- Part 5: `jackioh_engine::wire::portrait_or_default(Option<&str>) -> PortraitId`,
  `sources_of(SourceFilter) -> Vec<GameSource>` (`SourceFilter: Copy`),
  `parse_game_record(&Value) -> Result<GameRecord, E: Display>`; `GameRecord { id, source, mode,
  patch, .. }: Serialize`.
- sqlx 0.9 API assumed as 0.8's: `sqlx::query(&'static str)` (every statement is a `&'static str`,
  so this also holds under 0.9's `SqlSafeStr`), `.bind`, `.fetch_optional/.fetch_all/.execute(&mut
  **t)` (`Transaction: DerefMut<Target = PgConnection>`), `Pool::begin() -> Transaction<'static>`,
  `Transaction::commit`, `PgPoolOptions::{max_connections, idle_timeout, acquire_timeout,
  connect_lazy}`, `sqlx::types::Json<T>`, `Row::try_get(&str)`, `DatabaseError::{code,
  constraint}`; Decode for `Uuid`, `OffsetDateTime`, `Value`, `Vec<String>`, `i16`/`i32`/`i64`/`f64`.
- Not ported, on purpose: `profiles.setRating` (SURFACE §11.2: no caller); `createStore` /
  `postgresStore` aliases and `StoreUnavailableError` plumbing (SURFACE §11.3); the `onError` pool
  option and `pool.on("error")`, `checkout`'s error listener, `poolSession`, `joinedSession`,
  `withQuery`, `boundQuery` (`pg` driver plumbing; their doc comments are kept on `begin`,
  `run_as` and `create_postgres_store`); `pg`'s `keepAlive` (no sqlx option).

## Decisions
- One transaction per caller: there is no top-level one-statement session in Rust, every method
  runs in the `Tx` its caller opened, and each re-stamps its subject first (`run_as`), exactly as
  TS's `joinedSession.run` did. A failed statement aborts that Postgres transaction, so a caller
  that catches a store error and goes on (queue.rs's duplicate ticket, results.rs's retry after
  `Duplicate`) must begin a fresh `Tx`, as TS's top-level calls each ran in their own (said in
  `begin`'s doc). No savepoints were added.
- SQL: `ts("$n")` / `nullableTs("$n")` / the `*_COLUMNS` constants became the macros `ts!`,
  `nullable_ts!`, `*_columns!` and `match_values!`, joined with `concat!`, so each statement is a
  `&'static str` with TS's exact text. `nullable_ts!` sits beside `ts!` at the top (a macro must
  precede its use), not in "Small helpers" where TS had `nullableTs`.
- `ranked_lock_seasons` binds `SEASON_LOCK_ID` (`$1::bigint`) instead of interpolating it: same
  number, no integer literal in SQL text.
- Binds: strings bound as text and cast by the SQL (`$1::uuid`, `$4::jsonb`), JSON bound as
  `serde_json::to_string` text (TS's `json()`); epoch ms bound as `i64` into `$n::double precision`.
- Reads: uuid columns decoded as `Uuid` then `to_string()` (lower-case hyphenated, as Postgres
  prints it); timestamptz as `OffsetDateTime`, truncated to ms; jsonb as `Value`, except
  `player_settings.groups` and `player_stats.stats`, read through `Json<IndexMap<…>>` so the keys
  keep jsonb's printed order as node-pg's `JSON.parse` did.
- String unions cross through their serde form (`literal`, `from_literal`) after TS's own checks
  and messages, so no enum variant name is guessed.
- Deleted accounts (migration 0012): a seat TS read back as `null` in a string slot reads back as
  `""` (`MatchRow.players.0`, `ResultRow.players`, `SeriesSide.profile_id`); `to_match` still
  refuses a row without `p2_profile_id`, as TS did. Added to KNOWN DIVERGENCES.
- `assert_postgres_url`'s message names `src/db/fake.rs` instead of `src/api/e2e-store.ts`.
- `to_public_player_summary` and `count_number` are private copies of the fake's (TS imported
  them from memory-stores.ts), building `PublicPlayerSummary` through serde.
- Comments and doc comments are TS's, with paths and names moved to the Rust ones
  (`e2e-store.ts` → `src/db/fake.rs`, `test/db/contract.ts` → `tests/store/contract.rs`,
  `postgres.spec.ts` → `tests/store/postgres.rs`, `matches.create` → `matches_create`, …).
