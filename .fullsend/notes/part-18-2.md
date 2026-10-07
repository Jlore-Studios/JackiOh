# Slice: part 18 (server 1), chunk 2 of 6: http, crypto, decks, e2e, ranked, retention, settings, stats
BUILDS-RUN: 0

## FILES
- `crates/server/src/api/http.rs` ← `api/http.ts` (+ `api/deps.ts`'s logger and flood limit, not ported as a file):
  `ApiErrorCode` (`status`, `as_str`), `ApiError { code, message, details, retry_after_ms }` (`new`, `with_details`,
  `internal`, `body`; `Display`, `Error`, `IntoResponse`, `From<StoreError>`, `From<serde_json::Error>`), `bad_request`,
  `rate_limited`, `json(u16, Value)`, `ok(Value)`, `ok_of(&T)`, `to_json(&T)`, `error_response`, `Req`, `Caller`,
  `RequestContext { peer_address }`, `UNKNOWN_CLIENT_ADDRESS`, `client_address(&HeaderMap, Option<&str>, usize)`,
  `rate_limit_address(&str)`, `rate_limit_address_with_prefix(&str, i64)`, `bearer_token`, the readers `str`,
  `optional_str`, `bool`, `string_list`, `deck_list`, `ApiResult`, `AuthLevel`, `resolve_caller(&App, &HeaderMap)`,
  `assert_active(&Profile)`, `account_key`, `address_key`, `RateLimiter` (`new`, `allow`, `retry_after_ms`, `size`,
  `Default`), `create_rate_limiter(usize, i64)`, `api_rate_limiter()`, `dispatch(&App, &[app::Route], Request)`,
  `now_ms` (re-export of `app::now_ms`), `sleep(i64)`, `pad_to(i64, i64)`, `log_info`/`log_warn`/`log_alert(&str, Value)`.
- `crates/server/src/api/crypto.rs` ← `api/crypto.ts`: `code_from_bytes`, `random_code`, `format_code`, `normalize_code`,
  `canonical_invite_code`, `is_well_formed_code`, `Hashes` (`code`, `ip`), `create_hashes(code_pepper, ip_pepper)`,
  `hashes_for_pepper(pepper)` (index.ts's `<pepper>:code` / `<pepper>:ip`), `player_tag`, `safe_equal`, `SystemIds`,
  `system_ids` (TS name, `#[allow(non_upper_case_globals)]`).
- `crates/server/src/api/decks.rs` ← `api/decks.ts`: `DeckView`, `TrioView`, `DeckLimits`, `LIMITS`, handlers `list_decks`,
  `put_deck`, `delete_deck`, `put_trio`, `import_trio`, `delete_trio`; queue-time `ModeChoiceInput::{Bo1 { deck_id },
  Bo3 { trio_id }, Random}`, `FrozenChoice::{Bo1 { deck }, Bo3 { trio }, Random}`, `read_mode_choice(&Value)`,
  `freeze_choice(&App, &str, &ModeChoiceInput)`, `assert_not_in_series(&App, &str)`.
- `crates/server/src/api/e2e.rs` ← `api/e2e.ts` (minus the provider): `E2EAccountStatus`, `E2EAccount` (`&'static str`
  fields), `E2E_ACCOUNTS`, `E2EInviteCodeKind` (`as_str`), `E2EInviteCodes` (`of`, `entries`), `E2E_INVITE_CODES`,
  `auth_user_of`, `session_of`, `E2ESeedSummary`, `E2ESeedOptions` (`Default`), `seed_e2e_fixtures(&App)`,
  `seed_e2e_fixtures_with(&App, E2ESeedOptions)`.
- `crates/server/src/api/ranked.rs` ← `api/ranked.ts`: `load_patch_version`, `SeasonDeps { patch_version }`
  (`SeasonDeps::current()`), `build_season_id`, `OpenedSeason`, `open_season_in_tx(&mut Tx, &SeasonDeps)`,
  `open_season(&App)`, `RankedSideInput::{Player { profile_id }, Bot { bot_id }}`, `RankedGameInput`, `GlickoWrite`,
  `PeakWrite`, `RankedWrites`, `RankedPlan`, `plan_ranked_game(&mut Tx, &App, &RankedGameInput)`,
  `commit_ranked_game(&mut Tx, &RankedPlan, i64)`, `rate_ranked_game(&mut Tx, &App, &RankedGameInput)`, `own_rank`,
  `leaderboard`, `match_ranks` and their bodies; handlers `get_ranked`, `get_leaderboard`, `get_match_ranks`.
- `crates/server/src/api/retention.rs` ← `api/retention.ts` + index.ts's purge loop: `purge_expired(&App)`,
  `run_purge(Arc<App>)` (boot, then every `RETENTION_PURGE_INTERVAL_SECONDS`).
- `crates/server/src/api/settings.rs` ← `api/settings.ts`: `PlayerSettingsView`, `read_groups(&Value, i64)`, handlers
  `get_settings`, `put_settings`.
- `crates/server/src/api/stats.rs` ← `api/stats.ts`: the response structs, handlers `get_cards`, `get_card`, `get_player`,
  `put_player`, `get_players`.
- `apps/server/src/api/deps.ts`, `apps/server/src/api/loadout-validator.ts`: not ported, left in place (part 37).

## SURFACE
- §11.2's `Req`, `Caller`, `AuthLevel`, `ApiResult`, `ApiError`, `json` exactly; every handler is
  `pub async fn <name>(app: &App, req: Req) -> ApiResult`, named as part 18.1's `ROUTES` names them (its notes).
- Part 18.1's choices adopted (they compiled the table): `Route`, `Handler`, `HandlerFuture` and `h!` live in `app.rs`,
  `Route` is the tuple `(method, path, AuthLevel, Handler)`, and `dispatch(&App, &[Route], Request)` takes the table;
  `app::now_ms()` is the one clock (re-exported from `http`). TS's `route()`, `Route`, `create*Routes()` are not
  ported: each module's header comment lists its rows of `ROUTES` in TS order.
- Part 20.4's store conventions used for every `Tx` call: `&str`, `&[T]`, `&Row`, plain numbers and unions, integers `i64`
  (caps included: `MAX_SAVED_DECKS as i64`, `PlayerSettingsLimits`, `PlayerStatsListOptions`, `note_peak_jlorious`'s
  position), `FrozenDeck.portrait: Option<Option<String>>`, `SeasonStanding { rank, rating }`, `RatedGameKind`,
  `RatedReason`, `RatedGameRow.winner_side: Option<usize>`, `StoreError::Other(String)`.
- Part 18.3's ranked signatures used: `rate_game(&Glicko, &Glicko, f64) -> RatedGame { a, b }`, `target_ladder(&Percentile)`,
  `apply_ranked_game(&SeasonRank, &ApplyRankedGameInput)`, `with_jlorious_peak(&SeasonRank, i32)`,
  `visible_rank(Option<&SeasonRank>, Option<i32>)`, `tier_index_of(i32) -> i32`, `soft_reset -> SoftReset { changes, report }`,
  `season_id_of(&str) -> String`.
- Part 5.3's validator: `DeckDraftInput`'s predicates are `&dyn Fn(&str) -> bool`; every other validator input and result
  is serde, so `TrioDraftInput`, `ImportRoomInput`, `DeckInput`, `LoadoutInput` are built from TS's JSON with
  `serde_json::from_value` and results (`LoadoutResult`, `ImportRoom`, issue lists) are read as JSON.

## DEPENDS-ON (names I call that other parts write)
- `crate::app` (18.1): `App { env, db, auth, limiter, catalog }`, `Route`, `Handler`, `now_ms() -> i64`.
- `crate::auth` (18.1): `AuthUser { user_id, email: Option<String>, email_verified, app_metadata: Default }`,
  `Session { access_token, refresh_token: Option<String>, expires_at: Option<i64>, user }`, `Auth::verify(&str)`.
- `crate::env::Env { trusted_proxy_hops, code_pepper }` (18.3).
- `crate::config` (18.3): `API_MAX_BODY_BYTES`, `API_REQUESTS_PER_MINUTE`, `IPV6_RATE_LIMIT_PREFIX_BITS`,
  `MAX_TRUSTED_PROXY_HOPS`, `RATING_START` (`f64::from`), `CODE_ALPHABET: &str` (const-evaluable `len()`),
  `INVITE_CODE_FORMAT`, `INVITE_CODE_GROUP_SIZE`, `INVITE_CODE_SEPARATOR: &str`, `INVITE_CODE_LENGTH`, `PLAYER_TAG_LENGTH`,
  `DECK_NAME_MAX_LENGTH`, `DRAFT_ISSUES_REPORTED_MAX`, `MAX_SAVED_DECKS`, `MAX_SAVED_TRIOS`, `PLAYER_SETTINGS_*`,
  `CARD_STATS_*`, `PLAYER_STATS_*`, `PUBLIC_STATS_MIN_LIVE_GAMES`, `CODE_ATTEMPT_RETENTION_DAYS`,
  `MATCH_ACTION_RETENTION_DAYS`, `RETENTION_PURGE_INTERVAL_SECONDS` (integers cast with `as`).
- `crate::api::catalog::Catalog { version: String, defs: CardDefs, card_ids: Vec<String> }`, `is_token(&str)`,
  `is_banned(&str)` (18.1).
- `crate::db::store` (20.4): `Db::{begin, Fake}`, `Tx::commit`, `StoreError::Other`, rows `Profile`, `ProfileStatus`,
  `ProfileCreateInput`, `InviteCode`, `SavedDeck`, `SavedTrio`, `TrioSlots` (3-tuple), `FrozenDeck`, `FrozenTrio
  { name, decks: 3-tuple }`, `UpsertOutcome`, `TrioUpsertOutcome`, `RetentionPurgeInput/Result`, `PlayerSettingValue`
  (Deserialize), `PlayerSettingsGroup { at, values }` (Clone, Serialize), `PlayerSettingsRow`, `PlayerSettingsMergeInput`,
  `PlayerSettingsLimits`, `PlayerSettingsMergeOutcome::{Merged { settings }, Limit}`, `GameRecordQuery { source, mode,
  patch: Option<String> }`, `PlayerStatsListOptions { search, limit, offset }`, `Season`, `SeasonStanding`, `BotRating`,
  `Pilot`, `RatedSide`, `RatedGameRow`, `RatedGameKind`, `RatedReason`, `MatchRow { players: (String, String), ranked:
  Option<bool> }`; `Tx` methods `profiles_get_by_user_id`, `profiles_create`, `profiles_set_status`, `profiles_get_many`,
  `profiles_set_glicko`, `codes_insert`, `collection_get`, `decks_list/get/upsert/remove`, `trios_list/get/upsert/remove`,
  `series_active_for`, `matches_get`, `player_settings_get/merge`, `game_records_list`, `player_stats_get/put/list_public`,
  `ranked_lock_seasons/seasons/create_season/rated_players/reset_ratings/standings/ranks_of/put_rank/note_peak_jlorious/
  bot/put_bot/record_game/game`, `purge_expired`.
- `crate::db::fake::FakeData::{reset(&mut self), grants_for(&self, &str) -> Vec<_>}` (20.3).
- `crate::ranked::{glicko2, ladder, season}` (18.3), names above plus `GRAPE_TIERS`, `GrapeTier: Copy`, `Standing`,
  `SeasonRank: Clone`, `VisibleRank`, `PeakBadge` (Serialize), `place_of -> LadderPlace { division, pips }`, `fresh_rank`,
  `percentile_of`, `jlorious_order`, `peak_badge`, `GameResult`, `START_GLICKO`, `ResetReport: Serialize`.
- `jackioh_engine`: `validator::{check_deck_draft, DeckDraftInput { name: String, cards: Vec<String>, is_deckable,
  name_max_length: usize, portrait: Option<String>, is_portrait: Option<&dyn Fn> }, check_trio_draft, TrioDraftInput,
  check_import_room, ImportRoomInput, validate_deck, DeckInput, validate_loadout, LoadoutInput, normalize_name, TRIO_DECKS}`;
  `wire::codes::{canonical_code(&str, &CodeFormat) -> Option<String>, normalize_code_text}`; `wire::emotes::is_portrait_id`;
  `wire::stats::{card_stats(&[GameRecord], &CardStatsFilter), win_rate(&Tally), Tally, CardStatsFilter, GameRecord,
  GameSource, GameMode, SourceFilter, PilotFilter}`; root `PerPlayer`, `PLAYER_IDS`, `Winner: From<PlayerId>`,
  `CardCost`, `CardDef`. `jackioh_cards::catalog_version()`.

## GAPS
- Guessed shapes, each at one call site: `DeckDraftInput`'s owned `name`/`cards` and `portrait: Option<String>` (the
  struct literal in `decks.rs::deck_draft_issues`); `canonical_code`'s `&CodeFormat`; `Session`'s and `AuthUser`'s field
  types; `FakeData::grants_for`'s return; `GameRecordQuery` (part 20.3's name; 20.4 does not list it).
- Part 19.1 calls `rate_ranked_game(&mut Tx, &App, RankedGameInput)` by value, part 20.5 by reference; mine takes `&RankedGameInput`.
- Not ported: `createE2EAuth` and `SIGN_UP_UNAVAILABLE_MESSAGE` (the provider is `auth.rs`'s `E2eAuth`; sign-up is dropped,
  SURFACE §11.3); TS `route()`, `Route`, `Handler`, `createDeckRoutes`/`createRankedRoutes`/`createSettingsRoutes`/
  `createStatsRoutes`, `createRouter`'s wrapper (the table is `app::ROUTES`); `loadout-validator.ts`'s
  `sharedLoadoutValidator`/`loadoutValidator` (inlined in `decks.rs::assert_legal`); `deps.ts`'s `defaultConfig`,
  `defaultLimits`, `floodLimits`, `consoleLogger` (config constants, `log_*`); R257's legacy `deckIndex` branch and its
  `NOTHING_SAVED` message (SURFACE §11.3).
- Bodies built with `json!` serialise their keys sorted (serde_json without `preserve_order`); typed structs keep TS's order.
  Byte-for-byte equality with TS holds for error bodies and struct bodies only.
- `localeCompare` (stats' card sort tie-break) is approximated: case-insensitive, then as written.
- `PUT /api/stats/player`'s size check counts `serde_json`'s text (a `1.0` in the body is 3 units, JS wrote `1`).
- `read_groups` walks the body's groups in sorted key order (serde_json's map), not JS insertion order: with two bad groups
  the one named in the 400 can differ from TS's.
- Under `E2E=1` with a Postgres store, `seed_e2e_fixtures` refuses (TS only seeded its in-memory store).

## Decisions
- One store transaction per handler where TS made several plain store calls in a row (reads, or an upsert and its read-back);
  `trios/import` keeps TS's split (one transaction for the writes, one for the read-back). Handlers with a caller begin with
  `Some(profile_id)` (store.ts's claim sub), anonymous ones with `None`.
- A TS `throw new Error(...)` is `ApiError::internal(message)`; `dispatch` logs `handler.threw {path, message}` and answers 500
  "something went wrong"; a store failure inside `resolve_caller` is a 500 at once (TS rethrew non-`ApiError`s there).
- `retry_after_ms` and `details.retryAfterMs` are both written by `rate_limited`; `Retry-After` reads the field, else the detail.
- The router's per-app state (TS's `createRouter` closure) lives on `App.limiter`: the sliding windows and R190's fewest
  forwarded-entry count. The peer address is a `RequestContext` extension when one is set, else `ConnectInfo<SocketAddr>`.
- Body: a declared `content-length` over the cap is refused unread; otherwise `axum::body::to_bytes(body, cap)`, any read
  failure being "too large"; UTF-8 lossy with a leading BOM dropped (TS's `TextDecoder`).
- `isIPv4`/`isIPv6` are `std::net`'s parsers; the IPv6 groups are `Ipv6Addr::segments()`. Regexes are hand-written checks.
- `read_mode_choice` without `mode` answers `"mode" must be "bo1", "bo3" or "random"`; a `deckIndex` without `deckId` answers
  `Best of 1 needs "deckId", …`.
- `assert_legal` calls `validate_deck`/`validate_loadout` directly with the catalog as JSON (`app.catalog.defs`, banned ids
  from `is_banned`) and copies each error's `rule`, `message`, `deck?`, `cardId?` into `details`, as the TS adapter did.
- `seed_e2e_fixtures` resets through `Db::Fake`'s lock, writes through `Tx` (one transaction per account and per code) and
  reads the launch grant with `grants_for` after the commit; fixture code hashes use `hashes_for_pepper(&env.code_pepper)`,
  the same key `codes.rs` reads.
- `stats.rs` reads `patches.json` through a private `include_str!` copy of `load_patch_versions`; the current patch is
  `app.catalog.version` (equal to `catalog_version()` in a booted server, SURFACE §11.3; TS fell back to it when no
  recorder was bound). The `tutorial` mode guard is kept by serialising `mode`.
- `ranked.rs`: the patch version is `jackioh_cards::catalog_version()` (`SeasonDeps::current()`); Jlorious positions are `i32`
  for the ladder and `i64::from` into the store; leaderboard tiers are grouped by tier index (no map keyed by `GrapeTier`).
- `retention::run_purge` logs `retention.purged {codeAttempts, matchActions}` only when either is > 0 and
  `retention.purge_failed {message}` on an error, as index.ts did.
- `crypto.rs`: `CODE_ALPHABET`'s 32-symbol check is a `const _: () = assert!(…)`; `safe_equal` folds XOR; `Hashes`' `Debug`
  hides the peppers.
