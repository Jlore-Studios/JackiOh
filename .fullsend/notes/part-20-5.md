# Slice: part 20, chunk 5 of 5 (server 3: the Docker image and its rehearsal, test:db, three CLIs, three store tests, four SQL copies)
BUILDS-RUN: 0

(`sh -n` was run on the two shell scripts, a syntax check only; no cargo, rustc, pnpm, tsc or test runner.)

## FILES
- `crates/server/Dockerfile` (new, SURFACE §11.3): `rust:1.97-slim` builds `cargo build --release --locked -p
  jackioh-server` from `Cargo.toml`, `Cargo.lock` and `crates/` only; `debian:bookworm-slim` with `ca-certificates`
  and `tini` runs `/app/jackioh-server release` as a non-root user (`ENTRYPOINT [tini, --, /app/jackioh-server]`,
  `CMD ["release"]`, `ENV PORT=8787`).
- `crates/server/tests/db/bootstrap.sql` (new): `test/db/bootstrap.sql`, statements unchanged, path comments repointed.
- `crates/server/tests/db/run.sh` (new, 755): `test/db/run.sh` over cargo: build first, Postgres 16 in Docker,
  bootstrap, `cargo run -p jackioh-server -- migrate`, grants, then `cargo test -p jackioh-server --test server
  store:: -- --test-threads=1` with `DATABASE_URL`.
- `crates/server/tests/deploy/rehearse.sh` (new): `test/deploy/rehearse.sh` over the Docker image (below).
- `crates/server/src/cli/mint_code.rs`, `seed_accounts.rs`, `seed_catalog.rs` (new): the three TS scripts whole.
- `crates/server/tests/store/postgres.rs`, `redeem_race.rs`, `season_start.rs` (new): the three TS tests.
- `crates/server/tests/sql/{03b_legacy_loadout_seed,06_account_deletion,10_last_boards,14_patch_retcon}.sql`: `cp`,
  `cmp`-identical.

## SURFACE
- §11.2: every TS store call outside `store.tx` is `once!` (begin with the call's subject, the call, commit; an
  error drops the `Tx`, which rolls back); a TS `store.tx` is one `begin(None)` … `commit`. Rows go in as
  `serde_json::from_value(json!({…TS literal…}))` with the type inferred from the method's parameter, and come out
  as `serde_json::to_value`, so the tests read TS's JSON, not Rust field names.
- Argument shapes follow chunk 4's `store.rs` (its notes): `&str`, `&Row`, `&[T]`, `Option<&str>`, unions by value,
  integers `i64` (caps too).
- §11.3: the image is what render.yaml (chunk 4) names; `CATALOG_VERSION` is compared, not derived.

## DEPENDS-ON (names I call that other parts write)
- `db::store` (chunk 4): `Db::{Pg(PgPool), Fake(Arc<tokio::sync::Mutex<FakeData>>)}`, `Db::begin(&self,
  Option<&str>)`, `Tx::commit`, `StoreError: Display`, `MatchActionRow`, and the `Tx` methods profiles_{get_by_id,
  get_by_user_id, get_many(&[String]), create(&ProfileCreateInput), set_status(&str, ProfileStatus),
  set_in_match(&str, Option<&str>)}, codes_{insert, find_by_hash, claim(&str, i64), log_attempt(&CodeAttempt),
  count_attempts_by_profile, count_failures}, redeem(&RedeemInviteCodeInput), collection_get,
  decks_{upsert(&SavedDeck, i64), list, get, remove}, trios_upsert(&SavedTrio, i64), tickets_{insert, get,
  claim_pair(&str, &str, &str, i64)}, rooms_{create, get, claim(&str, &str, &str, i64)}, matches_{create, get,
  finish, live, discard_open, append_actions(&[MatchActionRow]), actions}, ranked_seasons. Every row type
  `Serialize + Deserialize` with TS's camelCase JSON.
- `db::fake` (chunk 3): `FakeData::seed_profile(&mut self, Value)` (TS `seedProfile`), reached through the
  `Db::Fake` lock in `season_start.rs`.
- `db::pg` (chunk 1): each method re-stamps `request.jwt.claim.sub` with its own subject inside a joined
  transaction, as TS's `joinedSession` did; `postgres.rs`'s "stamps each statement's own subject inside one
  transaction" fails otherwise (the subject given to `Db::begin` is only the first).
- `env::load_env(&IndexMap<String, String>) -> Result<Env, E: Display>`, `Env.{database_url, code_pepper,
  supabase_url, supabase_secret_key, node_env, catalog_version}`, each with `.as_str()` (a `String`, or for
  `node_env` an enum with `as_str`) (part 18).
- `api::codes::{mint_invite_code(&Db, &Hashes, MintInviteCodeInput) -> Result<{ id, formatted }, E: Display>,
  MintInviteCodeInput { max_uses: Option<i64>, expires_at: Option<i64> }}` and `api::crypto::create_hashes(code_pepper:
  &str, ip_pepper: &str) -> Hashes` — the same guesses chunk 4's `tests/store/mint_code.rs` makes, so part 31 fixes one
  shape (part 18).
- `api::loadout_validator::TRIO_DECKS`, `config::{AUTH_PASSWORD_MIN_LENGTH, AUTH_PASSWORD_MAX_LENGTH, MAX_SAVED_DECKS,
  MAX_SAVED_TRIOS, CODE_ATTEMPTS_PER_IP_PER_HOUR, SEASON_RESET_STRENGTH (f64)}` (any integer type an `as` cast or a
  `0..N` range takes) (part 18).
- `api::ranked::{SeasonDeps { patch_version: String }, rate_ranked_game(&mut Tx<'_>, &App, &RankedGameInput)}`
  (part 18); `cli::season_start::{parse_season_start_args(&[String]) -> Result<SeasonStartOptions, E: Display>,
  SeasonStartOptions { dry_run }, start_season(&Db, &SeasonDeps, SeasonStartOptions) -> Result<OpenedSeason>}` with
  `OpenedSeason: Serialize` (chunk 3).
- `tests/support/deps.rs` (part 18): `test_app() -> Arc<App>` with `App.db` a `Db::Fake`, and
  `TEST_PATCH_VERSION` (TS `"v0.1.1"`).
- `jackioh_cards::{catalog_json(), catalog_version()}` (part 1).
- The migrate CLI (chunk 3) prints `migrate: nothing to do…` when nothing is pending, which the rehearsal greps.
- The server (part 18) refuses to boot when `CATALOG_VERSION` is set and differs from `catalog_version()`, exits
  non-zero, and names the stale value in what it prints: the rehearsal's last boot asserts all three.

## GAPS
- Not ported: postgres.spec.ts's "src/index.ts's loadStore finds this module, and what it loads works" (it ran
  `test/db/loadstore-boot.ts` under `tsx`): `loadStore` and its dynamic import are gone (SURFACE §11.3), so there is
  no boot path to find; `tests/deploy/rehearse.sh`'s boots cover "the server builds a working PgStore".
- `api::codes::mint_invite_code` / `api::crypto::create_hashes` / `api::ranked::rate_ranked_game` /
  `api::ranked::SeasonDeps` shapes above are guesses at part 18's Rust (TS's `MintDeps`, `Hashes`, `SeasonDeps`
  were `Pick`s of `ServerDeps`, whose Rust form is `App`).
- `tests/store/redeem_race.rs` builds its fake end with `test_app()` (frozen by SURFACE §11.2) rather than
  `fake::create_e2e_store` with the contract's fixture catalog: the race reads no catalog, and each case builds a
  fresh app, which is TS's `reset()`. It uses fresh uuids for user ids on both ends (TS's memory harness used
  `user-<n>`), so the E2E fixture accounts cannot collide.
- Lines over rustfmt's 110 columns in a few places: formatting only (part 31/35 run `cargo fmt`).

## Decisions
- `seed-catalog` with no argument seeds `jackioh_cards::catalog_json()` at `catalog_version()`, and refuses (before
  writing) when `CATALOG_VERSION` is set to anything else, with the message naming both; with a path it seeds that
  file at `CATALOG_VERSION`, required as in TS. `parse_catalog(text, path)` is split out of `read_catalog` so the
  compiled-in catalog meets the same schema check; a record is re-read into `IndexMap` so entries keep the file's
  order (`Object.entries`), which `tests/store/seed_catalog.rs` checks (sets in order Core, Classic, Classic+).
- The CLIs connect with sqlx directly (`PgPoolOptions::max_connections(1)` for mint-code, as TS `max: 1`; a single
  `PgConnection` for seed-catalog and seed-accounts, as TS's `Client`). `DATABASE_URL` for seed-catalog is read from
  the environment directly, as TS did (mint-code's header says why mint-code does not).
- TS's SQL text is kept verbatim. Where `pg` let Postgres infer a parameter's type and sqlx would send `text`
  (`uuid = text` fails), the value is bound as `uuid::Uuid` instead of adding a cast; the `app.upsert_*` calls
  already carried TS's casts and bind strings.
- `Number(raw)` (mint-code's flags, seed-accounts' count) is a private `js_number` (trim, empty = 0, Infinity,
  0x/0o/0b, decimal literals, else NaN) with `Number.isInteger`. Option values are `i64`. An `--expires-in-days`
  that overflows epoch ms is refused before minting (JS would have minted and then thrown in `toISOString`).
- `seed_accounts_settings(source, supabase_url, node_env)` takes TS's `Pick<ServerEnv, …>` as its two values
  (chunk 4's test calls it so) and returns `SeedAccountsSettings { password }`. The password check counts UTF-16
  units for the minimum (TS `.length`) and UTF-8 bytes for the maximum (TS `TextEncoder`). The file holds no
  `password: "` or `const PASSWORD = "` text (the TS test's literal check); the local is named `secret`.
  `SUPABASE_URL`'s host comes from `reqwest::Url` (the url crate's type, re-exported); no new dependency.
- The profile-row poll (20 tries, 250 ms) and the admin listing's page size (200) are named constants in the file.
- `postgres.rs`'s concurrency cases ("only one of two simultaneous joiners …") race on a second pool of two
  connections; TS raced on its one-connection store, which serialised them on the pool. Two connections is what
  the describe's "under genuine concurrency" says; the expectation (exactly one winner) is unchanged.
- `postgres.rs` and `redeem_race.rs` each carry private copies of the harness pieces they use (`seed_cards`,
  `TRUNCATE`, `database_url`, the fixture ids), a `once!` macro and a 15-line `join_all` (no `futures` crate): every
  future is created before any is polled, so the calls overlap as `Promise.all`'s did. Pg cases return at once
  without `DATABASE_URL`; `run.sh` runs the binary's `store::` cases with `--test-threads=1` (one shared database,
  as vitest's `fileParallelism: false`).
- `r250_applies_app_settings_cap…` restores `max_saved_decks` before asserting (TS's `finally`).
- `season_start.rs`: `seedProfile` is `FakeData::seed_profile` through the lock (fixed ids `profile-a`/`-b`, as
  TS); TS's literal `"v0.1"` is `season_id_of(TEST_PATCH_VERSION)` with a private copy of `seasonIdOf`.
- Test names: TS `it` titles snake_cased with leading R-ids kept (`r250_…`, `r111_…`, `b14_…`); a `describe` that
  cites rulings is a `mod` with them leading (`r250_r252_app_upsert_deck_and_app_upsert_trio`,
  `r263_matches_discard_open`, `r672_profiles_current_match_id`).
- The rehearsal reads render.yaml's `runtime`, `dockerfilePath`, `dockerContext` and env values, fails if a
  `buildCommand` or `startCommand` is still there, builds the image, and boots it (container memory capped at the
  free instance's 512 MB, `docker stats` under 450 MB) on a shared Docker network with Postgres (or `--network
  host` for `REHEARSAL_PGHOST`): first boot migrates and reseeds at render.yaml's version, second migrates nothing,
  and a third with a stale `CATALOG_VERSION` must exit non-zero without serving and leave the database stamped
  with render.yaml's version (TS's "the start command overwrites a stale value" became "the server refuses it").
  The served version is read with `grep -o` (no Node at run time).
- The Dockerfile leaves `rust-toolchain.toml` out of the build (it would make rustup fetch clippy, rustfmt and the
  wasm target) and copies only the manifest, the lock file and `crates/`, so a context holding `node_modules` is
  never read.
