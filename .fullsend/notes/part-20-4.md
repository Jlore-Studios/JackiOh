# Slice: part 20, chunk 4 of 5 (persistence: the store port, the stats CLIs, their tests, render.yaml, four SQL files)
BUILDS-RUN: 0

## FILES
- `render.yaml` (change): `runtime: docker`, `dockerfilePath: crates/server/Dockerfile`, `dockerContext: .`;
  buildCommand, startCommand, NODE_VERSION, CYPRESS_INSTALL_BINARY gone; name, plan, region,
  branch, healthCheckPath, NODE_ENV, CATALOG_VERSION (same `value: v0.2.11` line, so `patches ship`
  still bumps it), TRUSTED_PROXY_HOPS and the five `sync: false` secrets kept; comments rewritten.
- `crates/server/src/db/store.rs` (new, ← `apps/server/src/api/ports.ts`): `StoreError`,
  `StoreResult`, every row and input type, `Db` (`Pg`, `Fake`; `begin`, `fake`, `close`), `Tx`
  (`Pg`, `Fake`; `commit` and one method per TS Store method: 87, all but `tx` and `profiles.setRating`), `StartMatchInput`,
  `MatchSeat`.
- `crates/server/src/cli/card_stats.rs` (new, ← `src/db/card-stats.ts`): `CardStatsOptions`,
  `parse_card_stats_args`, `card_stats_report`, `render_card_stats`, `run`.
- `crates/server/src/cli/import_dev_records.rs` (new, ← `src/db/import-dev-records.ts`):
  `ImportOutcome`, `run_file_path`, `import_dev_records`, `run`.
- `crates/server/tests/store/{card_stats,mint_code,seed_accounts,seed_catalog}.rs` (new, ← the TS
  tests of the same names; `seed_catalog.rs` holds both `seed-catalog.test.ts` and `.spec.ts`).
- `crates/server/tests/sql/{03_match_lifecycle,05_tutorial_progress,09_catalog_growth,13_glitch}.sql`
  (copied, `cmp`-identical).

## SURFACE
- §11.2's `Db`/`Tx` exactly: `Db::Pg(sqlx::PgPool)`, `Db::Fake(Arc<tokio::sync::Mutex<fake::FakeData>>)`,
  `Tx::Pg(sqlx::Transaction<'a, Postgres>)`, `Tx::Fake(fake::FakeTx<'a>)`,
  `Db::begin(&self, claim_sub: Option<&str>) -> Result<Tx<'_>, StoreError>`,
  `Tx::commit(self) -> Result<(), StoreError>`. Method names `<substore>_<method>` snake_cased
  (`player_settings_merge`, `last_boards_sample_others`, `game_records_list`, …), root `redeem` and
  `purge_expired`; `profiles_set_rating` not ported (§11.2).
- Every method body is `dispatch!`: `Tx::Pg(t) => pg::<name>(t, …).await`, `Tx::Fake(f) => fake::<name>(f, …)`
  (the fake sync, as §11.2 writes it).

## DEPENDS-ON (names I call that other parts write)
- `crate::db::pg::<method>(t: &mut sqlx::Transaction<'_, sqlx::Postgres>, …) -> Result<T, StoreError>`
  (async), and `crate::db::fake::<method>(f: &mut fake::FakeTx<'_>, …) -> Result<T, StoreError>`
  (sync), for every `Tx` method, with exactly the argument types of `store.rs` (part 20's pg.rs and
  fake.rs chunks).
- `crate::db::fake::begin(&Arc<tokio::sync::Mutex<FakeData>>) -> FakeTx<'_>` (async: it awaits the
  lock), `crate::db::fake::commit(FakeTx<'_>)` (sync, returns `()`), `FakeData: Default`, and the
  table `FakeData.game_records: Vec<GameRecord>` (read by `tests/store/card_stats.rs`).
- `crate::ranked::glicko2::Glicko`, `crate::ranked::ladder::{SeasonRank, VisibleRank}`,
  `crate::ranked::season::{ResetChange, ResetPlayer}` deriving `Serialize, Deserialize, Clone, Debug,
  PartialEq` (part 18).
- `jackioh_engine::wire::{GameRecord, GameSource (::Dev), GameMode, SourceFilter, PilotFilter, Pilot,
  PortraitId, CardStatsFilter { source, mode, patch, pilot } (Clone), CardStatsReport { games, cards:
  Vec<CardStats { card, … }> } (Serialize), DEFAULT_CARD_STATS_FILTER, SOURCE_FILTERS, GAME_MODES,
  PILOT_FILTERS (each `&[T]`, T: Copy + Serialize), card_stats(&[GameRecord], &CardStatsFilter) ->
  CardStatsReport, format_card_stats(&CardStatsReport, &dyn Fn(&str) -> Option<String>) -> String,
  parse_game_record_lines(&str) -> Result<Vec<GameRecord>, impl Display> (message "line N: …"),
  DEV_RECORD_ID_PREFIX: &str}` (part 5, `wire/stats.rs` and `wire/emotes.rs`).
- Tests only: `cli::mint_code::parse_mint_args(&[String]) -> Result<MintOptions { max_uses: Option<_>,
  expires_in_days: Option<_> }, impl Display>`; `api::codes::{mint_invite_code(&Db, &Hashes,
  MintInviteCodeInput) -> Result<{ id, formatted }, impl Debug>, MintInviteCodeInput { max_uses:
  Option<i64>, expires_at: Option<i64> }: Default}`; `api::crypto::{create_hashes(code_pepper: &str,
  ip_pepper: &str) -> Hashes, Hashes::code(&self, &str) -> String, Hashes::ip(&self, &str) -> String}`;
  `cli::seed_accounts::{seed_accounts_settings(source: &IndexMap<String, String>, supabase_url: &str,
  node_env: &str) -> Result<{ password: String }, impl Display>, SEED_PROJECT_VAR, SEED_PASSWORD_VAR}`;
  `config::AUTH_PASSWORD_MIN_LENGTH` (an integer); `cli::seed_catalog::{read_catalog(&str) (async) ->
  Result<Vec<CatalogEntry { id, set, tags: Vec<String>, token: bool, … }: Clone>, impl Display>,
  seed_catalog(connection_string: &str, catalog_version: &str, &[CatalogEntry]) (async) ->
  Result<usize, impl Display + Debug>}`.

## GAPS
- `crates/server/src/db/mod.rs` (part 1) declares the four modules but re-exports nothing, while
  SURFACE §11.2 writes `db::Profile` (in `Caller`), `db::Db` (in `App`). Either part 31 adds
  `pub use store::*;` to `db/mod.rs`, or callers write `db::store::Profile`. My files use
  `crate::db::store::…`.
- The test signatures under DEPENDS-ON "Tests only" are guesses at other chunks' and part 18's Rust
  (`mint_invite_code` taking `&Db`, `create_hashes` taking two `&str`, `seed_accounts_settings`
  taking the two env values as `&str`, `read_catalog`/`seed_catalog` async). Part 31: reconcile the
  call sites in `tests/store/{mint_code,seed_accounts,seed_catalog}.rs`, not the behaviour.
- `pg.rs` must map Postgres 23505 on `results_pkey` to `StoreError::Duplicate(match_id)` (TS
  `DuplicateResultError`), the variant `results.rs` retries on.
- `test:db` runs the Postgres half of `tests/store/seed_catalog.rs` (gated on `DATABASE_URL`) in the
  same binary as the store contract; both truncate shared tables, so `tests/db/run.sh` must run the
  binary with `--test-threads=1` (vitest ran spec files one at a time).
- `read_catalog` on the real catalog is asserted to keep the file's set order (Core, Classic,
  Classic+), as TS's test does; a `seed_catalog.rs` that parses the record into `serde_json::Value`
  (a sorted `BTreeMap` without `preserve_order`) fails it, so it must parse into `IndexMap`.
- Not touched (other chunks of part 20): `package.json`, `crates/server/Dockerfile`, `pg.rs`, `fake.rs`,
  `migrate.rs`, the other CLIs and tests.

## Decisions
- Argument conventions for every `Tx` method: `&str` for strings, `&[T]` for arrays, `&Row`/`&Input`
  for objects, `Option<&str>` for `string | null`, plain values for numbers and string unions.
  Integers are `i64` (epoch ms, counts, caps, `seq`, `max_uses`, positions, `games`), ratings `f64`
  (migration 0022's `double precision`), `RatedGameRow.winner_side` `Option<usize>` (an index).
- TS `x?: T | null` (three states) is `Option<Option<T>>` with the `absent_or_null` serde helper
  (`None` absent, `Some(None)` null): `Profile.display_name`, `ProfileCreateInput.display_name`,
  `FrozenDeck.portrait`, `Room.host_portrait`, `Ticket.portrait`. `x?: T` is `Option<T>` with skip;
  `x: T | null` is `Option<T>` serialised `null` (SURFACE §4.3).
- One type where TS had the same literals twice: `SeriesSeat = PlayerId` (alias, so
  `SeriesSeat::P1` still reads), series winners are `wire::Winner`, `Pilot` and `LastBoardEntry` are
  re-exported from the engine. `QueueMode` stays its own enum (TS keeps it apart from `GameMode`).
- `Record<QueueMode, number>` is `PerMode<T> { bo1, bo3, random }` with `Index<QueueMode>`.
- TS anonymous types named: `ProfileCreateInput`, `PlayerStatsListOptions`, `FavouriteCard`,
  `FunStats`, `RatedGameKind`; `GameOverReason | SeriesEnd` is the untagged `RatedReason`;
  `SeasonRank & { rating }` is `SeasonStanding { #[serde(flatten)] rank, rating }`.
- String unions use a local `store_union!` (the engine's `string_union!` is `pub(crate)`): serde
  literal, `as_str`, `FromStr` (error `StoreError::Other`), `Display`, `ALL`.
- `PlayerSettingValue` is untagged `Bool | Number(serde_json::Number) | Text`, so `1` round-trips as
  `1`.
- `StoreError { Duplicate(String), Db(#[from] sqlx::Error), Other(String) }` with `From<String>`,
  `From<&str>`, `From<serde_json::Error>`.
- Dropped from ports.ts, with where each went written in store.rs's header: `Timers`, `Logger`,
  `Ids`, `Hashes`, `ServerConfig`, `ApiLimits`, `CatalogInfo`, the loadout-validator types,
  `AuthUser`/`AuthSession`/`AuthProvider` (auth.rs), `MatchDirectory` (Registry), `GameRecorder`
  (direct `summarize_game`), `ServerDeps` (App). `ApiLimits`' "enforced by the store" note moved
  onto `redeem`'s doc. Kept: `StartMatchInput`, `MatchSeat` (data `Registry::start` takes).
- `Db::begin` issues `BEGIN` then store.ts's one `SESSION_SQL` (`set_config('role', 'service_role',
  true)`, `set_config('request.jwt.claim.sub', $sub, true)`), `None` binding `""` as TS's `subject ??
  ""`. `ACTING_ROLE`/`SESSION_SQL` are private copies in store.rs. Rollback is dropping the `Tx`.
  Added `Db::fake()` (an empty fake) and `Db::close()` (TS `PostgresStore.close()`).
- The two CLIs take `&Db` (TS's root `Store`) and open their own transactions; `import_dev_records`
  commits one transaction per record, as TS's one store call per record did. They connect with
  `PgPoolOptions::new().max_connections(1)` inline (TS `max: 1`) rather than through pg.rs, and name
  cards from `jackioh_cards::CATALOG` (TS `loadCatalog()`). Usage strings name `jackioh-server
  stats-cards`/`stats-import`; the `stats:import:` output line is TS's verbatim.
- TS's regexes are hand-written checks (no regex crate): `--name=value` parsing, the tests'
  `/\s{2,}/` split and the migrations' `cards_tags_check` scan.
- `run_file_path` returns a `PathBuf` (Node `path.resolve`: absolute wins, `.`/`..` folded) and
  still honours `INIT_CWD`.
- Tests build `GameRecord`s from TS's object literals with `json!` + `serde_json::from_value`, and
  compare filters and tallies as JSON, so they do not depend on part 5's Rust field or variant
  names. The Postgres half of the seed-catalog test runs its two TS `it`s in order inside one
  `#[tokio::test]` (truncate, seed, reseed, restore the fixture catalog).
- render.yaml keeps `NODE_ENV` (the E2E refusal reads it) and states that the server refuses to boot
  when `CATALOG_VERSION` differs from the compiled-in version (SURFACE §11.3).
