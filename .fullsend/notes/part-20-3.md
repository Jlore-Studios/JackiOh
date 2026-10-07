# Slice: part 20, chunk 3 of 5 (server 3: the fake store, the migration runner, season-start, the SQL suite copies)
BUILDS-RUN: 0

## FILES
- `crates/server/src/db/fake.rs` (new): `e2e-store.ts` + `memory-stores.ts`, one fake.
- `crates/server/src/db/migrate.rs` (new): `migrate.ts`.
- `crates/server/src/cli/season_start.rs` (new): `season-start.ts`.
- `crates/server/tests/store/migrations_pinned.rs` (new): `migrations-pinned.test.ts`, plus one check.
- `crates/server/tests/sql/{00_supabase_stub,01_schema_invariants,02_rls_as_client,04_decks_and_series,07_retention_purge,11_player_settings}.sql`: `cp`, unchanged.
- `crates/server/tests/sql/run.sh`: `cp`, then `SQL=crates/server/tests/sql`, a new `MIGRATIONS=crates/server/migrations` used in both places TS named `apps/server/src/db/migrations`, and three comments repointed. The other SQL files it applies (03, 03b, 05, 06, 08, 09, 10, 12, 13, 14) are other chunks' copies.
- `crates/server/.env.example`: `cp`, unchanged.
- `package.json`: `test:sql`, `test:db`, `test:deploy` now `sh crates/server/tests/{sql/run.sh,db/run.sh,deploy/rehearse.sh}`.

## SURFACE
- `db/fake.rs` follows §11.2: `FakeData` behind `Db::Fake(Arc<tokio::sync::Mutex<FakeData>>)`; `FakeTx<'a>` holds the
  `MutexGuard<'a, FakeData>` and a snapshot of the tables taken at begin, restored on drop unless `commit` ran.
  One sync free fn per store method, `<substore>_<method>(f: &mut FakeTx<'_>, <TS args in TS order>) -> Result<T, StoreError>`,
  plus `redeem`, `purge_expired`, `begin(&Arc<Mutex<FakeData>>) -> Result<FakeTx, StoreError>` (async) and
  `commit(FakeTx) -> Result<(), StoreError>`.
- `db/migrate.rs` follows §11.3: ledger, `REWRITTEN`, `pg_advisory_xact_lock(0x6a61636b)` per file, files embedded
  with `include_str!`, checksum = `jackioh_engine::replay::fnv1a32_utf16`.
- `run(args: Vec<String>) -> anyhow::Result<()>` in `db::migrate` and `cli::season_start`, as part 1's `main.rs` calls them.

## DEPENDS-ON (names I call that other parts write)
- `crate::db::store` (part 20 chunk 4): `Db` (`Db::Pg(PgPool)`, `Db::Fake(Arc<Mutex<FakeData>>)`), `Db::begin(Option<&str>)`,
  `Tx::commit`, `StoreError`, and the row types below.
- `jackioh_engine::replay::fnv1a32_utf16(&str) -> String` (part 5).
- `jackioh_engine::wire::stats::{GameRecord, GameSource, CardStatsFilter { source, mode, patch, pilot }, PilotFilter::Unified,
  record_matches(&GameRecord, &CardStatsFilter) -> bool, DEV_RECORD_ID_PREFIX}` (part 5).
- `crate::api::collection::{LAUNCH_COPIES: i32, LAUNCH_GRANT_REASON: &str}` (part 18/19).
- `crate::config::{CODE_ATTEMPTS_PER_PROFILE_PER_HOUR, CODE_ATTEMPTS_PER_IP_PER_HOUR, CODE_ATTEMPT_WINDOW_SECONDS}`
  (any integer type `i64::from` takes) and `{RATING_START, RATING_DEVIATION_START, RATING_VOLATILITY_START}` (any type
  `f64::from` takes) (part 18).
- `crate::ranked::glicko2::Glicko { rating, deviation, volatility }`, `crate::ranked::ladder::SeasonRank { season_id,
  profile_id, peak_jlorious: Option<i32>, … }`, `crate::ranked::season::{ResetPlayer { profile_id, glicko }, ResetChange
  { profile_id, before, after }}` (part 18).
- `crate::api::ranked::{SeasonDeps { patch_version: String }, OpenedSeason { season: Season, opened: bool, reset:
  Option<ResetReport> }, open_season_in_tx(&mut Tx<'_>, &SeasonDeps) -> Result<OpenedSeason, E: Display>}` (part 18).
- `crate::env::load_env(&IndexMap<String, String>) -> Result<Env, E: Display>` and `Env.database_url` (part 18).

## GAPS
- `StoreError` variants I construct (only in `fake.rs`'s `fail` and `duplicate_result`, two lines to fix if wrong):
  `StoreError::Other(String)` for a TS `throw new Error(msg)`, `StoreError::Duplicate { match_id: String }` for
  `DuplicateResultError`.
- Names I gave TS's anonymous types, which `db/store.rs` must define with these names and fields (or part 31 renames):
  `ProfileCreateInput { user_id, email, rating: f64, at: i64, display_name: Option<String> }` (`ProfileStore.create`'s
  input), `ListPublicOptions { search: Option<String>, limit: usize, offset: usize }` (`PlayerStatsStore.listPublic`),
  `CardCount { id: String, count: i64 }` (`favouriteCards`' items), `FunStats { nemesis_card_id: Option<String>,
  total_destroyed: i64, total_defeated: i64 }`, `SeasonStanding { rank: SeasonRank, rating: f64 }` (TS `SeasonRank & {
  rating }`; `#[serde(flatten)]` on `rank` keeps the JSON).
- Field and type assumptions on `db/store.rs`'s row types (§4.3 applied): epoch ms and counts `i64`; ratings `f64`;
  quantities, deltas, uses/max_uses `i32`; caps (`max_decks`, `max_trios`, `max_lessons`, `PlayerSettingsLimits.{max_groups,
  max_bytes}`, `sample_others`' `count`) `usize`; TS 2-tuples are Rust tuples (`ResultRow.players.0`, `SeriesRow.sides.0`,
  `RatedGameRow.sides.0`); `TrioSlots` is the 3-tuple `(Option<String>, Option<String>, Option<String>)`;
  `Profile.display_name: Option<String>`; unit-variant unions are `Copy` enums named by their literals (`QueueMode::{Bo1,
  Bo3, Random}`, `UpsertOutcome::{Created, Updated, Limit, NotOwner}`, `TrioUpsertOutcome::{…, UnknownDeck}`,
  `RedeemResult::{Ok, NotPending, EmailUnverified, RateLimitedProfile, RateLimitedIp, CircuitOpen, InvalidCode}`, …);
  `TutorialMergeOutcome::{Merged { progress }, Limit}`, `PlayerSettingsMergeOutcome::{Merged { settings }, Limit}`;
  `countOpenByMode` answers `IndexMap<QueueMode, i64>` (TS `Record<QueueMode, number>`); `PlayerStatsRow.stats` and
  `put`'s `stats` are `IndexMap<String, Value>`; `GameRecordQuery { source, mode, patch }` with `Copy` `source`/`mode`.
- Argument shapes of the fake fns, which `Tx`'s methods pass on: strings as `&str`, row inputs as `&T`, lists as `&[T]`,
  nullable strings as `Option<&str>` (`set_display_name`, `set_in_match`).
- `cli/season_start.rs` connects with its own `PgPoolOptions` (10 connections, 10 s idle and acquire timeouts, a
  private copy of `assertPostgresUrl`) instead of calling `db::pg`'s constructor, whose Rust shape I could not know.
- `app.rs`/`api/e2e.rs` (part 18) build the E2E store with `db::fake::create_e2e_store(E2eStoreOptions { catalog:
  FakeCatalog { card_ids, is_token, is_banned }, now, redemption: None })` and reach `FakeData::reset` and
  `FakeData::grants_for` through the `Db::Fake` handle's lock. Server tests build theirs with
  `create_memory_store(MemoryStoreOptions { now, redemption })` and use `FakeData::seed_profile(json!({…}))`,
  `FakeData.tables` and `FakeData.on_call` (TS `seedProfile`, `tables`, `onCall`).
- `db::pg` (part 20 chunk 1) imports `to_public_player_summary` from here: `crate::db::fake::to_public_player_summary(
  profile_id: &str, display_name: Option<&str>, stats: &IndexMap<String, Value>, updated_at: i64) -> PublicPlayerSummary`.

## Decisions
- One fake for both TS fakes. Where they differed: R111's launch grant runs only when built with a catalog
  (`create_e2e_store`), as only `e2e-store.ts` had it; error messages are `e2e-store.ts`'s (closer to Postgres's
  constraint names); `collection_append_grants` refuses `delta == 0` for both (the e2e store's strictness); every
  method charges `on_call` (the unit fake's behaviour; `None` charges nothing, as the e2e store did).
- `createTransactionQueue` and `createInMemoryRedeem`'s promise chain are not ported: the `tokio::sync::Mutex` behind
  `Db::Fake` is FIFO-fair and held for the whole transaction, so transactions and redemptions run one at a time.
  TS's nested `tx` joined the outer one; in Rust the caller passes its `&mut Tx` (there is no nested begin: it would
  wait on its own lock). `redeem` runs inside the caller's transaction.
- `defaultRedemptionSettings(overrides)` is `default_redemption_settings()` plus struct-update syntax; the settings are
  a public field of `FakeData`, so a test flips `enabled` between calls by writing it (or through its closure).
- `memory-stores.ts`'s table-type split (`DeckTables`, `emptyDeckTables`, …) folds into one `FakeTables` with
  `Default`; `empty_tables()` is kept. `keepOnly` is `Vec::retain` and returns the count removed. TS's
  `clone`/`structuredClone` are `.clone()`. `ProfileStore.setRating` (no caller) is not ported (SURFACE §11.2).
- `jsonb_text_bytes` prints integral numbers without a fraction (as `JSON.stringify` does; serde would print `1.0`), so
  the byte cap counts what TS counted. `count_number` takes an integral `f64` as a safe integer, as JS does.
  `to_public_player_summary` reads an array-valued `cards` by index, as `Object.entries` did.
- `migrate.rs`: `MIGRATIONS` lists the 26 files by hand with `include_str!` (no build script in the server crate);
  `tests/store/migrations_pinned.rs` adds a fourth test, that `MIGRATIONS` names every `.sql` file in
  `crates/server/migrations` in order with its exact bytes and its pinned checksum, so a new file cannot be left out.
  Multi-statement SQL (the ledger, each migration) runs through `sqlx::raw_sql`; database errors are reported by
  their Postgres message alone (what `pg`'s `error.message` was), so `"<file> failed: <message>"` reads as in TS. The
  ledger's table comment now names the Rust paths (it is re-applied on every run; nothing reads it); its columns,
  the checksums and the lock are unchanged.
- `season_start.rs`: `start_season(store: &Db, deps: &SeasonDeps, options: SeasonStartOptions)`; the real run is
  begin + `open_season_in_tx` + commit (what TS's `openSeason` was); the dry run drops the transaction instead of
  throwing `ROLL_BACK`. The patch version is `jackioh_cards::catalog_version()` (TS `loadPatchVersion()` read the same
  `patches.json`). The JSON report and the stderr line are TS's.
