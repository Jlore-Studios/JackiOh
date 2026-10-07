# Slice: part 18 (server 1), chunk 1 of 6: app, auth, and the auth/catalog/codes/collection/cors routes
BUILDS-RUN: 0

## FILES
- `crates/server/src/auth.rs`: `Auth { Supabase(SupabaseAuth), E2e(E2eAuth) }` with `verify`, `sign_in`,
  `delete_user`, `can_delete_users`; `AuthUser`, `Session` (+ alias `AuthSession`), `AuthError
  { Invalid, Rejected(String), Unavailable(String) }`; SupabaseAuth = TS `createSupabaseAuth` (JWKS via
  reqwest + jsonwebtoken with jose's cache/cooldown, HS256 tier 2, tier 3 `/auth/v1/user`, R194 session
  cache, R665 aal2 + `mfa_enrolled` memory, 30 s confirmed-email cache, admin getUserById/deleteUser);
  E2eAuth = TS `createE2EAuth` + test `createFakeAuth` (`add_user`, `set_email_verified`, `set_deletion`).
- `crates/server/src/app.rs`: `App` (+ `breaker`), `now_ms`, `load_server_env`, `browser_origins`,
  `HandlerFuture`/`Handler`/`Route`, `h!`, `ROUTES` (37), `all_routes`, `build`, `router`, `serve`.
- `crates/server/src/api/{auth,catalog,codes,collection,cors}.rs`.
- `crates/server/tests/support/deps.rs`: `test_app`, `test_app_with(TestAppOptions)`, `test_env`,
  `call`, `call_from`, `send`, `json_request`, `with_peer`, `read_json`, `create_test_catalog`,
  `fake_auth`/`add_user`/`set_email_verified`, `record_logs` (tracing layer → `LogEntry {level, event, data}`),
  `P1_TOKEN`/`P2_TOKEN`/`PENDING_TOKEN`, `TEST_CATALOG_VERSION`, `DEFAULT_TEST_IP`.
- `crates/server/tests/export_config.rs`: writes `apps/web/src/wire/serverConfig.ts` (42 constants, sorted).

## SURFACE
Matched §11.2's shapes, with these choices where SURFACE leaves a gap:
- `Route = (&'static str /*method*/, &'static str /*path*/, AuthLevel, Handler)` (a tuple, as §11.2
  writes it), `Handler = for<'a> fn(&'a App, Req) -> HandlerFuture<'a>`, `HandlerFuture<'a> =
  Pin<Box<dyn Future<Output = ApiResult> + Send + 'a>>`, all in `app.rs`. Every handler future must be Send.
- Handler names for TS's anonymous `route(...)` closures (TS name snake_cased where TS had one —
  `actor::rooms::{create, join}`, `api::queue::enqueue` from SURFACE's example — else invented):
  auth `sign_in`, `get_profile`, `delete_account`, `get_me`; catalog `get_catalog`; codes `redeem`,
  `get_status`; collection `get_collection`; decks `list_decks`, `put_deck`, `delete_deck`, `put_trio`,
  `import_trio`, `delete_trio`; queue `enqueue`, `dequeue`, `population`; series `get_series`, `pick`,
  `forfeit`, `match_series`; tutorial `get_tutorial`, `put_tutorial`; ranked `get_ranked`,
  `get_leaderboard`, `get_match_ranks`; rematch `offer_rematch`, `get_rematch`; settings `get_settings`,
  `put_settings`; stats `get_cards`, `get_card`, `get_player`, `put_player`, `get_players`.
- `router(app)`: `/ws/match` (any method) → `actor::ws_server::handle` for an `Upgrade: websocket`
  request, else the API; fallback → `api::cors::with_cors` → `api::http::dispatch(&app, ROUTES, request)`.
  `serve` uses `into_make_service_with_connect_info::<SocketAddr>()`, so the peer address is a
  `ConnectInfo<SocketAddr>` extension (absent under `oneshot`; `deps::with_peer` adds one).
- `call(app: impl AppHandle /*Arc<App> or &Arc<App>*/, method, path, token: Option<&str>, body: Value)`:
  body `Value::Null` sends none; writes `x-forwarded-for: 203.0.113.7` and runs with
  `TRUSTED_PROXY_HOPS=1`, as TS's `jsonRequest`/`createTestDeps`.
- `Req.address` read as the client address's IP hash (TS `req.ipHash`).

## DEPENDS-ON (called, written by other parts)
- `api::http` (18.x): `Req`, `Caller {profile, user}`, `AuthLevel::{None, User, Active}`, `ApiResult`,
  `ApiError` built by struct literal `{code, message, details, retry_after_ms}`, `ApiErrorCode::{BadRequest,
  Unauthorized, EmailUnverified, AccountBanned, Conflict, AlreadyInMatch, InvalidCode, RateLimited,
  Unavailable, Internal}`, `json(u16, Value) -> Response`, `RateLimiter`,
  `create_rate_limiter(limit, window_ms)` (args cast `as _`), `dispatch(&Arc<App> or &App, &[app::Route],
  axum::extract::Request) -> Response` (async).
- `env` (18.x): `Env` fields `e2e`, `public_origins: Vec<String>`, `catalog_version`, `deployed_commit:
  Option<String>`, `database_url`, `supabase_url`, `supabase_secret_key`, `supabase_jwks_url` (String),
  `supabase_jwt_secret: Option<String>`, `code_pepper: String`, `port` and `trusted_proxy_hops` (Display);
  `load_env(&IndexMap<String, String>) -> Result<Env, impl Display>`.
- `config` (18.x): the constants named in my files and the 42 in export_config.rs; struct consts
  `INVITE_CODE_FORMAT {alphabet, length, group_size, separator, max_input_length}` and
  `CATALOG_NUMBER_SET_OFFSETS {core, classic, classic_plus}`.
- `api::crypto` (18.x): `normalize_code`, `canonical_invite_code -> Option<String>`, `is_well_formed_code(&str, len)`,
  `format_code`, `random_code(len)`.
- `api::e2e::seed_e2e_fixtures(&App).await -> Result<_, impl Debug>` (18.x). Its fixture code hashes must
  equal `api::codes`'s private `code_hash`: HMAC-SHA256 hex, key `"{CODE_PEPPER}:code"`, over
  `normalize_code(plain)` (TS `createHashes`).
- `api::ranked::open_season(&App).await -> Result<{season: {id}, opened, reset}, impl Debug>` (18.x).
- `api::retention::run_purge(Arc<App>)` (18.x) and `api::results::run_reaper(Arc<App>)` (19) carry TS
  `index.ts`'s two loops: purge at boot then every `RETENTION_PURGE_INTERVAL_SECONDS`, logging
  `retention.purged {codeAttempts, matchActions}` when either is > 0 and `retention.purge_failed {message}`;
  reaper every `MATCH_REAPER_INTERVAL_SECONDS`, logging `matches.reaped {ids}` when non-empty and
  `matches.reaper_failed {message}`. `api::queue::run_matchmaker`, `api::series::run_sweeper` (19).
- Part 19: `actor::registry::Registry::new()`, `actor::ws_server::handle(Arc<App>, Request) -> Response`,
  the handlers above.
- Part 20: `db::store::{Db::{Pg, Fake}, Tx, StoreError, Profile {id, status, in_match_id, ..}, ProfileStatus
  {Pending, Active, Banned} (Serialize), InviteCode {id, code_hash, max_uses: i32, uses, revoked, expires_at,
  created_at}, RedeemInviteCodeInput {profile_id, code_hash: Option<String>, ip_hash}, RedeemResult {Ok,
  NotPending, EmailUnverified, RateLimitedProfile, RateLimitedIp, CircuitOpen, InvalidCode}, CollectionEntry
  {card_id, quantity: i32}, CollectionGrant {profile_id, card_id, delta, reason, at}}`; `db::fake::FakeData:
  Default`; `Db::begin(Option<&str>)`, `Tx::commit`; Tx methods `results_record_for(&str) -> {wins, losses,
  draws}`, `series_active_for(&str) -> Option<{id}>`, `profiles_remove(&str)`, `redeem(&RedeemInviteCodeInput)`,
  `codes_insert(&InviteCode)`, `codes_count_failures(i64)`, `codes_count_attempts_by_profile(&str, i64)`,
  `codes_oldest_attempt_at_by_profile(&str, i64) -> Option<i64>`, `collection_get(&str) -> Vec<CollectionEntry>`,
  `collection_upsert_quantities(&str, &[CollectionEntry])`, `collection_append_grants(&[CollectionGrant])`.

## Provided for other parts
- `app::now_ms() -> i64`: the server clock (wall clock anchored once, advanced by tokio's, so
  `tokio::time::pause/advance` moves it). Every module that read `deps.timers.now()` should use it.
- `api::collection::caller_profile(&Req) -> Result<Profile, ApiError>` (cloned), `owned_map(&App, &str) ->
  Result<IndexMap<String, i32>, ApiError>`, `grant_cards`, `grant_entire_catalog(&App, &str, Option<&str>)`,
  `LAUNCH_COPIES`, `LAUNCH_GRANT_REASON`.
- `api::catalog::{Catalog {version, defs: CardDefs, card_ids, commit}, Catalog::is_token/is_banned/defs_json,
  catalog_from, load_catalog(LoadCatalogOptions {version, json}), version_of, load_current_patch() ->
  Result<String, String>, load_patch_versions() -> Vec<String>, DEPLOYED_COMMIT_HEADER}` (loaders async, as TS).
- `api::codes::{mint_invite_code(MintDeps {db, code_pepper}, MintInput {max_uses, expires_at}) ->
  Result<MintedCode {id, formatted}, MintError>, redeem_code, BreakerState, create_breaker_state,
  DEFAULT_INVITE_CODE_MAX_USES}` (cli/mint_code.rs uses the first).
- `api::cors::{with_cors(&CorsOptions, Request, FnOnce(Request) -> Fut) -> Response, is_origin_allowed(&[String],
  Option<&str>), VITE_DEV_ORIGINS, CorsOptions {origins}}`; `app::browser_origins(&Env)` for the WS Origin check.

## GAPS
- **Manifest**: jsonwebtoken 11.1 has no crypto provider by default and panics on the first verify;
  the workspace dependency needs `features = ["rust_crypto"]` (or `aws_lc_rs`). Part 31 adds it.
- **SURFACE §11.2 App**: added `pub breaker: std::sync::Mutex<api::codes::BreakerState>` (R106's breaker
  lived in `createCodesRoutes()`'s closure; per-app state has no other home). Only `app::build` and
  `tests/support/deps.rs` construct `App`.
- SURFACE writes `db::Db` / `db::Profile`; `db/mod.rs` re-exports nothing, so I wrote `db::store::Db`,
  `db::store::Profile` (and the other row types) — part 31 either adds `pub use store::*;` or keeps these.
- If `api::http::dispatch` reads `crate::app::ROUTES` itself instead of taking the table, drop the
  argument in `app::serve_api`; if `Route` becomes a struct in http.rs, `ROUTES` needs the struct literal.
- `api::catalog::get_catalog` builds its response by hand (to serve the precomputed `{version, defs}`
  bytes in catalog order: serde_json has no `preserve_order` here) with `json()`'s two headers.
- Not ported, list for part 31: TS `toSession`, `AuthApiSession`, `AuthApiResult`, `PasswordAuthClient`,
  `createRealClients`' password half, `signUp` (the server-side password path; SURFACE §11.3);
  `catalogUrl`, `patchesUrl`, `readSnapshot`, `catalogAtVersion`, `PATCH_VERSION`, the snapshot cache
  (`/api/catalog/:version`, dropped); `create*Routes()` (the table is `app::ROUTES`); `loadStore`,
  `StoreUnavailableError`, `createRuntime`, `Runtime`, `RunningServer` (SURFACE §11.3); deps.ts's timers,
  ids, validators, dealer, match directory, testConfig/testLimits, TEST_PATCH_VERSION (file header says why).

## Decisions
- E2E's `CATALOG_VERSION` placeholder is `jackioh_cards::catalog_version()` (TS: `"v0.2.11"`), since
  env.rs must refuse a version that differs from the compiled-in one.
- `POST /api/auth/signin` stays in `ROUTES` unconditionally: under `E2E=1` the fixture provider answers;
  on Supabase it is 503 `PASSWORD_PATH_DISABLED_MESSAGE`, which is what TS production answered.
- `E2eAuth` defaults to `E2eDeletion::Unsupported` (TS's fixture provider had no `deleteUser`: 503);
  the test app sets `Deletes` (TS's fake had one). `Auth::can_delete_users()` is TS's
  `deps.auth.deleteUser === undefined` check, asked before anything is touched.
- JWKS: jose's `createRemoteJWKSet` behaviour kept (10 min max age, 30 s refetch cooldown when no key
  matches, 5 s fetch timeout, try every matching key); jose's claim checks kept (iss and aud required,
  exp/nbf checked when present, no leeway). HMAC tokens never verify in tier 1.
- Admin getUserById/deleteUser go straight to GoTrue (`/auth/v1/admin/users/{id}`, apikey + bearer
  secret key, `should_soft_delete: false`); a non-UUID id is "unavailable" as supabase-js's thrown
  validation was. Tier 3 and R194 use the secret key as `apikey` (no publishable key in Rust).
- `Debug` on Supabase types never prints the secret key or shared secret.
- Redemption padding (`pad_to`) uses `tokio::time::sleep_until` from the handler's first instant.
- Breaker updates take the lock in short sections between awaits (TS's object mutations between
  awaits); no guard is held across an await.
- Store errors in my handlers become 500 `internal` "something went wrong" with a `handler.threw` warn
  line (TS's router did both for a thrown error).
- Logs: `tracing` with `event = "<TS event name>"` and TS's camelCase keys as quoted field names; alert =
  ERROR. `serve` installs a JSON subscriber (flattened, stdout, `RUST_LOG` or `info`).
- serverConfig.ts: a two-line header comment, then `export const NAME = <JSON>;` sorted by name; the two
  object constants are written field by field in TS's key spelling (JSON keys sorted by serde_json).
