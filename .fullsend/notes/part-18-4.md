# Slice: part 18, chunk 4 of 6 (server 1: six of the HTTP API's test files)
BUILDS-RUN: 0

## FILES
- `crates/server/tests/api/catalog.rs` (new, ← `apps/server/test/api/catalog.test.ts`)
- `crates/server/tests/api/client_address.rs` (new, ← `client-address.test.ts`)
- `crates/server/tests/api/code_input_parity.rs` (new, ← `code-input-parity.test.ts`, now over
  `crates/engine/tests/fixtures/code-input-cases.json` by `include_str!`)
- `crates/server/tests/api/codes.rs` (new, ← `codes.test.ts`)
- `crates/server/tests/api/collection.rs` (new, ← `collection.test.ts`)
- `crates/server/tests/api/cors.rs` (new, ← `cors.test.ts`)

One `mod` per TS `describe`, one `#[tokio::test]` (or `#[test]`) per `it`, in TS order; R-ids lead
the names (`r190_b10_…`, `r163_…`, `r170_…`) so `spec check` (SURFACE §15) finds them. Each file
carries its own copy of the harness it needs (builder rule 5), so part 31 fixes a guessed name in the
same few lines of each file.

## SURFACE
- Apps are built only through SURFACE/part-1-fixed calls: `app::load_server_env(&IndexMap)` then
  `app::build(env)` under `E2E=1` (FakeStore + E2eAuth + R144 fixtures), and every request goes
  through `app::router(app)` with `tower::ServiceExt::oneshot` (so CORS, the router, the limiter
  and the handler all run). `TRUSTED_PROXY_HOPS`, `CODE_PEPPER` and `RENDER_GIT_COMMIT` are set
  through that environment, never by writing `Env` fields.
- `support::deps::test_app()` is not called: these suites vary the environment (hops, commit,
  pepper), which `test_app()` fixes; the builder they use is the same two calls. `call()` is not
  used either: these tests need raw body bytes (R145/R163 byte-identity), arbitrary headers
  (`X-Forwarded-For`, `Origin`, `X-Real-IP`) and the socket peer, which `call`'s
  `(u16, HeaderMap, Value)` signature cannot carry.
- Store values are built from TS's object literals with `serde_json::from_value(json!(…))` and read
  back with `serde_json::to_value`, as part 20.2 does, so the tests depend on TS's JSON shape, not on
  Rust field spellings. Direct field access only for `Profile.id`, `InviteCode.{id, revoked}`.
- Integer constants are compared through `as i64` / `as usize`, so their Rust widths do not matter.

## DEPENDS-ON (names I call that other parts write)
- `app` (part 18.1): `load_server_env`, `build`, `router`, `browser_origins(&Env) -> Vec<String>`,
  `ROUTES` iterable as `(method: Display, path: &str, AuthLevel, handler)` tuples, `App`'s six pub
  fields (SURFACE §11.2) with `App { catalog, ..app }` / `App { db, ..app }` legal, and
  `Arc::try_unwrap(app::build(..))` succeeding (build keeps no second `Arc<App>`).
- The router reads the peer as `axum::extract::ConnectInfo<SocketAddr>` from the request's
  extensions and treats a missing one as no peer (TS `peerAddress: null` → `UNKNOWN_CLIENT_ADDRESS`),
  rather than failing the extractor.
- `support::deps::add_user(app: &App, user_id: &str, email: &str, email_verified: bool) -> String`
  (sync; TS `FakeAuth.addUser`; part 18.1's `deps.rs`): the E2E auth must accept tokens added at run
  time, since every suite here needs many distinct callers.
- `db::fake::FakeData` (part 20.3): `reset(&mut self)`, `seed_profile(&mut self, Value)`,
  `tables.{profiles, codes, attempts, collection, grants}` (`Vec`s of `Serialize` rows in TS's JSON),
  `on_call: Option<Arc<dyn Fn(&str) -> Result<(), StoreError> + Send + Sync>>`, called with TS's
  method names (`"collection.appendGrants"`, `"collection.upsertQuantities"`), and not called by
  `seed_profile`.
- `db::store` (part 20.4): `Db::{Fake(Arc<tokio::sync::Mutex<FakeData>>), Pg(_)}`, `Db::begin(None)`,
  `Tx::{codes_log_attempt(&CodeAttempt), codes_insert(&InviteCode), commit}`, `StoreError::Other(String)`,
  `CodeAttempt`/`InviteCode` `Deserialize` from TS's literals (`InviteCode.id` accepts a UUID string).
- `api::http` (part 18.2): `client_address(&HeaderMap, Option<&str>, hops) -> String` (joins every
  `X-Forwarded-For` header), `rate_limit_address(&str) -> String` (the one-argument form; TS's
  `prefixBits` default), `UNKNOWN_CLIENT_ADDRESS`, `AuthLevel::{None, Active}`,
  `ApiError { code: ApiErrorCode, .. }: Debug`, `ApiErrorCode::BadRequest`.
- `api::catalog` (part 18.1): `Catalog: Clone` with pub `version: String`, `card_ids: Vec<String>`,
  `banned` (any `FromIterator<String>` set: the R164 ban hook) and methods `is_token(&str)`,
  `is_banned(&str)`; `catalog_from(CardDefs, &str) -> Catalog`; `version_of(&str) -> String`;
  `DEPLOYED_COMMIT_HEADER`; the deployed commit reaching `GET /api/catalog` from `RENDER_GIT_COMMIT`.
- `api::cors` (part 18.1): `with_cors(axum::Router, CorsOptions { origins: Vec<String> }) -> axum::Router`,
  `is_origin_allowed(&[String], Option<&str>) -> bool`, `VITE_DEV_ORIGINS`.
- `api::codes` (part 18.1): `mint_invite_code(&Db, &Hashes, MintInviteCodeInput { max_uses: Option<i64>,
  expires_at: Option<i64> }) -> Result<{ id: String, formatted: String }, impl Debug>` (async; the same
  guess part 20.4 made), `DEFAULT_INVITE_CODE_MAX_USES`.
- `api::crypto` (part 18.2): `create_hashes(code_key: &str, ip_key: &str) -> Hashes`, `Hashes`.
- `api::collection` (part 18.1): `grant_cards(&App, GrantInput { profile_id: String, entries:
  Vec<CollectionEntry>, reason: String }) -> Result<(), ApiError>` (async), `grant_entire_catalog(&App,
  &str, reason: &str)`, `owned_map(&App, &str) -> Result<IndexMap<String, int>, _>`, `LAUNCH_GRANT_REASON`.
- `env` (part 18.3): `load_env(&IndexMap<String, String>) -> Result<Env, impl Display>` listing every
  problem, refusing a `CATALOG_VERSION` other than `jackioh_cards::catalog_version()`; `Env` and
  `Env.trusted_proxy_hops`; `SERVER_ONLY_ENV_VARS: &[&str]`.
- `config` (part 18.3): `API_REQUESTS_PER_MINUTE`, `CODE_ATTEMPTS_PER_{PROFILE,IP}_PER_HOUR`,
  `CODE_ATTEMPT_WINDOW_SECONDS`, `DEFAULT_TRUSTED_PROXY_HOPS`, `MAX_TRUSTED_PROXY_HOPS`,
  `IPV6_RATE_LIMIT_PREFIX_BITS`, `REDEMPTION_{IDENTICAL_ERROR, RESPONSE_FLOOR_MS,
  CIRCUIT_FAILURE_THRESHOLD}`, `INVITE_CODE_{LENGTH, GROUP_SIZE, SEPARATOR, FORMAT.separator}`,
  `CODE_INPUT_MAX_LENGTH`, `API_MAX_BODY_BYTES`.
- `jackioh_engine::validator::{validate_loadout(&LoadoutInput), validate_deck(&DeckInput)}` (part 5's
  port of `packages/validator`): inputs `Deserialize` from TS's JSON, results `Serialize` as TS's
  `{ ok: true } | { ok: false, errors }` (a `Result` also reads; `issues()` takes either).

## GAPS
- Logging: the tests read `tracing` events back through a recording `Layer` installed with
  `tracing::subscriber::set_default` (current-thread `#[tokio::test]`). They expect each event's name
  in an `event` field or as the message, TS's data keys (snake_case is read as camelCase), and TS's
  `alert` level as `ERROR` (`codes.breaker_open`). `api.forwarded_for` must carry `fewestEntries`
  and `trustedProxyHops` and nothing else.
- Clocks: R107's padding must sleep on tokio's clock (`tokio::time`, SURFACE §11.3's "Timers →
  tokio::time"); the tests measure it with `tokio::time::Instant` under `start_paused` and assert
  `>= REDEMPTION_RESPONSE_FLOOR_MS`. `code_attempts.at` and `collection_grants.at` are read as
  wall-clock epoch ms (the tests seed attempts at `SystemTime::now()` and bound grant stamps by it).
- §9.4's breaker must be per `App` (TS: per router), not a static: `codes.rs`'s
  "is per-router, not module state" builds a second App over the same store and expects it closed.
- The forwarded-for minimum must be per App or per router (TS: per router); "a second router reports
  for itself" builds a second App.
- `cors.rs` builds `with_cors` around its own counting `axum::Router` (TS wrapped a counting handler);
  if 18.1 shaped CORS as a `tower` layer or `axum::middleware` fn instead, only `wrapped()` changes.
- R170 depends on the fake store's FIFO-fair `tokio::sync::Mutex` (part 20.3's notes) and on
  `resolve_caller`'s transaction being a request's first store access: the test queues behind it to
  remove the profile before the redemption's own transaction.
- Not ported (no Rust behaviour to test): catalog.test.ts's "refuses to invent a catalog when the file
  is missing or malformed" (the catalog is compiled in; `crates/cards/build.rs` is the guard), and
  R388's "serves every patch in patches.json", "shows what a patch changed" and "serves the version
  this server runs from the catalog it loaded" (SURFACE §11.3 drops `GET /api/catalog/:version`). R388
  needs its proof from the cards/tools side (`patches check`); the one R388 test kept asserts the 404.
- `collection.rs`'s "refuses a delta that is not a positive whole number" drops TS's `1.5` (an
  integer quantity cannot hold it).

## Decisions
- One builder per file (`test_env` + `load_server_env` + `build`), with R144's fixtures wiped by
  `FakeData::reset()` wherever a test reads whole tables, so tables start as empty as TS's memory
  store; `catalog.rs` keeps the fixtures and uses `e2e-token-pending` as its pending caller.
- TS's `/api/open` test route is `GET /api/catalog` (R163: `auth: none`), the real router having no
  test routes; TS's guarded stand-in route is the real `GET /api/collection` (`auth: active`).
- No `ApiLimits`: tests run on production constants. The breaker tests seed
  `REDEMPTION_CIRCUIT_FAILURE_THRESHOLD - k` neighbour failures (distinct profiles and IP hashes)
  instead of shrinking the threshold to `k`; R107's 150 samples clear `tables.attempts` after each
  sample instead of raising the threshold.
- No `Ids`: `codes.rs` mints with the real `mint_invite_code` and finds codes by id/hash;
  `code_input_parity.rs` writes the one code a row needs straight into `invite_codes` (hashed with
  `${CODE_PEPPER}:code`, `DEFAULT_INVITE_CODE_MAX_USES` uses), as TS's `idsMinting` made the mint do.
- Hashes the tests compare are computed by a private HMAC-SHA256 over `sha2` (no MAC crate API), keyed
  `${CODE_PEPPER}:code` / `${CODE_PEPPER}:ip` as `index.ts` keys them; this pins the derivation every
  stored invite-code hash depends on.
- R107's cost model is tallied, not slept: the fake's `on_call` hook is synchronous and cannot move
  tokio's clock, so the per-call 3 ms and the 11 ms row fetch (charged when the looked-up code's row
  exists) are counted per sample; elapsed times are tokio's paused clock.
- `catalog.rs`'s loadout-validator tests hand `jackioh_engine::validator` the snapshot a handler builds
  from the catalog handle (`snapshot`: version, catalog.json's defs, the handle's banned ids), the job
  `loadout-validator.ts` did; R164's banned catalog is `Catalog { banned, ..catalog.clone() }`.
- `describe("catalog")` is `mod loaded_catalog` (a `mod catalog` in `catalog.rs` is clippy's
  `module_inception`).
- B5's per-row `it`s are two looping tests (rows with and without a canonical code), each assertion
  naming its row; every row still gets a fresh server, profile and address.
- "lets the environment pin the version instead" became the Rust contract: the App's version is the
  compiled-in one, and `load_env` refuses any other `CATALOG_VERSION`.
