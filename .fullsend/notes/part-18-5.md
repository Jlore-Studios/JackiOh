# Slice: part 18 (server 1), chunk 5 of 6 (#406): the decks, e2e, ranked, rate-limit, redeem-feedback and retention tests
BUILDS-RUN: 0

## FILES
- `crates/server/tests/api/decks.rs` (new) ← `apps/server/test/api/decks.test.ts`: 60 tests (TS's 52 `it`s + its 9-row
  D-rule table as 9 tests, minus the one SURFACE §11.3 removes, below).
- `crates/server/tests/api/e2e.rs` (new) ← `apps/server/test/api/e2e.test.ts`: 32 tests.
- `crates/server/tests/api/ranked.rs` (new) ← `apps/server/test/api/ranked.test.ts`: 12 tests.
- `crates/server/tests/api/rate_limit.rs` (new) ← `apps/server/test/api/rate-limit.test.ts`: 12 tests.
- `crates/server/tests/api/redeem_feedback.rs` (new) ← `apps/server/test/api/redeem-feedback.test.ts`: 29 tests.
- `crates/server/tests/api/retention.rs` (new) ← `apps/server/test/api/retention.test.ts`: 2 tests.
Every `it` is a `#[test]`/`#[tokio::test]` named by SURFACE §7.3 (R-ids leading), every `describe` a `mod`, in TS order;
header comments and every comment stating a rule or citing a ruling are kept. Each file opens with a private harness
section (rule 5: its own copies of `from`, `json_of`, `fake`, `store!`, senders, hashes), so a wrong guess below is fixed
in one place per file.

## SURFACE
- §11.2: HTTP through `support::deps::{test_app, call}` and, where a test needs its own IP, headers or raw body bytes,
  through the frozen `app::router(Arc<App>)` with `tower::ServiceExt::oneshot` (private `send`). Store reads and writes
  through `Db::begin(None)` / `Tx::<substore>_<method>` / `Tx::commit`; fake tables through `Db::Fake`'s lock.
- §11.2 time: `#[tokio::test(start_paused = true)]` and `tokio::time::advance` for TS's manual and virtual timers;
  `app::now_ms()` (part 18.1) for TS's `timers.now()`. Tests that talk to a socket (the Supabase flood) run on the real
  clock: a paused clock would fire reqwest's timeouts while the socket is idle.
- §5.1: every port value is built from TS's own object literal (`from(json!(…))`), every answer compared as JSON, so the
  tests depend on TS's JSON keys, not on Rust field spellings or integer widths (as parts 20.2 and 18.6 did).
- §11.3 deltas ported as their Rust answer: no sign-up route (404), `GET /api/catalog` serves the compiled-in catalog at
  `jackioh_cards::catalog_version()`, the queue body names `mode` and `deckId` (no legacy `deckIndex` / no-mode body).

## DEPENDS-ON (names I call that other parts write)
- `crate::support::deps` (18.1): `test_app().await -> Arc<App>` (FakeStore, E2E auth and fixtures, `TRUSTED_PROXY_HOPS=1`,
  no spawned task holding a second `Arc`: `Arc::try_unwrap` swaps `db`, `auth` or `env.e2e`), `call(&Arc<App>, &str, &str,
  Option<&str>, Value /*Null = no body*/).await -> (u16, HeaderMap, Value)` (18.1's notes), and
  **`add_user(&App, user_id: &str, email: &str, email_verified: bool) -> String`** (TS `deps.auth.addUser`; 18.1 lists an
  `add_user` but not its arguments).
- `jackioh_server::app` (18.1): `App { env, db, auth, .. }` with pub fields and struct-update construction, `now_ms() -> i64`,
  `router(Arc<App>)`, `ROUTES: [(method, path, AuthLevel, Handler)]` (a tuple, 18.1's notes; `method`, `path` Display).
- `jackioh_server::env::Env { e2e: bool, code_pepper: String }` (18.3).
- `jackioh_server::auth` (18.1): `Auth::{Supabase, E2e}`, `Auth::verify(&str) -> Result<AuthUser, _>`, `Auth::sign_in(&str, &str)
  -> Result<Session, _>`, `AuthUser { user_id, email: Option<String>, email_verified, app_metadata (.is_empty()) }`,
  `Session { access_token, user }`, `create_supabase_auth(SupabaseAuthInput { url, secret_key, ..Default }) -> SupabaseAuth`
  accepting an `http://127.0.0.1:<port>` project URL.
- `jackioh_server::api::http` (18.2): `ApiError { code, message, details: Option<Value>, .. }: Debug`, `ApiErrorCode::{BadRequest,
  NotFound, AlreadyInMatch, LoadoutInvalid}`, `AuthLevel::Active`, `create_rate_limiter(limit, window_ms) -> RateLimiter` with
  `allow(&self, &str, i64) -> bool`, `retry_after_ms(&self, &str, i64) -> i64`, `size(&self) -> usize`, `account_key(&str)`,
  `address_key(&str) -> String`, `rate_limited(&str, i64) -> ApiError`, `error_response(ApiError) -> Response`.
- `jackioh_server::api::decks` (18.2), as part 19.1 also calls them: `read_mode_choice(&Value) -> Result<ModeChoiceInput,
  ApiError>`, `freeze_choice(&App, &str, &ModeChoiceInput) -> Result<FrozenChoice, ApiError>`, `assert_not_in_series(&App, &str)
  -> Result<(), ApiError>`; `ModeChoiceInput` and `FrozenChoice` `Serialize` as TS's `{ mode, … }`.
- `jackioh_server::api::ranked` (18.2): `open_season(&App) -> Result<OpenedSeason, _>`, `open_season_in_tx(&mut Tx, &SeasonDeps {
  patch_version: String })` (part 20.3's name), `rate_ranked_game(&mut Tx, &App, RankedGameInput) -> Result<RatedGameRow, _>`
  (by value, as part 19.1 calls it), `own_rank(&App, &str) -> Result<OwnRankBody, _>`, `leaderboard(&App, &str) ->
  Result<LeaderboardBody, _>`, `RankedGameInput: Deserialize`; all results `Serialize` as TS. The patch the app rates in
  is `jackioh_cards::catalog_version()` (as part 20.3 decided for season-start).
- `jackioh_server::api::results::record_result(&Arc<App>, RecordResultInput) -> Result<ResultRow, ApiError>` (19.1),
  `actor::contracts::RecordResultInput: Deserialize` (19.x).
- `jackioh_server::api::codes::{mint_invite_code(MintDeps { db: &Db, code_pepper: &str }, MintInput { max_uses: Option<i32>,
  expires_at: Option<i64> }) -> Result<MintedCode { id, formatted }, _>}` (18.1's notes); `api::crypto::normalize_code(&str) ->
  String`, `api::crypto::player_tag(&str) -> String` (18.2); `api::cors::is_origin_allowed(&[String], Option<&str>)` (18.1);
  `api::collection::{LAUNCH_COPIES, LAUNCH_GRANT_REASON}` (18.1); `api::queue::e2e_seed_count() -> usize` (19.1);
  `api::retention::purge_expired(&App) -> Result<RetentionPurgeResult, _>` (18.2).
- `jackioh_server::api::e2e` (18.2): `E2E_ACCOUNTS` (iterable of `E2eAccount { user_id, email, password, token, status:
  Serialize }`, `'static`), `E2E_INVITE_CODES: E2eInviteCodes { good, missing, expired, exhausted: &'static str }`,
  `seed_e2e_fixtures(&App) -> Result<E2eSeedSummary { granted_cards, .. }, _>` and **`seed_e2e_fixtures_with(&App,
  E2eSeedOptions { accounts, codes: Option<E2eInviteCodes> }: Default)`** (TS's `options` argument; SURFACE fixes only the
  one-argument form), errors `Display` with TS's messages ("CODE_ALPHABET", "same code").
- `jackioh_server::ranked` (18.3): `glicko2::START_GLICKO` (`Serialize`, `.rating`), `ladder::{SeasonRank, fresh_rank(&str, &str,
  i64), tier_bottom(i32) -> i32}`, `season::season_id_of(&str)`.
- `jackioh_server::config` (18.3): `API_REQUESTS_PER_MINUTE`, `API_MAX_BODY_BYTES`, `CODE_ATTEMPTS_PER_{PROFILE,IP}_PER_HOUR`,
  `CODE_ATTEMPT_WINDOW_SECONDS`, `REDEMPTION_CIRCUIT_{FAILURE_THRESHOLD,WINDOW_SECONDS}`, `REDEMPTION_IDENTICAL_ERROR`,
  `CODE_ATTEMPT_RETENTION_DAYS`, `MATCH_ACTION_RETENTION_DAYS`, `MAX_SAVED_DECKS`, `MAX_SAVED_TRIOS`, `DECK_NAME_MAX_LENGTH`,
  `DRAFT_ISSUES_REPORTED_MAX` (caps and lengths `usize`), `JLORIOUS_SIZE`, `RANK_PLACEMENT_GAMES`, `RATING_DEVIATION_START`,
  `SEASON_RESET_STRENGTH: f64`. Integers I do arithmetic on are cast `as i64`/`as usize`.
- `jackioh_server::db` (20): `store::{Db::{Fake, Pg}, Tx, StoreError::Other(String), CodeAttempt, CollectionEntry, InviteCode,
  MatchActionRow, MatchRow, ProfileCreateInput, Room, SavedDeck, SavedTrio, SeriesRow, Ticket, BotRating}` (all
  `Deserialize`), the Tx methods of part 20.2's list; `fake::FakeData { tables: { attempts, codes, decks, trios, profiles,
  matches, match_actions, seasons, season_ranks, rated_games }, on_call: Option<Arc<dyn Fn(&str) -> Result<(), StoreError> +
  Send + Sync>>, redemption: RedemptionSettings { enabled: Arc<dyn Fn() -> bool + Send + Sync>, .. } }`, `FakeData::{seed_profile
  (Value), grants_for(&str)}`, `fake::{create_e2e_store(E2eStoreOptions { catalog: FakeCatalog { card_ids, is_token, is_banned },
  now, redemption }) -> Db}`; the fake reports TS's dotted method names to `on_call` ("decks.upsert", "trios.upsert").
- `jackioh_engine` (5.3): `validator::{DeckDraftInput { name, cards: Vec<String>, is_deckable: &dyn Fn(&str) -> bool,
  name_max_length, portrait: Option<String>, is_portrait: Option<&dyn Fn(&str) -> bool> }, check_deck_draft(&_),
  check_trio_draft(&_), check_import_room(&_), validate_deck(&_), normalize_name(&str) -> String}` (the non-predicate inputs
  built from JSON); `wire::emotes::{PORTRAIT_IDS, is_portrait_id(&str)}`; `config::DECK_SIZE`; `jackioh_cards::{CATALOG,
  CATALOG_IDS, catalog_version, catalog_json}` (part 1).

## GAPS
- The two bold guesses above: `support::deps::add_user`'s arguments and `api::e2e::seed_e2e_fixtures_with`.
- Not ported (SURFACE §11.3 removes the behaviour): decks' "R257 reads a legacy deckIndex as a position in the saved list".
  Recast: readModeChoice's two legacy tests are one test that the legacy forms are refused (400); "R165 … nothing saved"
  asks by `deckId` instead of `deckIndex: 0`; R143's queue tests send `{ mode: "bo1", deckId }`; e2e's "cannot create an
  account" asserts there is no sign-up route.
- TS seams with no Rust counterpart, recast onto observable behaviour: retention's `store.purgeExpired` spy → rows at and
  one millisecond past each cutoff; decks' recording validator → the real validator's verdict (a legal deck freezes, an
  illegal one is refused naming the deck; the 422's details equal `validate_deck`'s errors); rate-limit's ad-hoc routers →
  real routes of the same auth level (`/api/stats/players` none, `/api/auth/me` user, `/api/tutorial` active), "per router"
  → "per App"; `codeDeps({ breakerFailureThreshold: 1 })` → the window filled to `REDEMPTION_CIRCUIT_FAILURE_THRESHOLD - 1`
  first; `rateLimited` with NaN/Infinity → only -1 (the wait is an `i64`); fake match directory's `started` → the match
  rows; fake ids' `seed-<n>` → 32 lower-case hex digits; ranked's `{ ...deps, patchVersion }` → `open_season_in_tx` with a
  `SeasonDeps` for the next minor, and "back in placements" read as no rank row in the new season.
- `e2e_seed_count()` is a process static (19.1): the R143 test checks the count is no higher than before its enqueue,
  which another test enqueueing a seed at the same moment could still upset.
- `tracing` capture in rate_limit.rs reads the `event` field (or the message) and a `key` field (quotes trimmed): 18.1 logs
  `event = "api.rate_limited"`; the `key` field name is TS's.
- Lines over rustfmt's 110 columns: formatting only (part 31/35 run `cargo fmt`). `as` casts that turn out to be the same
  type are clippy `unnecessary_cast` warnings for Wave 3.

## Decisions
- Hashes in the tests are private HMAC-SHA256 copies (`hmac`, `sha2`), peppered as `index.ts` does (`{CODE_PEPPER}:code`
  over `api::crypto::normalize_code`, `{CODE_PEPPER}:ip` over the trimmed lower-cased address): 18.1 keeps the server's
  private, and these are what R144's fixtures and §9.4's attempt rows are keyed by.
- Card ids are the real catalog's (`CATALOG_IDS` without Tokens, TS's `isToken`: flag or tag); TS's synthetic `token-sheep`
  is the catalog's first Token, `core-999` stays (unknown to the real catalog, so D3 as before); D2's "21" is `DECK_SIZE + 1`.
  The e2e store tests keep TS's four-card grant catalog through `create_e2e_store`.
- Legal decks for the freeze and R143 tests: `DECK_SIZE` distinct non-Token cards, owned one copy each through
  `collection_upsert_quantities`; trio decks take disjoint slices.
- The rolled-back transaction is a dropped `Tx` (no commit); TS's nested `tx` is a helper handed the outer `&mut Tx`; the
  invite-code race is `tokio::join!` of two one-claim transactions (the fake's mutex serialises them).
- `toMatchObject` is a private `assert_matches` (objects by subset, arrays element-wise, numbers by value); `toBeCloseTo(_, 9)`
  is `|a − b| < 0.5e-9`; ratings are compared through `as_f64` (an `f64` serialises as a JSON float).
