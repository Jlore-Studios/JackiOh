# Slice: part 18 (server 1), chunk 3 of 6: config, env, the pure ranked math, the tutorial routes, the auth and account tests
BUILDS-RUN: 0

## FILES
- `crates/server/src/config.rs` ← `apps/server/src/config.ts`: every constant in TS order with its comments, plus
  `ROOM_CODE_TTL_SECONDS = 900` (was `api/deps.ts`'s literal), `rating_window(f64) -> f64`, the two struct consts
  `RankTierPercents`/`CatalogNumberSetOffsets` (Serialize, TS key spelling), `INVITE_CODE_FORMAT`/`ROOM_CODE_FORMAT` as
  `jackioh_engine::wire::CodeFormat`, and `SERVER_CONFIG: LazyLock<IndexMap<&str, Value>>` (TS's frozen snapshot, TS key order).
- `crates/server/src/env.rs` ← `env.ts`: `Env` (alias `ServerEnv`), `NodeEnv`, `EnvError { problems }` (Display = TS's message),
  `PUBLIC_ENV_VARS`, `SERVER_ONLY_ENV_VARS`, `load_env(&IndexMap<String, String>) -> Result<Env, EnvError>`, `server_env()`.
- `crates/server/src/ranked/glicko2.rs` ← `glicko2.ts`: `Glicko`, `Score = f64`, `RatedOpponent`, `START_GLICKO`,
  `glicko2_period(&Glicko, &[RatedOpponent], tau: f64)`, `RatedGame { a, b }`, `rate_game(&Glicko, &Glicko, Score) -> RatedGame`.
- `crates/server/src/ranked/ladder.rs` ← `ladder.ts`: `GrapeTier`, `GRAPE_TIERS`, `RankTier`, `PIPS_PER_TIER`, `LADDER_TOP`,
  `GameResult`, `SeasonRank`, `Standing`, `fresh_rank`, `tier_index_of(i32) -> i32`, `tier_bottom(i32) -> i32`, `LadderPlace`,
  `place_of`, `Percentile`, `percentile_of(f64, &[f64])`, `target_ladder(&Percentile)`, `PipDeltaInput`, `pip_delta(&PipDeltaInput)`,
  `ApplyRankedGameInput`, `apply_ranked_game(&SeasonRank, &ApplyRankedGameInput)`, `jlorious_order(&[Standing]) -> Vec<String>`,
  `with_jlorious_peak(&SeasonRank, i32)`, `VisibleRank`, `visible_rank(Option<&SeasonRank>, Option<i32>)`, `PeakBadge`,
  `peak_badge(&SeasonRank) -> Option<PeakBadge>`, `VisibleRankWire` (the JSON shape of the two unions).
- `crates/server/src/ranked/season.rs` ← `season.ts`: `season_id_of(&str) -> String` (panics with TS's message), `ResetPlayer`,
  `ResetChange`, `ResetReport`, `SoftReset { changes, report }`, `reset_glicko(&Glicko, f64)`, `soft_reset(&[ResetPlayer]) -> SoftReset`.
- `crates/server/src/api/tutorial.rs` ← `api/tutorial.ts`: `TutorialProgressView`, `read_completed(&Value)`,
  `read_hidden_choice(&Value, now: i64)`, handlers `get_tutorial`, `put_tutorial` (chunk 1's ROUTES names).
- `crates/server/tests/api/auth.rs` ← `test/api/auth.test.ts` (R159 ×4, R194 ×7, R160 ×5, `/api/auth/me` ×3, `/api/profile` ×4,
  R665 ×5), plus the shared test doubles `GoTrue` (a scripted GoTrue on 127.0.0.1: JWKS, `/auth/v1/user`, admin user GET/DELETE,
  with call counts), `Clock`, `Harness`, `supabase_auth`, `sign`, `app_with`, `raw`, `row`, `fake_of`, `Captured` (tracing capture).
- `crates/server/tests/api/account.rs` ← `test/api/account.test.ts` (12 `it`s: 9 for the route, 3 for the provider's delete).

## SURFACE
- §4.2/§4.3 applied. Integer widths (no TS type says): durations, epoch ms and counts compared with store counts `i64`; lengths,
  caps and limits on collections `usize`; game/ladder quantities `i32`; ratings and Glicko numbers `f64`
  (`RATING_START = 1000.0`); `IPV6_RATE_LIMIT_PREFIX_BITS: u32`, `GLICKO_MAX_ITERATIONS: u32`, `MATCH_VOIDED_CLOSE_CODE: u16`,
  `Env.port: u16`, `Env.trusted_proxy_hops: usize`.
- §11.2: handler shape `pub async fn <name>(app: &App, req: Req) -> ApiResult`; errors built as `ApiError` struct literals.
- §11.3: `CATALOG_VERSION` must equal `jackioh_cards::catalog_version()`; `load_env` adds a problem otherwise.
- §5.1: `VisibleRank`/`PeakBadge` serialise as TS's unions (through `VisibleRankWire`, keys in TS order).

## DEPENDS-ON (names I call that other parts write)
- `jackioh_engine::wire::CodeFormat { alphabet: &'static str, length: usize, group_size: usize, separator: &'static str,
  max_input_length: usize }` (part 5), const-constructible (config.rs builds two `const`s of it).
- `crate::api::http::{Req { caller, body, .. }, Caller { profile, .. }, ApiError { code, message, details, retry_after_ms },
  ApiErrorCode::{BadRequest, Conflict, Internal}, ApiResult, json(u16, Value) -> Response}` (part 18).
- `crate::app::{App { db, auth, .. }, now_ms() -> i64, router(Arc<App>) -> axum::Router}` (chunk 1).
- `crate::db::store::{Db::Fake, Tx, Profile { id, in_match_id, .. }, TutorialHiddenChoice { hidden: bool, at: i64 }
  (Serialize camelCase), TutorialProgressRow { completed: Vec<String>, hidden_choice: Option<TutorialHiddenChoice> },
  TutorialMergeInput { profile_id, completed, hidden_choice, at }, TutorialMergeOutcome::{Merged { progress }, Limit}}`,
  `Db::begin(Option<&str>)`, `Tx::commit`, Tx methods `tutorial_get(&str)`, `tutorial_merge(&TutorialMergeInput, usize)` (part 20);
  tests also `profiles_get_by_id`, `profiles_get_by_user_id`, `profiles_set_in_match(&str, Option<&str>)`, `series_create`,
  `series_update -> bool`, `results_insert`, `results_record_for -> { wins, losses, draws }`, `decks_upsert(&_, usize)`,
  `decks_list`, `tutorial_merge`, `collection_upsert_quantities(&str, &[_])`, `collection_get`, `tickets_insert`,
  `tickets_open_for_profile`, `rooms_create -> bool`, `rooms_get`, and `db::fake::FakeData::seed_profile(Value)`; every row type
  they take is built from TS's JSON with `serde_json::from_value` (`row()`), so each needs `Deserialize` (camelCase).
- `jackioh_cards::catalog_version()` (part 1).
- Tests: `crate::support::deps::{test_app() -> Arc<App>, call(&Arc<App>, &str, &str, Option<&str>, Value) -> (u16, HeaderMap, Value)}`
  (chunk 1); `jackioh_server::auth::{Auth::{Supabase, E2e}, Auth::{verify, sign_in, delete_user}, AuthError::Unavailable(..),
  AuthUser { user_id, email: Option<String>, email_verified, app_metadata (Serialize) }, E2eDeletion::Unsupported,
  E2eAuth::set_deletion(&self, E2eDeletion), create_supabase_auth(SupabaseAuthInput) -> SupabaseAuth, SupabaseAuthInput { url,
  secret_key, jwt_secret: Option<String>, now: Option<Arc<dyn Fn() -> i64 + Send + Sync>>, .. }: Default}` (chunk 1).

## GAPS
- `SupabaseAuthInput.now` (TS's `now` seam) is assumed to exist; if chunk 1's provider reads `app::now_ms()` instead, delete the
  `now:` line in `tests/api/auth.rs::supabase_auth` — the tests already move tokio's paused clock too (`Clock::charge`), so they
  pass either way. `SupabaseAuthInput: Default` is assumed (the tests fill four fields and `..Default::default()` the rest).
- The provider is assumed to accept an `http://127.0.0.1:<port>` project URL (the scripted GoTrue) and to call GoTrue's own paths
  (`/auth/v1/.well-known/jwks.json`, `/auth/v1/user`, `/auth/v1/admin/users/{id}` GET and DELETE), as chunk 1's notes say.
- `tests/api/auth.rs::app_with` swaps `App.auth` by `Arc::try_unwrap(test_app().await)`: it needs `test_app()` to hand back the only
  handle (no spawned loop holding one). If not, chunk 1's `test_app_with(TestAppOptions)` with an auth option replaces it.
- `db::store::TutorialHiddenChoice` must derive `Serialize` (camelCase) for `TutorialProgressView`.
- `jsonwebtoken` needs a crypto backend feature (`rust_crypto`) for the tests' `encode` too (chunk 1 already lists it).
- `config::INVITE_CODE_FORMAT`/`ROOM_CODE_FORMAT` need part 5's `CodeFormat` fields to be `&'static str`/`usize` (a `String` field
  cannot be built in a `const`); otherwise part 31 turns the two consts into `fn`s.
- Not ported: TS `createTutorialRoutes()` (the table is `app::ROUTES`, which lists `get_tutorial`/`put_tutorial`); TS
  `api/deps.ts`'s `defaultConfig`/`defaultLimits`/`floodLimits`/`consoleLogger` (PORT-MAP: folded into config.rs; the constants
  they read are all here).
- Tests dropped or recast (v0.3.0 behaviour, SURFACE §11.3): TS's R160 sign-up tests and "one error per endpoint" (no sign-up route:
  recast as "sign-up answers every address with the same 404"); R160's scripted password client (the E2E fixture sign-in stands in);
  "is declared `user`" reads the route table in TS, here it is checked by letting a pending account delete itself; "the provider
  cannot delete users" uses the fixture auth with deletion switched off; TS's monkeypatched `series.activeFor` is a real series row.

## Decisions
- `Env` fields are TS's names snake_cased and lower-cased (`supabase_url`, `e2e`, `catalog_version`, …), as parts 18.1, 18.6 and
  20.3 call them; `Env`'s `Debug` redacts the secret key, the connection string, the JWT secret and the pepper.
- JS semantics kept by hand where Rust differs: `String.prototype.trim`'s whitespace set (`js_trim`), `Number(text)` for `PORT`
  (`js_number`: hex/octal/binary literals, `Infinity`, decimals), `JSON.stringify` for every `(got …)`, UTF-16 lengths for the
  pepper and lesson ids, `Number.isSafeInteger` for `hiddenChoice.at`; regexes (`MINOR_VERSION`, lesson ids, the commit SHA) are
  hand-written matchers (no regex crate on the server).
- `CATALOG_VERSION` that differs from `jackioh_cards::catalog_version()` is one more problem line in `load_env`'s report (TS's
  wording style); E2E's placeholder must therefore be the compiled-in version (chunk 1 does this).
- `season_id_of` panics (SURFACE §4.4.9: the version is compiled in, so an unreadable one is a broken build), as chunk 6's test expects.
- `glicko2_period` takes `tau: f64` (TS's default `GLICKO_TAU` passed by `rate_game`); ladder functions take their argument
  structs by reference and the anonymous input types are `PipDeltaInput`/`ApplyRankedGameInput`, as chunk 6's tests call them.
- `RANK_TIER_PERCENTS`'s module-load check (positive, sums to 100) is a compile-time `const _: () = assert!(…)`.
- `VisibleRank`/`PeakBadge` are Rust enums with `#[serde(into, try_from = "VisibleRankWire")]`: TS's `tier`-discriminated unions
  whose Grape arm takes five tier values cannot be an internally tagged enum.
- Sums are explicit `fold(0.0, +)` (Rust's float `Sum` starts at -0.0); sorts are stable `sort_by` (SURFACE §4.4.1).
- `put_tutorial` reads the time from `app::now_ms()` (chunk 1: the server's one clock); store errors become 500 `internal`
  "something went wrong" with a `handler.threw` warn line (TS's router did both); logs are `tracing` with `event = "tutorial.merged"`
  and TS's camelCase keys.
- Tests: TS's injected seams are replaced by a scripted GoTrue over loopback (nothing reaches the internet); tokens carry a far-future
  `exp` so the verifier's expiry rule passes without a clock; tests that move the cache clock run `start_paused` and `Clock::charge`
  advances both the seam's clock and tokio's; R160 byte comparisons go through the real router with `tower::ServiceExt::oneshot`
  (`raw`), everything else through `support::deps::call`.
