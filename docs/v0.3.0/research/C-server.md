# Research C: `apps/server` and the web client's engine seams (for the v0.3.0 Rust rewrite)

Read-only research. Paths are relative to `/home/user/JackiOh`. Line counts are from `wc -l` on 2026-10-06.
The `it(` counts are a grep of `it(` / `test(` at line start, so they are approximate (±a few for `it.each`).

## 0. Headline findings (read first)

1. **The server is one Node process**, `apps/server/src/index.ts` (401 lines): a Hono/`@hono/node-server` listener for a hand-written router (`src/api/http.ts`), plus a `ws` upgrade on the same port at **`/ws/match`**, plus four in-process timers (matchmaker every 3 s, series sweeper every 5 s, ceiling reaper every 30 s, retention purge at boot and then hourly). It also opens the ranked season at boot. There is **no health route**: Render probes **`GET /api/catalog`**.
2. **39 HTTP routes**, all under `/api/`, registered by `route(method, path, auth, handler)` calls (§2). Auth levels are `none` / `user` / `active`. Every error is `{ "error": { code, message, details? } }`. A wrong method answers **404** (`not_found`), not 405.
3. **The wire format of an action frame differs from the parsed TS type.** On the wire it is `{"type":"action","action":{...ActionBody,"nonce":"…"}}`, and the nonce may also sit at the top level. The internal `ActionMessage` is `{type,nonce,body}` (`protocol.ts:504`).
4. **The browser authenticates the socket with `?token=<jwt>&matchId=<id>` in the URL** (`apps/web/src/game/net.ts:446` `socketUrlFor`). The e2e Node player does the same. The server's README calls the `Sec-WebSocket-Protocol: jackioh.v1, <token>` path the preferred one, but no client uses it. All three token paths must be kept, or the client must be changed at the same time.
5. **The web client imports `apps/server/src/config.ts` by relative path** in 23 non-test web files and 10 e2e files (§7.4). Deleting or renaming the TS server breaks the web build unless those constants move first (for example into `packages/shared`).
6. **Migration ledger checksum**: `app.migrations.checksum` is FNV-1a over **UTF-16 code units** (`charCodeAt`), as lowercase hex padded to 8 (`src/db/migrate.ts`). A Rust runner must reproduce it exactly, keep `REWRITTEN` (`0013_retention_purge.sql: ["16b93e4d"]`) and keep the per-transaction `pg_advisory_xact_lock(0x6a61636b)`. Otherwise every existing database refuses to boot.
7. **The rules live in SQL as much as in TS**: 31 `app.*` functions and 10 triggers in 26 migrations (6,068 lines of SQL). `src/db/store.ts` (2,979 lines) mostly calls them (`app.redeem_invite_code`, `app.upsert_deck`, `app.upsert_trio`, `app.append_match_action`, `app.claim_ticket_pair`, `app.live_matches`, `app.merge_tutorial_progress`, `app.merge_player_settings`, `app.forget_voided_match`, `app.purge_expired_rows`). Every transaction first runs `SET LOCAL role service_role` plus `request.jwt.claim.sub` (store.ts header, rule 2). Keeping the schema means keeping those calls.
8. **Finished actors are never removed from the registry.** Only the reaper calls `matches.stop` (`results.ts:309`). Rematch presence (`GET /api/matches/:id/rematch` → `opponentHere`) reads the live actor's open sockets (`registry.presenceOf`), so this behaviour is load-bearing.
9. **WASM API surface needed by the web** (§7.6): hotseat needs `createGame/beginGame/reduce/legalActions/viewFor/hashState/registeredCatalog`. Practice needs those plus `lastBoardFor`, `seatPlayedBy`, `createRng(seed,cursor)` with `.cursor`, `buildAiDeck`, `aiToAct`, `decide`, and reads `state.result`. The e2e harness reads raw `GameState` JSON fields off `window.__jackioh.state` and folds logs in Node with the TS engine (`e2e/support/tasks/replay-runner.ts`).

---

## 1. `apps/server/src` file map

Totals: 25,772 lines in `src/`, of which 6,068 are SQL migrations and 19,704 are TypeScript. Tests: 24,591 lines of TS and 6,191 of SQL in `test/`.

### 1.1 Composition root, env and config

| File | Lines | Responsibility |
| --- | --- | --- |
| `src/index.ts` | 401 | Composition root. `loadServerEnv` (E2E placeholder env), `createRuntime` (binds every port; swaps the store and auth for E2E), `allRoutes`, `start()`: Hono `serve` + `withCors` + `attachWebSocketServer`, the matchmaker, the series sweeper, the reaper (30 s), the retention purge, `openSeason` at boot |
| `src/env.ts` | 349 | `loadEnv()`: validates every env var and reports all problems at once. `PUBLIC_ENV_VARS` / `SERVER_ONLY_ENV_VARS` lists |
| `src/config.ts` | 782 | Every server number (R79 clocks, codes, rate limits, ranked/Glicko, deck/trio caps, series, tutorial, settings, stats) **plus client-only UI numbers** that the web imports (AUTH_*, GATE_SLOW_NOTICE_SECONDS, DECK_AUTOSAVE_*, MATCH_FOUND_NAV_DELAY_MS, SERIES_POLL_SECONDS…). `SERVER_CONFIG` frozen snapshot |

### 1.2 HTTP layer and cross-cutting concerns

| File | Lines | Responsibility |
| --- | --- | --- |
| `src/api/http.ts` | 709 | Framework-free router: `route()`, `matchPath` (`:param`), `ApiError` + `ApiErrorCode`→status table, `errorResponse` (adds `Retry-After`), JSON body reader (64 KiB cap), `clientAddress` (XFF from the right, R190), `rateLimitAddress` (IPv6 /56), `bearerToken`, `resolveCaller` (lazily creates a pending profile), `assertActive`, sliding-window `createRateLimiter` (300/min per account, or per IP-hash bucket), `padTo` |
| `src/api/cors.ts` | 160 | `withCors`: preflight 204, allow-list from `PUBLIC_ORIGINS`, `VITE_DEV_ORIGINS` (`http://localhost:5173`, `http://127.0.0.1:5173`) |
| `src/api/crypto.ts` | 106 | `randomCode` from `CODE_ALPHABET` (byte & 31), `formatCode`, `canonicalInviteCode`, `createHashes` (HMAC-SHA256 hex, peppers `${CODE_PEPPER}:code` / `:ip`), `playerTag` (sha256(profileId) → 6 alphabet chars), `systemIds` (uuid v4, seed = 16 random bytes hex, code) |
| `src/api/deps.ts` | 78 | `defaultConfig()`, `defaultLimits()` (room TTL is a literal `15*60*1000`), `floodLimits`, `consoleLogger` (JSON lines `{level,event,...}` to stdout/stderr) |
| `src/api/ports.ts` | 1272 | All port types: `Timers`, `ServerConfig`, `ApiLimits`, `CatalogInfo`, `AuthProvider`, `Hashes`, the `Store` and its 17 sub-stores, `MatchDirectory`, `GameRecorder`, `Ids`, `Logger`, `ServerDeps` |

### 1.3 Auth, invite codes, account

| File | Lines | Responsibility |
| --- | --- | --- |
| `src/api/auth.ts` | 898 | `createSupabaseAuth`: JWKS (jose) → HS256 secret → GoTrue `/auth/v1/user`. Session liveness cache (R194), email confirmation from the admin API (`getUserById`), aal2 rule for TOTP (R665), `deleteUser`. Routes: `/api/auth/signup`, `/api/auth/signin`, `/api/auth/me`, `/api/profile`, `DELETE /api/account` |
| `src/api/codes.ts` | 424 | `mintInviteCode`, six-step redemption over `store.redeem`, constant-time floor (250 ms), circuit breaker (100 failures in 600 s), routes `/api/codes/redeem`, `/api/codes/status` |
| `src/api/collection.ts` | 192 | `GET /api/collection`, `grantCards` (two-table tx), `ownedMap`, `callerProfile` helper |

### 1.4 Decks, trios, queue, rooms, series (Conquest), rematch

| File | Lines | Responsibility |
| --- | --- | --- |
| `src/api/decks.ts` | 674 | Deck/trio draft routes (D1–D4, T1–T3 via validator), `POST /api/trios/import` (one tx), `readModeChoice` (incl. legacy `deckIndex`), `freezeChoice` (L1–L6 at queue time), `assertNotInSeries` |
| `src/api/loadout-validator.ts` | 71 | The only import site of `@jackioh/validator` for L1–L6 (`sharedLoadoutValidator`), and it re-exports draft checks, `TRIO_DECKS` |
| `src/api/queue.ts` | 578 | Enqueue (freezes the deck/trio into the ticket), `tryPair` (rating window widening), `startPairedMatch` / series, `startMatchmaker` (3 s sweeper), E2E `seed` override (module-level `Map`), routes `/api/queue` (POST/DELETE), `/api/queue/population` |
| `src/match/rooms.ts` | 350 | `POST /api/rooms`, `POST /api/rooms/:code/join` (atomic claim; bo1/bo3/random), E2E room seed `Map` |
| `src/api/series-rules.ts` | 654 | Pure Conquest transitions over `SeriesRow` (`newSeries`, `pickDeck`, `beginGame`, `gameEnded`, `timeoutPicks`, `abandonUnstarted`, `forfeitSeries`, `rateSeries`, `gameSeats`, `projectSeries` → `SeriesView`) |
| `src/api/series.ts` | 570 | Store half: `startSeries`, `ensureSeriesGame`, `resumeSeries`, `advanceSeriesInTx`, compare-and-set `writeTransition`, `sweepSeries`/`startSeriesSweeper` (5 s), routes `/api/series/:id`, `/pick`, `/forfeit`, `/api/matches/:matchId/series` |
| `src/api/rematch.ts` | 276 | R672 rematch offers held **in memory** (module `Map`, TTL 10 min), `POST`/`GET /api/matches/:matchId/rematch` |

### 1.5 Results, ranked, stats, retention, tutorial, settings

| File | Lines | Responsibility |
| --- | --- | --- |
| `src/api/results.ts` | 325 | `createRecordResult` (one tx: results row, rated game, last boards, clear in-match, cancel tickets, finish, advance series; then `resumeSeries`, `recordLiveGame`), `createVoidMatch` (R679), `reapStuckMatches` (ceiling draw, `turns = 0`) |
| `src/api/game-records.ts` | 85 | `recordLiveGame`: folds `(seed, decks, log)` through `engine.summarizeGame` → `game_records` row (`source: "live"`) |
| `src/api/ranked.ts` | 474 | Season open (`openSeason`/`openSeasonInTx`, soft reset R609), `planRankedGame`/`commitRankedGame`/`rateRankedGame`, `ownRank`, `leaderboard`, `matchRanks`, routes `/api/ranked`, `/api/leaderboard`, `/api/matches/:matchId/ranks` |
| `src/ranked/glicko2.ts` | 147 | Pure Glicko-2 (R603) |
| `src/ranked/ladder.ts` | 286 | Pure visible ladder: Grape tiers, divisions, pips, Jlorious (R605–R608) |
| `src/ranked/season.ts` | 106 | Pure season id from the patch minor version, soft reset (R609) |
| `src/api/stats.ts` | 432 | Public card stats and drill-down (`cardStats` from `@jackioh/shared`), player stats get/put, public players list |
| `src/api/retention.ts` | 26 | `purgeExpired` → `store.purgeExpired` (code_attempts 30 d, match_actions 90 d after the match ended) |
| `src/api/tutorial.ts` | 121 | `GET`/`PUT /api/tutorial` (grow-only merge, R320) |
| `src/api/settings.ts` | 151 | `GET`/`PUT /api/settings` (group merge by `at`, R633/R634) |

### 1.6 Catalog

| File | Lines | Responsibility |
| --- | --- | --- |
| `src/api/catalog.ts` | 276 | `loadCatalog` (reads `packages/cards/catalog.json` via `import.meta.resolve("@jackioh/cards")`), `catalogAtVersion` (reads `packages/cards/patches/<v>.json`, cached), `loadCurrentPatch`/`loadPatchVersions` (`patches.json`), `DEPLOYED_COMMIT_HEADER = "x-deployed-commit"`, routes `/api/catalog`, `/api/catalog/:version` |

### 1.7 Match actor, protocol, sockets

| File | Lines | Responsibility |
| --- | --- | --- |
| `src/match/protocol.ts` | 532 | Message unions, builders, `parseClientMessage` (total, whitelists fields), constants `MAX_FRAME_BYTES = 65536`, `MAX_NONCE_LENGTH = 128`, `SERVER_NONCE_PREFIX = "srv-"`, `MATCH_VOIDED_CLOSE_REASON = "voided"` |
| `src/match/wsServer.ts` | 341 | `ws` adapter: `WS_PATH = "/ws/match"`, `WS_SUBPROTOCOL = "jackioh.v1"`, `WS_CLOSE` {4401, 4403, 4404, 1011}, token from header / subprotocol / `?token=`, origin check (HTTP 403), per-address cap (HTTP 429), `maxPayload` |
| `src/match/actor.ts` | 917 | One actor per match: serialized task queue, nonce→ack map, reduce → append → push views/prompts/clocks, clock expiry → server actions, terminal → `recordResult` or `voidMatch`, flood limit per seat, emote relay, aim relay (R738), seat swap (R677), absent-seat grace (R744) |
| `src/match/clock.ts` | 299 | Turn, prompt, mulligan, grace and ceiling deadlines over the `Timers` port |
| `src/match/registry.ts` | 195 | `MatchDirectory` + `attach`/`actorFor`/`live`: `start` (last boards, glitch boards, createGame+beginGame **before** the row is written), `rebuild` by `fold`, in-flight rebuild dedupe, `presenceOf` |
| `src/match/contracts.ts` | 158 | `Socket`, `ClockView`, `ClockExpiry`, `MatchClock`, `RecordResult`, `VoidMatch`, `ActorDeps` |
| `src/match/engine.ts` | 198 | `EnginePort` type, `MatchSnapshot`, `REQUIRED_ENGINE_EXPORTS`, `EngineUnavailableError`, `setEnginePort` (test injection), `loadEnginePort` (dynamic string-specifier import) |
| `src/match/engine.real.ts` | 103 | The only `@jackioh/engine` import: `registerAll()`, maps the engine onto the port; `dealRandomDeck` = `buildAiDeck(createRng(seed), DECK_SIZE, {banned: []})` |

### 1.8 Persistence and CLIs (`src/db/**`)

| File | Lines | Responsibility |
| --- | --- | --- |
| `src/db/store.ts` | 2979 | `createPostgresStore({connectionString, max?, onError?})` over `pg`. Aliases `createStore`, `postgresStore`. Every tx: `set_config('role'…)` service_role + `request.jwt.claim.sub`. "KNOWN DIVERGENCES" section at line 2875 |
| `src/db/migrate.ts` | 156 | `db:migrate`: ledger `app.migrations(filename, applied_at, checksum)`, FNV-1a checksum, `REWRITTEN`, per-file transaction with `pg_advisory_xact_lock(0x6a61636b)` |
| `src/db/seed-catalog.ts` | 187 | `db:seed-catalog`: upserts `public.cards` from catalog.json at `CATALOG_VERSION`, writes `app.settings.catalog_version` (one tx) |
| `src/db/mint-code.ts` | 113 | `codes:mint`: one invite code to stdout; `--max-uses=N`, `--expires-in-days=N`; loads the full env |
| `src/db/seed-accounts.ts` | 286 | `db:seed-accounts`: GoTrue admin `POST /auth/v1/admin/users` (email_confirm), activates, seeds decks + trio; needs `SEED_ACCOUNTS_PROJECT` and `SEED_ACCOUNTS_PASSWORD`; refuses under production |
| `src/db/season-start.ts` | 109 | `db:season-start [--dry-run]`: same path as the boot season open |
| `src/db/card-stats.ts` | 138 | `stats:cards` CLI over `gameRecords.list` + `cardStats` |
| `src/db/import-dev-records.ts` | 86 | `stats:import <jsonl>` (resolves the path against `INIT_CWD`) |
| `src/db/migrations/*.sql` | 6068 (26 files) | §4.3 |

### 1.9 E2E (test-server) mode and the in-memory stores

| File | Lines | Responsibility |
| --- | --- | --- |
| `src/api/e2e.ts` | 301 | Fixture accounts (`e2e-token-p1`, `e2e-token-p2`, `e2e-token-pending`), fixture invite codes (good `ABCD-EFGH-JKMN-PQRS`, missing `ZZZZ-…`, expired `XPRD-…`, exhausted `XHST-…`), `createE2EAuth`, `seedE2EFixtures` (R144 reseed at boot) |
| `src/api/e2e-store.ts` | 767 | In-memory `Store` for `E2E=1` (`createE2EStore`, `reset`, `grantsFor`) |
| `src/api/memory-stores.ts` | 866 | In-memory decks, trios, series, tutorial, settings, game records, ranked, last boards and player stats, shared by `e2e-store.ts` and `test/fakes/store.ts` |

---

## 2. HTTP API

### 2.1 Router mechanics (`src/api/http.ts`)

- Routes are tried in registration order (`allRoutes()` in `index.ts`). The path must match segment for segment (`:name` → `req.params`). If the path matches but the method does not, the answer is **404** `not_found` "method not allowed for this path". An unknown path is 404 "no such endpoint". OPTIONS preflights are answered by `withCors` before the router runs.
- Per request: (1) hash the client address with the IP pepper (`clientAddress` + `rateLimitAddress`). (2) For a non-`none` route, refuse early if the address bucket is already over. (3) `resolveCaller`: a bearer token → `auth.verifyAccessToken` → `profiles.getByUserId`, or **create a pending profile** at `RATING_START`. (4) Count against `account:<profileId>` or `address:<ipHash>` (300/min, `API_REQUESTS_PER_MINUTE`). (5) Throw any held auth error. (6) For `active`, `assertActive` (403 `account_pending` / `account_banned`). (7) Read the JSON body (GET/HEAD → `{}`; > 65,536 bytes → 400; a body that is not a JSON object → 400). (8) Call the handler. A thrown non-`ApiError` becomes 500 `internal`.
- Success: `json(200, body)` with `content-type: application/json; charset=utf-8` and `cache-control: no-store`. The exceptions are the stats routes (`public, max-age=300` for cards and `60` for players) and `DELETE /api/account` (204, no body).
- An `x-forwarded-for` request logs `api.forwarded_for {fewestEntries, trustedProxyHops}` whenever it sets a new minimum.

**Error code → status** (`ApiErrorCode` table at `http.ts:27`):

| code | status | code | status |
| --- | --- | --- | --- |
| bad_request | 400 | already_queued | 409 |
| unauthorized | 401 | invalid_code | 400 |
| email_unverified | 403 | update_required | 409 |
| account_pending | 403 | loadout_invalid | 422 |
| account_banned | 403 | match_not_finished | 422 |
| not_found | 404 | series_game | 422 |
| conflict | 409 | double_requires_ranked | 422 |
| already_in_match | 409 | rate_limited | 429 (+`Retry-After` seconds, `details.retryAfterMs`) |
| unavailable | 503 | internal | 500 |

### 2.2 Every route

Auth: `none` = open; `user` = verified token + profile, any status; `active` = also `status = 'active'`.

| # | Method | Path | Handler | Auth | Request body / query | 200 response | Other statuses |
| --- | --- | --- | --- | --- | --- | --- | --- |
| 1 | POST | `/api/auth/signup` | `api/auth.ts:766` | none | `{email, password}` | `{pendingEmailVerification:true, userId, session:null}` or `{pendingEmailVerification:false, userId, session:{accessToken, refreshToken, expiresAt}}` | 400; 401 (one identical message); **503 always in production** (no publishable key is passed to `createSupabaseAuth`); 503 in E2E |
| 2 | POST | `/api/auth/signin` | `api/auth.ts:785` | none | `{email, password}` | `{userId, emailVerified, session:{accessToken, refreshToken, expiresAt}}` | 401; 503 in production; works in E2E with the fixture passwords |
| 3 | GET | `/api/profile` | `api/auth.ts:812` | active | – | `{id, email, status, record:{wins,losses,draws}, winRate:number|null}` | 401/403 |
| 4 | DELETE | `/api/account` | `api/auth.ts:845` | user | – | **204, no body** | 409 `already_in_match` (live match), 409 `conflict` (active series), 503 (provider unavailable / no deleteUser) |
| 5 | GET | `/api/auth/me` | `api/auth.ts:866` | user | – | `{profile:{id,status}, needsInviteCode, emailVerified, currentMatchId, currentSeriesId, email}` | 401 |
| 6 | GET | `/api/catalog` | `api/catalog.ts:254` | none | – | `{version, defs: CardDefs}`, plus header **`x-deployed-commit: <hex sha>`** when `RENDER_GIT_COMMIT` is set (env field `DEPLOYED_COMMIT`, kept only if `/^[0-9a-f]{7,64}$/` after lower-casing) | – |
| 7 | GET | `/api/catalog/:version` | `api/catalog.ts:268` | none | – | `{version, defs}` from `packages/cards/patches/<version>.json` (the current version is served from memory) | 404 (version not in `patches.json`, or not matching `/^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/`). **Not used by the web client** |
| 8 | POST | `/api/codes/redeem` | `api/codes.ts:378` | user | `{code}` | `{status:"active", needsInviteCode:false}` | Every answer is padded to 250 ms. 400 `invalid_code` (identical body for missing/expired/exhausted/revoked/malformed); 403 `email_unverified`; 409 `conflict` (not pending); 429 (+Retry-After = window); 503 (breaker open) |
| 9 | GET | `/api/codes/status` | `api/codes.ts:404` | user | – | `{redemptionEnabled, retryAfterMs, attemptsRemaining, attemptsRetryAfterMs}` | – |
| 10 | GET | `/api/collection` | `api/collection.ts:184` | active | – | `{catalogVersion, entries:[{cardId, quantity}]}` sorted by cardId | – |
| 11 | GET | `/api/decks` | `api/decks.ts:275` | active | – | `{catalogVersion, decks: DeckView[], trios: TrioView[], limits:{decks:10, trios:5, nameLength:40}}`; DeckView = `{id,name,cards,portrait,catalogVersion,createdAt,updatedAt}`, TrioView = `{id,name,deckIds:[id|null×3],createdAt,updatedAt}` | – |
| 12 | PUT | `/api/decks/:id` | `api/decks.ts:288` | active | `{name, cards:string[], catalogVersion, portrait?:string|null}`; `:id` is a client-minted UUID | `{deck: DeckView}` | 400 (draft issues; `details` = up to 50 issues); 404 (another profile's id); 409 `update_required` (`details:{expected, received}`); 409 `conflict` cap (`details:{limit}`) |
| 13 | DELETE | `/api/decks/:id` | `api/decks.ts:345` | active | – | `{deleted:boolean}` (idempotent; empties trio slots) | 400 bad id |
| 14 | PUT | `/api/trios/:id` | `api/decks.ts:358` | active | `{name, deckIds:[id|null, id|null, id|null]}` | `{trio: TrioView}` | 400; 404; 409 cap (`{limit}`); 409 unknown deck (`{unknownDeck:true}`) |
| 15 | POST | `/api/trios/import` | `api/decks.ts:408` | active | `{catalogVersion, trio:{id,name}, slots:[{id,name,cards}|null ×3]}` | `{decks: DeckView[], trio: TrioView}` | 400 (shape or draft); 409 `update_required`; 409 `conflict` room (`details:{decksShort, triosShort, limits:{decks,trios}}`); 409 race; 404 |
| 16 | DELETE | `/api/trios/:id` | `api/decks.ts:483` | active | – | `{deleted}` | – |
| 17 | POST | `/api/queue` | `api/queue.ts:501` | active | `{mode:"bo1", deckId}` / `{mode:"bo3", trioId}` / `{mode:"random"}`. Legacy: no `mode` means bo1, and `{deckIndex}` may replace `deckId`. **`seed`** is allowed only when `E2E` (otherwise 400) | `{ticketId, status:"open"|"matched"|"cancelled", matchId (non-bo3 matched only), seriesId (bo3 matched), population, mode}` | 409 `already_in_match` (also when in a series, `details.seriesId`), 409 `already_queued` (`details.ticketId`); 422 `loadout_invalid` (details = issues) |
| 18 | DELETE | `/api/queue` | `api/queue.ts:548` | active | – | `{cancelled:false}` or `{cancelled:true, ticketId}` | – |
| 19 | GET | `/api/queue/population` | `api/queue.ts:571` | user | – | `{population, byMode:{bo1,bo3,random}}` | – |
| 20 | POST | `/api/rooms` | `match/rooms.ts:199` | active | Same mode choice as the queue, + E2E `seed` | `{code (6 chars), expiresAt (now + 15 min), mode}` | 409 `already_in_match`; 422; 503 (no free code after `CODE_ATTEMPTS` tries) |
| 21 | POST | `/api/rooms/:code/join` | `match/rooms.ts:250` | active | Mode choice that must equal the room's, + E2E `seed` | `{matchId|null, seriesId|null, code, seat:"p2", mode}` | 404 (not open / expired / malformed); 409 (own room; mode mismatch `details:{mode}`; already joined; host busy); 422 |
| 22 | GET | `/api/series/:id` | `api/series.ts:518` | active | – | `SeriesView` (`series-rules.ts:65`: `{id,status,gameNo,winsNeeded,maxGames,pickDeadline,now,currentMatchId,ranked,you:{seat,wins,trioName,decks[{slot,name,cards,won,games}],pick,autoPick},opponent:{wins,decks[{slot,won}],picked},games[{gameNo,matchId,yourSlot,opponentSlot,youWentFirst,result,reason}],result}`) | 404 for a stranger |
| 23 | POST | `/api/series/:id/pick` | `api/series.ts:532` | active | `{slot:int, gameNo?:int>=1}` | `SeriesView` (a repeat of the same pick answers 200) | 400 (`slot_out_of_range`, `slot_won`, bad shape); 409 other refusals; 404 |
| 24 | POST | `/api/series/:id/forfeit` | `api/series.ts:552` | active | – | `SeriesView` | 409; 404 |
| 25 | GET | `/api/matches/:matchId/series` | `api/series.ts:563` | active | – | `{series: SeriesView|null}` | – |
| 26 | GET | `/api/tutorial` | `api/tutorial.ts:90` | active | – | `{progress:{completed:string[], hiddenChoice:{hidden,at}|null}}` | – |
| 27 | PUT | `/api/tutorial` | `api/tutorial.ts:95` | active | `{completed:string[] (lower-case slugs ≤40 chars, ≤32), hiddenChoice?:{hidden:boolean, at:int}|null}` | `{progress}` (union) | 400; 409 cap (`{limit:32}`) |
| 28 | GET | `/api/ranked` | `api/ranked.ts:458` | active | – | `{season, tag, rank: VisibleRank, streak, record:{games,wins,losses,draws}, badges: PeakBadge[]}` | – |
| 29 | GET | `/api/leaderboard` | `api/ranked.ts:461` | active | – | `{season, jlorious:[{position,tag,you}], tiers:[{tier,count,players:[{tag,division,pips,you}]}], raisins, you: VisibleRank}` | – |
| 30 | GET | `/api/matches/:matchId/ranks` | `api/ranked.ts:468` | active | – | `{ranked, seats:{p1:{tag,rank,you}, p2:{…}}}` | 404 unless the caller plays the match |
| 31 | POST | `/api/matches/:matchId/rematch` | `api/rematch.ts:207` | active | `{stakes: 1|2}` | `{matchId: string|null}` (non-null once both seats offered equal stakes) | 400; 404; 422 `double_requires_ranked`, `match_not_finished`, `series_game`; 409 `already_in_match` |
| 32 | GET | `/api/matches/:matchId/rematch` | `api/rematch.ts:251` | active | – | `{youOffered: 1|2|null, opponentOffer, opponentHere:boolean, matchId}` | 404; 422 `series_game` |
| 33 | GET | `/api/settings` | `api/settings.ts:122` | active | – | `{settings:{groups:{[groupId]:{at, values:{[name]: bool|number|string}}}}}` | – |
| 34 | PUT | `/api/settings` | `api/settings.ts:127` | active | `{groups:{[slug≤40]:{at:int, values:{[name]:bool|number|string≤40}}}}` (≤8 groups, ≤32 keys) | `{settings}` | 400; 409 cap (`{groups:8, bytes:4096}`) |
| 35 | GET | `/api/stats/cards` | `api/stats.ts:96` | none | Query: `patch`, `source` (`live`/`dev`/`ai`), `set`, `rarity`, `cost`, `card` | `PublicStatsCardsResponse` `{patch, previousPatch, gate:{cleared, liveGames, minLiveGames:1000}, source, sourceLabel, minSample:20, totalGames, cards:[…], summary:{…}}` | Cached `max-age=300` |
| 36 | GET | `/api/stats/cards/:id` | `api/stats.ts:261` | none | – | `{card:{id,name,cost,rarity}, patches[], byTurn[], coPlayed[]}` | 404; cached 300 s |
| 37 | GET | `/api/stats/player` | `api/stats.ts:379` | active | – | `{stats:object, isPrivate, updatedAt|null}` | – |
| 38 | PUT | `/api/stats/player` | `api/stats.ts:393` | active | `{stats?:object, isPrivate?:boolean}`, JSON ≤ 16,384 bytes | `{stats, isPrivate, updatedAt}` | 400 |
| 39 | GET | `/api/stats/players` | `api/stats.ts:421` | none | Query: `search`, `page` (1-based) | `{players: PublicPlayerSummary[], page, limit:50}` | Cached 60 s |

The same port also serves the WebSocket upgrade at **`GET /ws/match`** (§3). Answers outside the router: preflight `OPTIONS` → 204 (with CORS headers only for an allowed origin), an upgrade from a disallowed origin → raw `HTTP/1.1 403`, too many sockets per address → raw `HTTP/1.1 429`.

**Not present:** no health route (Render uses `/api/catalog`), no admin HTTP routes (admin is the CLIs in §5), and no E2E-only routes. E2E differs only by (a) the store/auth swap and fixture reseed at boot (`index.ts`), (b) the `seed` body field on `POST /api/queue`, `/api/rooms` and `/api/rooms/:code/join` (`seedOverrideOf`, `queue.ts:63`), and (c) Vite dev origins added to the allow-list (`browserOrigins`, `index.ts:158`).

Routes the web client calls (`apps/web/src/net/api.ts`): `/api/account`, `/api/auth/me`, `/api/catalog`, `/api/codes/redeem`, `/api/codes/status`, `/api/collection`, `/api/decks[/:id]`, `/api/leaderboard`, `/api/profile`, `/api/queue`, `/api/queue/population`, `/api/ranked`, `/api/rooms[/:code/join]`, `/api/settings`, `/api/stats/player`, `/api/stats/cards[/:id]`, `/api/stats/players`, `/api/trios/import`, `/api/trios/:id`, `/api/tutorial`, `/api/matches/:id/{series,ranks,rematch}`, `/api/series/:id[/pick|/forfeit]`. **Never called by the client:** `/api/auth/signup`, `/api/auth/signin`, `/api/catalog/:version`. The e2e specs call `/api/queue` (14×), `/api/auth/me`, `/api/rooms`, `/api/collection`, `/api/decks`, `/api/codes/*`, `/api/trios/import`, `/api/stats/*`, `/api/ranked`, `/api/profile` and `/api/catalog` directly. `e2e/cypress/e2e/99-online-smoke.cy.ts:163` uses the legacy `{deckIndex: 1}` body.

---

## 3. WebSocket protocol and the match actor

### 3.1 Handshake and authentication (`src/match/wsServer.ts`)

- Path **`/ws/match`** on the same HTTP listener. `ws` runs in `noServer` mode, and an upgrade on any other path is left alone.
- Order of checks: (1) `Origin` must be in `PUBLIC_ORIGINS` (trailing slash ignored, case-insensitive). No `Origin` header (a Node client) passes, and so does an empty allow-list. Failure → raw `HTTP/1.1 403 Forbidden`, socket destroyed. (2) At most `WS_MAX_CONNECTIONS_PER_ADDRESS = 10` open or in-progress sockets per rate-limit address (XFF per `TRUSTED_PROXY_HOPS`) → raw `HTTP/1.1 429`. (3) `wss.handleUpgrade` with `maxPayload: 65536` (an oversized frame is closed with **1009**). The server echoes only the subprotocol `jackioh.v1`.
- **Token sources, in order** (`tokenFrom`, line 124): `authorization: Bearer <t>` header (Node clients); `Sec-WebSocket-Protocol: jackioh.v1, <token>`; `?token=<t>` query (line 132). **The browser client uses `?token=`** (`apps/web/src/game/net.ts:446`), and so does the e2e `wsPlayer` (`e2e/support/tasks/wsPlayer.ts:156`).
- After the upgrade: `auth.verifyAccessToken` → `profiles.getByUserId` → `assertActive` → `matchId = ?matchId ?? profile.inMatchId`. That id must equal `profile.inMatchId`. Then `registry.attach(matchId, profile.id, socket)`.
- Every refusal sends the frame `{"type":"error","code":"forbidden","message":…}` and then closes with: **4401** (no/invalid token, no profile), **4403** (wrong match, ApiError other than 404, pending/banned), **4404** (not in a match, or registry `not_found`), **1011** (internal). Only the close code varies (R148).
- Text frames only. A binary frame is answered `{"type":"error","code":"malformed",…}`.
- **No heartbeats / ping-pong** anywhere, server or client.

### 3.2 Client → server frames (`parseClientMessage`, `protocol.ts:476`)

| `type` | Fields on the wire | Server behaviour |
| --- | --- | --- |
| `hello` | `token?`, `matchId?`, `roomCode?` (all ignored) | Push a fresh `view` + `clock` + `portraits` to this socket ("reconnect gets a fresh full view") |
| `joinRoom` | `roomCode`, `token?` | Always answered `error` `unsupported` ("join a room with POST /api/rooms/:code/join") |
| `action` | **`action: { type, ...ActionBody fields, nonce }`**; the nonce may be top-level `nonce` instead (`raw.nonce ?? parsed.nonce`, line 504). `playerId` is discarded | Nonce must be 1..128 chars and must not start with `srv-` (R270). Allowed types: `mulligan{keep:string[]}`, `play{instanceId, zone?{row:"units"|"backrow", lane:int≥0}, x?:int≥0, embiggen?:bool, tributes?:string[], targets?:Selection[], modes?:string[]}`, `attack{attackerId, targetId}`, `switchPosition{instanceId}`, `activatePower{instanceId, targets?}`, `answer{choiceId, selection:Selection[]}`, `offerDraw`, `answerDraw{accept}`, `concede`, `endTurn`, `setAutoEndTurn{enabled}`. Server-only and refused as malformed: `timeout`, `disconnectExpired`, `ceilingReached`. `Selection` = `{pick:"instance",instanceId}` / `{pick:"hero",player}` / `{pick:"zone",player,row,lane}` / `{pick:"mode",option}` / `{pick:"none"}` |
| `emote` | `emote: EmoteId` (one of the ten in `@jackioh/shared` `emotes.ts`) | Shared `emoteGate`. A pass is relayed to the opponent only; a fail is silently dropped |
| `aim` | `aim: null | {source, target}` of public handles (`@jackioh/shared` `aim.ts` `parseAim`) | Coalesced relay to the opponent, ≤1 per `AIM_RELAY_INTERVAL_MS = 100` per seat. An aim naming a hidden handle is dropped (or the opponent's arrow is cleared) |

Anything else → `error` `malformed` with a reason. The socket stays open.

### 3.3 Server → client frames

| `type` | Fields | When |
| --- | --- | --- |
| `view` | `view: PlayerView` (with `clockMs` overwritten by the actor from `clock.remainingFor(seat)`), `legal: ActionBody[]` = `legalActions(state, sameSeat)` | After every applied action (both seats), on attach, on `hello` |
| `ack` | `nonce`, `seq` | Reply to an accepted action. A replayed nonce gets the **original** ack (rebuilt from the log on restart) |
| `error` | `code` ∈ `malformed`/`illegal_action`/`forbidden`/`rate_limited`/`unsupported`/`match_over`/`internal`, `message`, `nonce?` | `illegal_action.message` is the reducer's reason verbatim; `internal` when the log append failed |
| `prompt` | `forYou:true, pendingFor, choiceId, kind, deadline` or `forYou:false, pendingFor, deadline` | Only when `pendingFor` changes or the set of seats owing a mulligan changes. `deadline` = `clocks.promptDeadline` |
| `clock` | `now` (server epoch ms), `clocks: {turnDeadline, promptDeadline, graceDeadline:{p1,p2}, ceilingAt}` | After every change (both seats), on attach (self + other), on `hello`, to the other seat on a disconnect |
| `portraits` | `p1, p2: PortraitId` | On attach and on `hello` |
| `emote` | `from, emote` | Relay to the opponent |
| `aim` | `from, aim|null` | Relay to the opponent; `null` when the sender's socket closes or the game ends |

Close codes the server sends: **1000** "replaced by a new socket" (same account attaches again) or "detached"; **1001** "the actor is going away" / "server closing"; **4410** `MATCH_VOIDED_CLOSE_CODE`, reason `voided` (R679); plus the handshake codes 4401, 4403, 4404, 1011 and 1009. The client treats 4401/4403/4404 as final, 4410 as closed, and anything else as reconnect with backoff `[250, 500, 1000, 2000, 5000]` ms (`apps/web/src/game/net.ts:151–161`).

### 3.4 Rate limits on the socket

- Actions: `MATCH_ACTIONS_PER_SECOND = 5` per seat over a 1,000 ms sliding window (R137) → `error rate_limited` with the nonce.
- Emotes: the shared `emoteGate` (silent drop). Aims: 100 ms coalescing. Frames: 64 KiB. Sockets per address: 10.

### 3.5 Clocks (`src/match/clock.ts`, R79/R268)

`TURN_CLOCK_SECONDS = 75`, `PROMPT_CLOCK_SECONDS = 30`, `MULLIGAN_CLOCK_SECONDS = 45`, `DISCONNECT_GRACE_SECONDS = 60`, `MATCH_CEILING_MINUTES = 120` (measured from `match.createdAt`).

- The turn clock belongs to the active player. It **pauses** while the non-active seat holds a prompt, and that prompt runs its own 30 s clock. After the pause it resumes with the time it had left.
- The mulligan clock is one deadline for both seats while `mulliganOwed` is non-empty and no prompt is pending. It is reported as `promptDeadline`, and on expiry each seat still owing gets one `timeout`.
- Grace runs per seat and is stored on the match. The turn clock keeps running during grace.
- Expiry → server action with nonce `srv-<kind>-<nextSeq>`: turn/prompt → `{type:"timeout"}` stamped with that player; mulligan → `timeout` per owing seat; grace → `{type:"disconnectExpired", player}` stamped with the disconnected seat (R146); ceiling → `{type:"ceilingReached"}` stamped with the active seat.
- Every clock change is persisted with `store.matches.setClocks(matchId, clocks)`, skipped when unchanged.

### 3.6 Actor lifecycle (`registry.ts` + `actor.ts`)

1. **Create** (`registry.start(StartMatchInput)`, called by queue pairing, room join, series `ensureSeriesGame` and rematch). It reads both seats' `lastBoards.get(profileId, "server")` and `lastBoards.sampleOthers([p1,p2], GLITCH_BOARDS_SAMPLED=2)`, builds the `MatchRow` (`status:"live"`, `clocks.ceilingAt = now + 120 min`, portraits), computes `dealt` = both seats when `mode` (from `input.mode`, else `matches.modeOf(id)`) is `random`, then `engine.beginGame(engine.createGame({seed, decks, lastBoards?, glitchBoards?, dealt?}))`. Only after that succeeds does it write `store.matches.create(row)` and create the actor with the begun state. `profiles.inMatchId` is set by the caller (queue: before start, in a tx; rooms: after start).
2. **Arm**: the actor syncs the clock from `engine.snapshot(state)` and persists the clocks. If the state already has a result (crash between the terminal action and the results write), it heals by running `onTerminal`.
3. **Attach** (`registry.attach` → `actorFor`). If the actor is not in memory: `store.matches.get` + `store.matches.actions` + `modeOf` → `engine.fold({seed, decks, log, lastBoards?, glitchBoards?, dealt?})`. Fold errors are logged `match.fold.errors` (alert) but the actor is rebuilt anyway. Concurrent rebuilds are deduped. Then: replace any old socket of that account, `clearGrace`, `startAbsentGrace` (R744: a seat never attached on this actor starts grace now, at the stored deadline if one exists), persist, push `view` + `clock` + `portraits` to self and `clock` to the other seat.
4. **Action** (serialized queue): dedupe the nonce; refuse with `match_over` once finished; `action = {...body, playerId: playing(home), nonce}`; `engine.reduce`. On error → `illegal_action`. On success → `store.matches.appendActions([{matchId, seq: nextSeq, action, at}])` **before** committing the in-memory state (append failure → `internal`). Then `afterChange`: snapshot, seat-swap check (R677), `clock.sync`, persist the clocks, push `view` ×2, prompts, `clock` ×2, and if there is a result, `onTerminal`.
5. **Disconnect**: `onSocketGone` → clear that seat's aim → `clock.startGrace(seat)` → persist → `clock` to the other seat.
6. **End** (`onTerminal`): stop the clock, clear aims. If the reason is `voided` → `onVoid` (close both sockets with 4410, drop from the registry, `voidMatch` → `matches.forgetVoided` + `resumeSeries`). Otherwise `recordResult({matchId, seats: creditedSeats() (swap-aware), outcome, turns, at, lastBoards: engine.lastBoards(state)})`, then `store.matches.finish(id, at)` and log `match.over`. **The actor stays in the registry map**, and its sockets stay open.
7. **Reaper** (every 30 s, `reapStuckMatches`): every `matches.live()` row past `ceilingAt` gets a `match-ceiling` draw with `turns = 0` and ratings unchanged, then `matches.stop(id)` if the actor is in memory, then `resumeSeries`.
8. **Restart survival**: nothing is rebuilt at boot. The first socket for a live match folds `(seed, decks, log [+ boards, dealt])`, re-arms a fresh turn clock, keeps `ceilingAt`, and restores stored grace deadlines.

### 3.7 Engine port: exactly what the server calls

`src/match/engine.real.ts` calls these `@jackioh/engine` exports (`REQUIRED_ENGINE_EXPORTS`, `engine.ts:150`): `createGame`, `beginGame`, `reduce`, `legalActions`, `viewFor`, `fold`, `hashState`, `mulliganOwed`, `createRng`, `lastBoardFor`, `summarizeGame`, `seatsSwapped`. It also reads raw `GameState` fields `turn`, `active`, `phase`, `pending.playerId` and `result` in `snapshot`, and imports `DECK_SIZE` from `@jackioh/engine/config`, `buildAiDeck` from `@jackioh/ai` and `registerAll` from `@jackioh/cards`.

| EnginePort method | Signature | Used by |
| --- | --- | --- |
| `createGame` | `({seed, decks:[string[],string[]], catalog?, lastBoards?, glitchBoards?, dealt?}) → State` | registry.start |
| `beginGame` | `(State) → {state, events, error?}` | registry.start |
| `reduce` | `(State, Action) → {state, events, error?}` | actor.applyAction |
| `legalActions` | `(State, PlayerId) → ActionBody[]` | actor.pushView |
| `viewFor` | `(State, PlayerId) → PlayerView` | actor (views, prompts, aim checks) |
| `fold` | `({seed, decks, log, catalog?, lastBoards?, glitchBoards?, dealt?}) → {state, errors:[{nonce,error}]}` | registry.rebuild, actor (when no state is passed) |
| `hashState` | `(State) → string` | tests only |
| `snapshot` | `(State) → {turn, active, pendingFor, mulliganOwed, phase, result, seatsSwapped}` | actor, clock |
| `dealRandomDeck` | `(seed) → string[]` (seeds `${seed}:p1-deck` / `${seed}:p2-deck`) | queue, rooms, rematch (All Random) |
| `lastBoards` | `(State) → [LastBoardEntry[], LastBoardEntry[]]` | actor.onTerminal |
| `summarizeGame` | `(FoldArgs) → GameSummary|null` | game-records (via `deps.games.summarize`) |

Glitch (R676–R679): seat swap via `seatsSwapped`; `glitchBoards` frozen on the match row (`matches.p1_glitch_board` / `p2_glitch_board`, migration 0024); void handled as above.

---

## 4. Store port, persistence, auth, env, config

### 4.1 `Store` (`src/api/ports.ts:1086`); every method returns a `Promise`

| Sub-store | Methods |
| --- | --- |
| root | `tx<T>(fn:(t:Store)=>Promise<T>)`, `redeem({profileId, codeHash|null, ipHash}) → "ok"|"not_pending"|"email_unverified"|"rate_limited_profile"|"rate_limited_ip"|"circuit_open"|"invalid_code"`, `purgeExpired({codeAttemptsBefore, matchActionsEndedBefore}) → {codeAttempts, matchActions}` |
| `profiles` | `getById(id)`, `getByUserId(userId)`, `getMany(ids)`, `create({userId,email,rating,at,displayName?})`, `setStatus(id,status)`, `setGlicko(id, glicko)`, `setRating(id, n)` (**no caller in src/api**), `setDisplayName(id, name|null)`, `setInMatch(id, matchId|null)`, `remove(id) → boolean` |
| `codes` | `insert(InviteCode)`, `findByHash(hash)`, `claim(codeId, now) → boolean`, `logAttempt({profileId|null, ipHash, result:"ok"|"rejected", reason, at})`, `countAttemptsByProfile(id, since)`, `oldestAttemptAtByProfile(id, since)`, `countAttemptsByIp(ipHash, since)`, `countFailures(since)` |
| `collection` | `get(profileId) → {cardId,quantity}[]`, `upsertQuantities(profileId, entries)`, `appendGrants({profileId,cardId,delta,reason,at}[])` |
| `decks` | `list(profileId)` (oldest first, then id), `get(deckId)`, `upsert(SavedDeck, maxDecks) → "created"|"updated"|"limit"|"not_owner"`, `remove(profileId, deckId) → boolean` |
| `trios` | `list`, `get`, `upsert(SavedTrio, maxTrios) → …|"unknown_deck"`, `remove` |
| `matches` | `create(MatchRow)`, `get(id)`, `appendActions(MatchActionRow[])`, `actions(id) → {matchId,seq,action,at}[]`, `setClocks(id, MatchClocks)`, `finish(id, at)`, `live() → MatchRow[]`, `modeOf(id) → "bo1"|"bo3"|"random"|null`, `discardOpen(id)`, `forgetVoided(id)` |
| `rooms` | `create(Room) → boolean` (false = code taken), `get(code)`, `claim(code, guestProfileId, matchId, at) → Room|null` (single-shot) |
| `tickets` | `insert`, `get`, `openForProfile`, `listOpen`, `countOpen`, `countOpenByMode → Record<QueueMode,number>`, `claimPair(aId, bId, matchId, at) → boolean`, `cancel(id, at)` |
| `results` | `insert(ResultRow)` (throws `DuplicateResultError` on a second row), `getByMatch(id)`, `recordFor(profileId) → {wins,losses,draws}` |
| `series` | `create`, `get`, `update(next) → boolean` (compare-and-set on `version`), `byMatch(nextMatchId)`, `withGame(matchId)`, `activeFor(profileId)`, `active()` |
| `ranked` | `lockSeasons()`, `seasons()`, `createSeason(s) → boolean`, `ratedPlayers()`, `resetRatings(changes)`, `standings(seasonId)`, `rank(seasonId, profileId)`, `ranksOf(profileId)`, `putRank(row)`, `notePeakJlorious(seasonId, profileId, position)`, `bot(botId)`, `putBot(bot)`, `recordGame(RatedGameRow)`, `game(id)` |
| `tutorial` | `get(profileId)`, `merge({profileId, completed, hiddenChoice, at}, maxLessons) → {kind:"merged",progress}|{kind:"limit"}` |
| `playerSettings` | `get(profileId)`, `merge({profileId, groups, at}, {maxGroups, maxBytes}) → merged|limit` |
| `lastBoards` | `get(profileId, "server"|"practice")`, `put(profileId, kind, board, at)`, `sampleOthers(excludeIds, count) → LastBoardEntry[][]` |
| `gameRecords` | `insert(GameRecord) → boolean`, `list({source, mode, patch}) → GameRecord[]` |
| `playerStats` | `get(profileId)`, `put(profileId, stats, isPrivate, at)`, `listPublic({search?, limit, offset})` |

Key row types (`ports.ts`): `MatchRow {id, seed, players:[id,id], decks:[string[],string[]], catalogVersion, ranked?, mode?, stake?:1|2, status:"live"|"finished", createdAt, finishedAt, clocks, lastBoards?, glitchBoards?, portraits?}`; `MatchClocks {turnDeadline, promptDeadline, graceDeadline:{p1,p2}, ceilingAt}`; `Ticket {id, profileId, rating, mode, deck, portrait?, trio, catalogVersion, enqueuedAt, status, matchId}`; `Room {code, hostProfileId, mode, hostDeck, hostPortrait?, hostTrio, catalogVersion, createdAt, expiresAt, guestProfileId, matchId}`; `SeriesRow` (`ports.ts:774`, with a `version` for CAS).

Other ports: `MatchDirectory {start(StartMatchInput), has(id), stop(id), presenceOf(id) → {p1,p2}|null}`; `AuthProvider {verifyAccessToken, signUp, signInWithPassword, deleteUser?}`; `Hashes {code, ip}`; `Ids {uuid, seed, code(len)}`; `Timers {now, after(ms, fn) → {cancel}}`; `Logger {info, warn, alert}`; `GameRecorder {patch, summarize}`.

Match-side contracts (`src/match/contracts.ts`): `Socket {isOpen, send(text), close(code?, reason?), attach({message, close})}`; `MatchClock {sync(ClockView), startGrace(player, deadline?), clearGrace(player), snapshot(), remainingFor(player), stop()}`; `RecordResult(RecordResultInput) → ResultRow`; `VoidMatch({matchId, players, at})`; `ActorDeps {store, timers, config, log, engine, createClock, recordResult, voidMatch}`.

### 4.2 `src/db/**` implementation notes

- `store.ts` uses `pg` `Pool` (`keepAlive: true`) and one session helper. Every transaction runs `begin`, then the role switch to `service_role` and `request.jwt.claim.sub` via `set_config(…, true)`. SQL functions it calls: `app.redeem_invite_code`, `app.upsert_deck`, `app.upsert_trio`, `app.append_match_action`, `app.claim_ticket_pair`, `app.live_matches` (2×), `app.merge_tutorial_progress`, `app.merge_player_settings`, `app.forget_voided_match`, `app.purge_expired_rows`, `app.current_profile_id`, `app.profile_is_active`. Rooms are rows of `public.matches` in status `open`. Postgres `23505` on `results_pkey` becomes `DuplicateResultError`.
- `store.ts:2875` "KNOWN DIVERGENCES" from the in-memory store: redemption limits (§9.4's 5/20 are hard-coded in SQL), attempt reason, profile email, foreign keys, deck/trio strictness, tutorial/settings strictness, caps (also `app.settings.max_saved_decks`), timestamps, launch grant, rooms, reserved match ids, deleted accounts, clocks (grace deadline columns).

### 4.3 Migrations (`apps/server/src/db/migrations`, 26 files, 6,068 lines)

Schema totals: 25 `public` tables (bot_ratings, cards, code_attempts, collection, collection_grants, decks, game_records, invite_codes, last_boards, loadout_deck_cards, loadout_decks, loadouts, match_actions, matches, player_settings, player_stats, profiles, rated_games, results, season_ranks, seasons, series, tickets, trios, tutorial_progress) plus `app.settings`, `app.migrations`; 31 `app.*` functions; 10 triggers (`on_auth_user_created`, `profiles_grant_launch_collection`, `profiles_release_open_matches`, `settings_grant_catalog_growth`, `match_actions_deny_mutation`, `collection_grants_deny_mutation`, four `*_set_updated_at`).

| File | Lines | One-line summary |
| --- | --- | --- |
| `0001_profiles_and_invites.sql` | 738 | `app` schema, `app.settings`, `profiles` (auth.users trigger → pending), `invite_codes`, `code_attempts`, `app.redeem_invite_code` (six steps), helpers `current_profile_id`, `profile_is_active`, `deny_row_mutation`, `catalog_version`, `setting` |
| `0002_collection.sql` | 489 | `cards` (+tag check), `collection`, `collection_grants` (append-only), `app.grant_cards`, the launch grant on activation, `app.assert_catalog_version` |
| `0003_loadouts.sql` | 525 | `loadouts`, `loadout_decks`, `loadout_deck_cards`, L4 unique index, `app.save_loadout`, `app.resolve_deck` (retired by R254, kept) |
| `0004_matches.sql` | 932 | `tickets`, `matches`, `match_actions`, `results`; `app.create_room`, `join_room`, `start_match`, `append_match_action`, `claim_ticket_pair`, `end_match`, `live_matches`, `reap_stale_matches` |
| `0005_service_role_reads_auth_users.sql` | 43 | Grants service_role SELECT on `auth.users` |
| `0006_redeem_ip_lock.sql` | 201 | Replaces `app.redeem_invite_code` so the per-IP window holds under concurrency |
| `0007_decks_and_trios.sql` | 625 | `decks`, `trios`, `app.upsert_deck`, `app.upsert_trio`; data migration loadout → 3 decks + "My trio" |
| `0008_queue_modes.sql` | 145 | `tickets.mode`, `frozen_trio`; `matches.room_mode`, `room_trio` |
| `0009_series.sql` | 153 | `series` (server-only, CAS on `version`) |
| `0010_jlockeed_tag.sql` | 41 | `cards_tags_check` + Jlockeed |
| `0011_tutorial_progress.sql` | 256 | `tutorial_progress`, `app.merge_tutorial_progress` |
| `0012_account_deletion.sql` | 219 | FK changes so `delete from auth.users` cascades; `app.release_open_matches`; updated `deny_row_mutation` |
| `0013_retention_purge.sql` | 111 | `app.purge_expired_rows` (rewritten once; see `REWRITTEN`) |
| `0014_game_records.sql` | 113 | `game_records` (server-only) |
| `0015_classic_sets_tags.sql` | 46 | Tag check + Book, Pancake, AI |
| `0016_catalog_growth_grants.sql` | 73 | Trigger `app.on_catalog_version_stamped`: a new catalog version grants its new cards |
| `0017_last_boards.sql` | 77 | `last_boards`, `matches.p1_last_board` / `p2_last_board` |
| `0018_player_settings.sql` | 230 | `player_settings`, `app.merge_player_settings` |
| `0019_hero_portraits.sql` | 253 | Deck portraits; replaces `app.upsert_deck`; match/ticket portrait columns |
| `0020_plague_tag.sql` | 38 | Tag check + Plague |
| `0021_player_stats.sql` | 33 | `player_stats` |
| `0022_ranked_ladder.sql` | 472 | Fractional Glicko columns, `seasons`, `season_ranks`, `bot_ratings`, `rated_games`, `ranked` flags, column-whitelist grants; replaces `app.end_match`, `app.reap_stale_matches` |
| `0023_rematch.sql` | 35 | `matches.mode`, `matches.stake` |
| `0024_glitch_boards.sql` | 93 | `matches.p1_glitch_board` / `p2_glitch_board`, `app.forget_voided_match` |
| `0025_patch_retcon.sql` | 88 | Renames rows filed under the old patch names (R743) |
| `0026_catalyst_prime_acclaimed_tags.sql` | 39 | Tag check + Catalyst, Prime, Acclaimed |

Migrations are append-only. `apps/server/test/db/migrations-pinned.test.ts` pins every file's checksum.

### 4.4 Database test suites

- **`pnpm test:sql`** → `sh apps/server/test/sql/run.sh` (151 lines). Needs only Docker (postgres image). It applies `00_supabase_stub.sql` (roles `anon`/`authenticated`/`service_role`, `auth.users`, `auth.uid()`), then 0001–0006, then `03b_legacy_loadout_seed.sql`, then 0007–0026, then the assertion files `01_schema_invariants.sql` (932), `02_rls_as_client.sql` (943), `03_match_lifecycle.sql` (1408), `04_decks_and_series.sql` (802), `05_tutorial_progress.sql` (222), `06_account_deletion.sql` (218), `07_retention_purge.sql` (80), `08_game_records.sql` (85), `09_catalog_growth.sql` (81), `10_last_boards.sql` (215), `11_player_settings.sql` (256), `12_ranked.sql` (475), `13_glitch.sql` (114), `14_patch_retcon.sql` (151). A gate on `ON_ERROR_STOP` plus a grep for FAIL. **This suite is pure SQL and survives a Rust rewrite unchanged**, as long as the schema is kept.
- **`pnpm test:db`** → `sh apps/server/test/db/run.sh` (74 lines): Docker postgres:16, `test/db/bootstrap.sql` (57), the real `migrate.ts`, `test/db/grants.sql` (11), then vitest with `test/db/vitest.config.ts` (`*.spec.ts`): `contract.postgres.spec.ts` (runs `test/db/contract.ts`, **2,251 lines, ~101 `it`**, the Store contract also run in memory by `contract.memory.test.ts`), `postgres.spec.ts` (743, ~31: role/claim inside the store tx, etc.), `redeem-race.postgres.spec.ts` (+ `redeem-race.ts`, 245, ~8), `seed-catalog.spec.ts` (106, ~2). `KEEP_DB=1` and `DB_PORT` are supported.
- **`pnpm test:deploy`** → `sh apps/server/test/deploy/rehearse.sh` (205 lines). It creates a database owned by a **non-superuser** `migrator`, installs the bootstrap as superuser, then reads **render.yaml's own `startCommand`** and non-secret env (`NODE_ENV`, `TRUSTED_PROXY_HOPS`). `CATALOG_VERSION` starts stale on purpose and must be overwritten from `patches.json`. It sets a fake `RENDER_GIT_COMMIT` and probes `GET /api/catalog` within `READY_SECONDS` for render.yaml's version plus the `x-deployed-commit` header. It checks memory under `MEMORY_LIMIT_MB` (512 MB free instance less headroom), then boots a second time and requires that nothing migrates.
- **CATALOG_VERSION derivation**: render.yaml `startCommand` = `CATALOG_VERSION=$(node scripts/catalog-version.mjs) && export CATALOG_VERSION && pnpm --filter @jackioh/server release && pnpm --filter @jackioh/server start`. `scripts/catalog-version.mjs` prints `packages/cards/patches/patches.json`'s last entry `.version` and exits 1 with nothing on stdout if there is none. Server-side, `loadCurrentPatch()` (`catalog.ts`) reads the same file for the game-record patch, and `loadPatchVersion()` (`ranked.ts:65`) reads it for the season id.

### 4.5 Env vars (`src/env.ts`)

| Var | Type | Default | Required | Notes |
| --- | --- | --- | --- | --- |
| `SUPABASE_URL` | https URL | – | yes | Also derives the JWKS URL and `${url}/auth/v1` as the issuer |
| `SUPABASE_SECRET_KEY` | string | – | yes | `sb_secret_…` or a legacy service_role JWT; used for admin `getUserById` / `deleteUser` and as `apikey` on `/auth/v1/user` |
| `DATABASE_URL` | string | – | yes | `pg` connection; E2E placeholder `memory://e2e-fixture-store` |
| `SUPABASE_JWKS_URL` | https URL | `${SUPABASE_URL}/auth/v1/.well-known/jwks.json` | no | |
| `SUPABASE_JWT_SECRET` | string | undefined | no | HS256 fallback |
| `CODE_PEPPER` | string ≥32 chars | – | yes | Peppers `${p}:code` / `${p}:ip` (HMAC-SHA256) |
| `PORT` | int 1–65535 | 8787 | no | Render injects it |
| `PUBLIC_ORIGINS` | comma list | – | **yes** (error if empty) | CORS and the WS Origin check |
| `NODE_ENV` | development/test/production | development | no | |
| `E2E` | `1`/`true`/`0`/`false` | false | no | Refused together with production |
| `CATALOG_VERSION` | string | – | yes | E2E default `v0.2.11` (`index.ts:126`) |
| `TRUSTED_PROXY_HOPS` | int 0–5 | 0 | no | render.yaml sets 1 |
| `RENDER_GIT_COMMIT` | hex 7–64 | undefined | no | → `DEPLOYED_COMMIT` → `x-deployed-commit` |

Script-only vars: `SEED_ACCOUNTS_PROJECT`, `SEED_ACCOUNTS_PASSWORD` (seed-accounts), `INIT_CWD` (stats:import). Client half (`PUBLIC_ENV_VARS`): `VITE_SUPABASE_URL`, `VITE_SUPABASE_PUBLISHABLE_KEY`, `VITE_SERVER_HTTP_URL`, `VITE_SERVER_WS_URL`, `VITE_CATALOG_VERSION`, `VITE_AUTH_OAUTH_PROVIDERS`. Production values are in `apps/web/.env.production`: `VITE_SERVER_HTTP_URL=https://jackioh-server.onrender.com`, `VITE_SERVER_WS_URL=wss://jackioh-server.onrender.com/ws/match`.

### 4.6 Server config constants (`src/config.ts`, 782 lines), grouped

- **Clocks (R79/R268)**: `TURN_CLOCK_SECONDS 75`, `PROMPT_CLOCK_SECONDS 30`, `MULLIGAN_CLOCK_SECONDS 45`, `DISCONNECT_GRACE_SECONDS 60`, `MATCH_CEILING_MINUTES 120`, plus `*_MS` variants.
- **Codes**: `CODE_ALPHABET "23456789ABCDEFGHJKLMNPQRSTUVWXYZ"`, `ROOM_CODE_LENGTH 6`, `INVITE_CODE_LENGTH 16`, `INVITE_CODE_GROUP_SIZE 4`, `INVITE_CODE_SEPARATOR "-"`, `CODE_INPUT_MAX_LENGTH 64`, `INVITE_CODE_FORMAT`, `ROOM_CODE_FORMAT`.
- **Proxy / HTTP**: `DEFAULT_TRUSTED_PROXY_HOPS 0`, `MAX_TRUSTED_PROXY_HOPS 5`, `IPV6_RATE_LIMIT_PREFIX_BITS 56`, `API_MAX_BODY_BYTES 65536`, `API_REQUESTS_PER_MINUTE 300`, `MATCH_ACTIONS_PER_SECOND 5`, `AIM_RELAY_INTERVAL_MS 100`, `WS_MAX_CONNECTIONS_PER_ADDRESS 10`.
- **Auth** (mostly client-side): `AUTH_PASSWORD_MIN_LENGTH 12`, `AUTH_PASSWORD_MAX_LENGTH 72`, `AUTH_EMAIL_RESEND_COOLDOWN_SECONDS 60`, `AUTH_PENDING_ADDRESS_TTL_SECONDS 86400`, `AUTH_SESSION_REFRESH_MARGIN_SECONDS 60`, `AUTH_SESSION_RENEWAL_FLOOR_SECONDS 10`, `AUTH_SIGN_OUT_WAIT_SECONDS 5`, `AUTH_SESSION_LIVE_CACHE_SECONDS 30` (server), `AUTH_PROVIDER_TIMEOUT_SECONDS 30` (server), `API_REQUEST_TIMEOUT_SECONDS 75` (client), `GATE_SLOW_NOTICE_SECONDS 5`, `CODE_STATUS_RECHECK_FLOOR_SECONDS 1`.
- **Redemption**: `CODE_ATTEMPTS_PER_PROFILE_PER_HOUR 5`, `CODE_ATTEMPTS_PER_IP_PER_HOUR 20`, `CODE_ATTEMPT_WINDOW_SECONDS 3600`, `REDEMPTION_RESPONSE_FLOOR_MS 250`, `REDEMPTION_IDENTICAL_ERROR "This invite code is invalid."`, `REDEMPTION_CIRCUIT_FAILURE_THRESHOLD 100`, `REDEMPTION_CIRCUIT_WINDOW_SECONDS 600`.
- **Retention**: `CODE_ATTEMPT_RETENTION_DAYS 30`, `MATCH_ACTION_RETENTION_DAYS 90`, `RETENTION_PURGE_INTERVAL_SECONDS 3600`.
- **Queue**: `RATING_WINDOW_START 100`, `RATING_WINDOW_WIDEN_BY 50`, `RATING_WINDOW_WIDEN_EVERY_SECONDS 10`, `RATING_WINDOW_UNCAPPED_AFTER_SECONDS 60`, `MATCHMAKER_SWEEP_INTERVAL_SECONDS 3`, `ratingWindow()`.
- **Ranked**: `RATING_START 1000`, `RATING_DEVIATION_START 350`, `RATING_VOLATILITY_START 0.06`, `GLICKO_TAU 0.5`, `GLICKO_SCALE 400/ln10`, `GLICKO_CONVERGENCE 1e-6`, `GLICKO_MAX_ITERATIONS 100`, `RANK_TIER_PERCENTS`, `RANK_DIVISIONS_PER_TIER 3`, `RANK_PIPS_PER_DIVISION 3`, `RANK_PLACEMENT_GAMES 5`, win/loss/streak pips, convergence, `JLORIOUS_SIZE 100`, `PLAYER_TAG_LENGTH 6`, `SEASON_RESET_STRENGTH 0.5`, `SEASON_RESET_DEVIATION_BOOST 150`.
- **Matches**: `MATCH_REAPER_INTERVAL_SECONDS 30`, `RESULT_WRITE_ATTEMPTS 3`, `GLITCH_BOARDS_SAMPLED 2`, `MATCH_VOIDED_CLOSE_CODE 4410`, `REMATCH_OFFER_TTL_MS 600000`, `MATCH_FOUND_NAV_DELAY_MS 1200` (client).
- **Decks**: `MAX_SAVED_DECKS 10`, `MAX_SAVED_TRIOS 5`, `DECK_NAME_MAX_LENGTH 40`, `DRAFT_ISSUES_REPORTED_MAX 50`, `DECK_CODE_VERSION 2`, `DECK_CODE_CORE_ONLY_VERSION 1`, `CATALOG_NUMBER_SET_OFFSETS`, `DECK_CODE_MAX_INPUT_LENGTH 512`, `DECK_AUTOSAVE_DEBOUNCE_MS 800`, `DECK_AUTOSAVE_RETRY_SECONDS 5`, `TRIO_CODE_VERSION 2`, `TRIO_CODE_CORE_ONLY_VERSION 1`, `TRIO_CODE_MAX_INPUT_LENGTH 2048`.
- **Series**: `SERIES_WINS_NEEDED 3`, `SERIES_MAX_GAMES 7`, `SERIES_PICK_SECONDS 60`, `SERIES_SWEEP_INTERVAL_SECONDS 5`, `SERIES_START_GRACE_SECONDS 15`, `SERIES_START_GIVE_UP_SECONDS 120`, `SERIES_WRITE_ATTEMPTS 3`, `SERIES_POLL_SECONDS 2` (client).
- **Tutorial / settings / stats**: `TUTORIAL_LESSONS_MAX 32`, `TUTORIAL_LESSON_ID_MAX_LENGTH 40`, `PLAYER_SETTINGS_GROUPS_MAX 8`, `_KEYS_MAX 32`, `_NAME_MAX_LENGTH 40`, `_TEXT_MAX_LENGTH 40`, `_BYTES_MAX 4096`, `PUBLIC_STATS_MIN_LIVE_GAMES 1000`, `CARD_STATS_MIN_SAMPLE 20`, `CARD_STATS_CURVE_TOP 6`, `CARD_STATS_CACHE_TTL_SECONDS 300`, `PLAYER_STATS_CACHE_TTL_SECONDS 60`, `PLAYER_STATS_PAGE_LIMIT 50`, `PLAYER_STATS_BYTES_MAX 16384`.
- `src/api/deps.ts:57` `roomCodeTtlMs: 15 * 60 * 1000` is a literal with no named constant (a rule-9 gap).

### 4.7 Auth details

- **JWT verification** (`createSupabaseAuth`, `auth.ts:417`). Tier 1: `jose.jwtVerify` against the remote JWKS (`createRemoteJWKSet`) with `issuer = ${SUPABASE_URL}/auth/v1` and `audience = "authenticated"`. Tier 2: HS256 with `SUPABASE_JWT_SECRET` if it is set. Tier 3: only for a token whose header says HS256 while no secret is configured, `GET ${authBase}/user` with `apikey` + bearer.
- **Session liveness (R194)**: if the token has `session_id`, `GET /auth/v1/user` with the token (30 s timeout) is called. A 401/403 answer means the session ended → invalid. A positive answer is cached per session for 30 s. "Unavailable" keeps the identity, and the email then counts as unverified unless it is already cached.
- **Email verification**: from the provider (admin `getUserById(...).email_confirmed_at`, or `/user`), never from token metadata. Only a positive answer is cached, for 30 s per user.
- **aal2 (R665)**: if the provider user has a verified TOTP factor, a token whose `aal` is not `aal2` is refused. MFA enrolment is remembered per user across outages.
- **AuthUser** = `{userId: sub, email, emailVerified, appMetadata: app_metadata claim}`.
- **Invite gate**: `profiles.status` pending/active/banned. Pending accounts can only use `user` routes (`/api/auth/me`, `/api/codes/*`, `/api/queue/population`, `DELETE /api/account`). Redemption flips pending → active inside `app.redeem_invite_code`, and the activation trigger grants the collection. The profile row is created by the `auth.users` trigger, or lazily by `resolveCaller`.
- **E2E fixtures (R144)**: under `E2E=1`, `createE2EAuth` accepts the bearer tokens `e2e-token-p1`, `e2e-token-p2` (active) and `e2e-token-pending`, and `seedE2EFixtures` resets the in-memory store at every boot (profiles, launch grant, codes good/expired/exhausted) **before** the port opens.

---

## 5. Server scripts and CLIs (`apps/server/package.json`)

All run under `tsx --env-file-if-exists=.env`.

| Script | Command | File | What it does |
| --- | --- | --- | --- |
| `dev` | `tsx watch … src/index.ts` | `src/index.ts` | Server with reload |
| `start` | `tsx … src/index.ts` | `src/index.ts` | Production entry (render.yaml) |
| `db:migrate` | `src/db/migrate.ts` | 156 lines | Apply `migrations/*.sql` in lexical order with the ledger and checksum |
| `db:seed-catalog` | `src/db/seed-catalog.ts` | 187 | Upsert `public.cards` from `packages/cards/catalog.json` (array, `{cards}` or id-keyed object) at `CATALOG_VERSION`, set `app.settings.catalog_version` (one tx). The settings write fires the 0016 growth-grant trigger |
| `release` | `db:migrate && db:seed-catalog` | – | Run by render.yaml before every boot |
| `codes:mint` | `src/db/mint-code.ts` | 113 | Mint one invite code (`--max-uses=N`, `--expires-in-days=N`); plaintext to stdout, metadata to stderr |
| `db:seed-accounts` | `src/db/seed-accounts.ts` | 286 | Test accounts through the GoTrue admin API, activated, with starter decks + trio; guarded by `SEED_ACCOUNTS_PROJECT`/`SEED_ACCOUNTS_PASSWORD`, refuses in production |
| `db:season-start` | `src/db/season-start.ts` | 109 | Open the build's season (R609) with `--dry-run` (rollback + report) |
| `stats:cards` | `src/db/card-stats.ts` | 138 | Card win-rate report from `game_records` (`--source`, `--mode`, `--patch`, `--pilot`, `--card`, `--json`) |
| `stats:import` | `src/db/import-dev-records.ts` | 86 | Load an `ai:stats` JSONL (dev records) into `game_records` |
| `typecheck` | `tsc -p tsconfig.json` | – | |
| `test` | `vitest run` | `vitest.config.ts` | Project `server`, `test/**/*.test.ts` |

Root scripts that touch the server: `test:sql`, `test:db`, `test:deploy` (above) and `typecheck` (`tsc -p apps/server/tsconfig.json`). `packages/ai/scripts/stats.ts` produces the JSONL that `stats:import` reads.

---

## 6. `apps/server/test`

Totals: 24,591 lines of TS, 6,191 lines of SQL, about 866 `it(`. Doubles live in `test/fakes/`: `deps.ts` (494, `createManualTimers`, scripted auth, small catalog), `engine.ts` (659, scripted `EnginePort` with cards `test-prompt-self`, `test-prompt-enemy`, `test-lethal`, and `{mulligan:true}`), `socket.ts` (91), `store.ts` (546, built on `memory-stores.ts`).

### 6.1 Essential behavioural contracts (port these semantics into Rust tests)

| Topic | File (lines, ~it) |
| --- | --- |
| Actor protocol (nonce dedupe, seat stamping, views and legal per seat, prompts, clocks, mulligan, draw/concede, flood) | `match/actor.test.ts` (1991, 46) |
| Crash recovery by fold | `match/recovery.test.ts` (622, 13), `match/series-recovery.test.ts` (488, 14) |
| Clock (R79/R268/R744) | `match/clock.test.ts` (451, 22) |
| WS upgrade, auth, close codes | `match/ws-server.test.ts` (157, 6) |
| Rooms | `match/rooms.test.ts` (704, 22) |
| Glitch swap/boards/void | `match/glitch.test.ts` (324, 11) |
| Aim relay | `match/aim.test.ts` (232, 8) |
| All Random dealt decks | `match/dealt-deck.test.ts` (128, 3) |
| Last boards | `match/last-boards.test.ts` (225, 3) |
| Real engine through the port | `match/engine.real.test.ts` (198, 5) |
| Redemption, breaker, timing | `api/codes.test.ts` (807, 30), `api/redeem-feedback.test.ts` (673, 29) |
| JWT, session, aal2 | `api/auth.test.ts` (984, 30), `api/account.test.ts` (254, 12) |
| Decks/trios/import | `api/decks.test.ts` (1032, 53) |
| Queue and pairing | `api/queue.test.ts` (1241, 35) |
| Conquest | `api/series-rules.test.ts` (799, 39), `api/series.test.ts` (661, 20) |
| Results, rating, reaper | `api/results.test.ts` (579, 20), `api/ranked.test.ts` (354, 12) |
| Rate limit / client address / CORS | `api/rate-limit.test.ts` (237, 12), `api/client-address.test.ts` (637, 44), `api/cors.test.ts` (250, 7) |
| Catalog route + header | `api/catalog.test.ts` (421, 23) |
| Collection, tutorial, settings, rematch, stats, records, retention | `api/collection.test.ts` (333, 20), `api/tutorial.test.ts` (174, 10), `api/settings.test.ts` (233, 14), `api/rematch.test.ts` (638, 19), `api/stats.test.ts` (509, 7), `api/game-records.test.ts` (236, 7), `api/retention.test.ts` (69, 2) |
| Pure ranked math | `ranked/glicko2.test.ts` (125, 10), `ranked/ladder.test.ts` (239, 16), `ranked/season.test.ts` (66, 6) |
| **Store contract (both stores)** | `db/contract.ts` (2251, ~101) via `contract.memory.test.ts` (13) / `contract.postgres.spec.ts` (10); `db/redeem-race.ts` (245, 8) via the memory/postgres pair; `db/postgres.spec.ts` (743, 31) |
| **Schema (keep as-is)** | `test/sql/*.sql` (6,191 lines) + `run.sh` |
| Deploy rehearsal (rewrite for the Rust binary) | `test/deploy/rehearse.sh` (205) |

### 6.2 Incidental or TS-implementation-specific

| File (lines, ~it) | Why |
| --- | --- |
| `validator-single-source.test.ts` (245, 6) | Asserts `@jackioh/validator` has one import site in TS |
| `api/code-input-parity.test.ts` (268, 8) | Parity between server config and `@jackioh/shared` code-input rules (becomes Rust↔TS parity) |
| `api/e2e.test.ts` (670, 32) | E2E fixture mode |
| `env-deployed-commit.test.ts` (41, 4) | Env parsing of `RENDER_GIT_COMMIT` |
| `db/migrations-pinned.test.ts` (75, 3) | Checksum pins (port it: the pins are the compatibility contract) |
| `db/pool-error.test.ts` (156, 6), `db/store-guards.test.ts` (32, 2), `db/loadstore-boot.ts` (45) | `pg` pool and dynamic-import plumbing |
| `db/seed-catalog.test.ts` (121, 7), `db/seed-catalog.spec.ts` (106, 2), `db/seed-accounts.test.ts` (58, 6), `db/mint-code.test.ts` (85, 7), `db/season-start.test.ts` (119, 4), `db/card-stats.test.ts` (157, 9) | CLI wrappers |
| `db/harness.ts` (236), `db/bootstrap.sql` (57), `db/grants.sql` (11), `db/run.sh` (74), `db/vitest.config.ts` (31) | Harness |

---

## 7. Web client: engine, AI, validator and catalog use

### 7.1 Engine-facing modules (line counts)

| File | Lines | Role |
| --- | --- | --- |
| `apps/web/src/game/engine.ts` | 108 | Web `EnginePort` type: `createGame({seed, decks, catalog?, handicaps?})`, `beginGame`, `reduce`, `legalActions`, `viewFor`, `hashState`, `catalog?()`; `REQUIRED_ENGINE_EXPORTS`; `EngineUnavailableError`; `setEnginePort`/`injectedEnginePort` (tests); `loadEnginePort()` = code-split `import("./engine.real.ts")` (line 100) |
| `apps/web/src/game/engine.real.ts` | 54 | `registerAll()` from `@jackioh/cards`; maps `@jackioh/engine` functions; `catalog = registeredCatalog` |
| `apps/web/src/game/hotseat.ts` | 210 | `createHotseat({seed, decks, engine, handicaps…})` → `HotseatSession {seed, seat, setSeat, view, legal, dispatch(body) → {events, error?}, log, hash, state, subscribe}`; nonces via `defaultNonce(n)` |
| `apps/web/src/routes/dev/hotseat.tsx` | 481 | `/dev/hotseat` (DEV_ONLY): loads the port, reads `?seed&a&b` + `window.__jackiohE2E` / localStorage `jackioh.e2e.decks`, installs `window.__jackioh` |
| `apps/web/src/practice/core.ts` | 416 | Practice game in the worker: `createPracticeCore(env).handle(PracticeRequest) → PracticeResponse` |
| `apps/web/src/practice/practice.worker.ts` | 32 | Module worker entry: waits `saves.ready`, then `postMessage(core.handle(event.data))` |
| `apps/web/src/practice/host.ts` | 174 | `createPracticeHost({forceInThread?, env?})`: `new Worker(new URL("./practice.worker.ts", import.meta.url), {type:"module"})` (line 55), or an in-thread host (dynamic `import("./core.ts")`, setTimeout 0, used under jsdom). API `request(body) → Promise<PracticeResponse>`, `dispose()` |
| `apps/web/src/practice/protocol.ts` | 97 | `PracticeRequest`: `start{config}`, `resume{config}`, `act{action}`, `aiStep`, `catalog`, `debug`. `PracticeResponse`: `started{snapshot, defs, aiSeat}`, `snapshot{snapshot}`, `catalog{defs}`, `debug{debug}`, `failed{message}`. `PracticeSnapshot {view, legal, aiToAct, error, lastBoard?}`. `PracticeStartConfig {seed, difficulty, humanSeat, deck: random|preset{id}|saved{index,cards,portrait?}, lesson?, lastBoard?}`. `PracticeDebug {seed, decks, handicaps, log, state (raw JSON), hash, difficulty, humanSeat, lesson?, lastBoards?, dealt?}` |
| `apps/web/src/practice/controller.ts` | 359 | Drives the host from the page (AI pacing) |
| `apps/web/src/practice/saveStore.ts` | 122 | IndexedDB save `{catalog, config, log, aiCursor, hash}` (R668). A resume re-folds the log and checks `hashState`, so **TS-engine saves will not fold in a Rust engine** (the catalog check refuses them if the version is bumped) |
| `apps/web/src/practice/emotes.ts` | 186 | **Main-thread** import of `@jackioh/ai` (`createEmotePersona`, `pickPersona`) and `@jackioh/ai/config` `AI_EMOTE` |
| `apps/web/src/practice/decks.ts` | 217 / `config.ts` 79 | Presets (`presetById`), `PRACTICE_AI_CLOCK_MS` |
| `apps/web/src/tutorial/harness.ts` | – | Test-only (never loaded by the page): runs lessons through the real `createPracticeCore` with `AI_GATE_BUDGET` and `createRng` |
| `apps/web/src/routes/practice.tsx` | 961 | Practice/tutorial route; `window.__jackiohPractice` |
| `apps/web/src/routes/match.tsx` | 442 | Online match route; `getCatalog()`; installs the networked dev handle |

Web tests that exercise the engine through these seams: `practice/core.test.ts` (1072), `controller.test.ts` (716), `host.test.ts` (268), `core-fallback.test.ts` (172), `core-glitch.test.ts` (138), `saveStore.test.ts` (122), `resume.test.ts` (61), `lastBoard.test.ts` (43).

### 7.2 Non-`shared` package imports in `apps/web/src` (non-test files)

| File | Specifier | Symbols |
| --- | --- | --- |
| `game/engine.real.ts` | `@jackioh/engine` | `* as engine` (createGame, beginGame, reduce, legalActions, viewFor, hashState, registeredCatalog) |
| `game/engine.real.ts` | `@jackioh/cards` | `registerAll` |
| `practice/core.ts` | `@jackioh/engine` | `beginGame, createGame, createRng, hashState, lastBoardFor, legalActions, reduce, registeredCatalog, seatPlayedBy, viewFor, type GameState, type Rng` |
| `practice/core.ts` | `@jackioh/engine/config` | `AI_DIFFICULTY, AI_TUTORIAL, DECK_SIZE, type Handicap` |
| `practice/core.ts` | `@jackioh/ai` | `AI_BUDGET, aiToAct, buildAiDeck, decide, type SearchBudget` |
| `practice/core.ts` | `@jackioh/cards` | `CATALOG_VERSION, registerAll` |
| `practice/emotes.ts` | `@jackioh/ai` | `createEmotePersona, pickPersona, type AiEmote, type EmotePersona` |
| `practice/emotes.ts` | `@jackioh/ai/config` | `AI_EMOTE` |
| `tutorial/harness.ts` (test support) | `@jackioh/ai` / `@jackioh/engine` | `AI_GATE_BUDGET, type SearchBudget` / `createRng` |
| `game/engine.ts`, `routes/dev/hotseat.tsx` | `@jackioh/engine/config` | `type Handicap` |
| `practice/PracticeSetup.tsx` | `@jackioh/engine/config` | `AI_DIFFICULTY, DIFFICULTIES, DRAWS_PER_TURN, HUMAN_HANDICAP, type Difficulty, type Handicap` |
| `practice/PracticeResult.tsx`, `practice/config.ts`, `practice/testids.ts`, `practice/protocol.ts` | `@jackioh/engine/config` | `type Difficulty` (protocol also `type Handicap`) |
| `practice/Tier.tsx`, `routes/practice.tsx` | `@jackioh/engine/config` | `DIFFICULTIES, type Difficulty` |
| `practice/resume.ts` | `@jackioh/engine/config` | `DIFFICULTIES` |
| `practice/decks.ts`, `routes/play.tsx` | `@jackioh/engine/config` | `DECK_SIZE` |
| `game/decks.ts` | `@jackioh/engine/config` | `DECK_SIZE, MAX_COPIES` |
| `game/deckbuilder/deckSize.ts` | `@jackioh/engine/config` | `export { DECK_SIZE, MAX_COPIES }` |
| `tutorial/TutorialResult.tsx` | `@jackioh/engine/config` | `DIFFICULTIES, TURN_CAP_PLAYER_TURNS` |
| `tutorial/scripts/basics.ts` | `@jackioh/engine/config` | `MAX_MANA` |
| `tutorial/scripts/traps.ts` | `@jackioh/engine/config` | `COIN_DEF_ID` |
| `audio/musicDirector.ts` | `@jackioh/engine/config` | `HERO_HEALTH` |
| `cards/glitch.ts` | `@jackioh/engine/config` | `GLITCH_DEF_ID` |
| `game/deckbuilder/sync.ts` | `@jackioh/validator` | `checkDeckDraft, checkImportRoom, checkTrioDraft, normalizeName` |
| `game/deckbuilder/deckCode.ts` | `@jackioh/validator` | `checkDeckDraft, normalizeName, type CatalogSnapshot, type Collection` |
| `game/deckbuilder/trioCode.ts` | `@jackioh/validator` | `checkTrioDraft, normalizeName, type CatalogSnapshot, type Collection` |
| `game/deckbuilder/TrioImportPanel.tsx` | `@jackioh/validator` | `checkImportRoom, trioConflicts, type CatalogSnapshot, type Collection` |
| `game/deckbuilder/workshop.ts` | `@jackioh/validator` | `trioConflicts, validateDeck, validateTrio, type CatalogSnapshot, type Collection, type LoadoutResult` |
| `routes/play.tsx` | `@jackioh/validator` | `validateDeck, validateTrio, type CatalogSnapshot, type Collection, type LoadoutDeck, type LoadoutResult` |
| `game/deckbuilder/fixtures.ts` | `@jackioh/validator` | `LOADOUT_DECKS, type CatalogSnapshot, type Collection` |
| `game/deckbuilder/testids.ts` | `@jackioh/validator` | `type LoadoutRule` |
| `game/deckbuilder/{CardBrowser,DeckSidebar,ManaCurve}.tsx`, `routes/almanac.tsx` | `@jackioh/validator` | `type CatalogSnapshot` |
| `game/deckbuilder/{DeckEditor,DeckWorkshop,ImportPanel,PoolGrid,TrioEditor}.tsx`, `filters.ts`, `loadout.ts`, `routes/decks.tsx` | `@jackioh/validator` | `type CatalogSnapshot, type Collection` |
| `routes/almanac.tsx`, `routes/landingFan.ts`, `routes/stats.tsx`, `stats/PlayerStatsCard.tsx`, `audio/voiceData.ts`, `emotes/portraits.ts` | `@jackioh/cards/catalog.json` | default `catalogJson` (bundled catalog) |
| `cards/flavour.ts` | `@jackioh/cards/flavour.json` + `@jackioh/cards` | `flavourJson`, `type CardFlavour` |
| `cards/inspect/Flavour.tsx` | `@jackioh/cards` | `type CardFlavour` |

### 7.3 `@jackioh/shared` imports in `apps/web/src` (151 import lines, non-test)

**Runtime (value) symbols** the client needs from shared, with file counts: `keywordKey` 5, `fillParams` 5, `opponentOf` 4, `hasKeyword` 3, `SHIPPED_SETS` 3, `readCodeInput` 2, `portraitOrDefault` 2, `isVoiceEmote` 2, `aimKey` 2, `VOICE_EMOTE_IDS` 2, `PORTRAIT_IDS` 2, `PORTRAITS` 2, `EMOJI_EMOTE_IDS` 2, `DEFAULT_PORTRAIT` 2, and 1 each: `pickPortrait`, `parseAim`, `normalizeCodeText`, `isPortraitId`, `isEmoteId`, `isCodeSeparator`, `formattedCaret`, `findCodeInText`, `excludedCharacters`, `emoteGate`, `PARAM_PLACEHOLDER`. Everything else is types.

Per file (types unless noted):

```
audio/cues.ts            CardType, GameEvent, GameEventType, PlayerId, PlayerView, PrintedRarity, Rarity, Tag, UnitView
audio/director.ts        GameEvent, PlayerId, PlayerView, UnitView
audio/musicDirector.ts   GameEvent, PlayerId, PlayerView
audio/types.ts           VoiceEmoteId
audio/useGameAudio.ts, audio/usePickupSound.ts   PlayerView
audio/voiceData.ts       PlayerView, PortraitId, VoiceEmoteId
auth/CodeField.tsx       findCodeInText, formattedCaret, isCodeSeparator, normalizeCodeText, readCodeInput (values); CodeFormat, CodeInputProblem
auth/codeInput.ts        excludedCharacters (value); CodeFormat, CodeInputProblem, CodeInputReading
cards/CardFace.tsx       keywordKey (value); CardType, Rarity
cards/CardMarks.tsx      CardMark
cards/CardRef.tsx, cards/glitch.ts, cards/inspect/CardDetail.tsx, cards/inspect/References.tsx, cards/refs.ts   CardDef
cards/KeywordFx.tsx, cards/glossary.ts   KeywordKind
cards/MinionFace.tsx     hasKeyword, keywordKey (values); Keyword, UnitView
cards/RulesText.tsx      CardDef, PreviewValue
cards/art/CardArt.tsx, cards/art/themes.ts   CardType, Tag
cards/cardState.ts       CardType, Enchantment, QuestView
cards/inPlay.ts          Tag
cards/keywordVisuals.ts  Keyword, KeywordKind, UnitView
cards/marks.ts           CardMark, CardView, GameEvent
cards/model.ts           fillParams, keywordKey (values); CardDef, CardMark, CardFace, CardType, Enchantment, Keyword, Param, QuestView, Rarity, PreviewValue, PrintedRarity, SetName, Tag, Tuning
cards/refContext.tsx     CardDef, CardDefs
cards/tuning.ts          PARAM_PLACEHOLDER, keywordKey (values); CardDef, Keyword, KeywordKind, Param, Tuning
emotes/EmojiArt.tsx      EmojiEmoteId
emotes/Portrait.tsx      PortraitId
emotes/PortraitPicker.tsx  EMOJI_EMOTE_IDS, PORTRAIT_IDS, PORTRAITS, VOICE_EMOTE_IDS (values); EmoteId, PortraitId
emotes/play.ts           isVoiceEmote (value); EmojiEmoteId, EmoteId, PortraitId, VoiceEmoteId
emotes/portraits.ts      PORTRAIT_IDS, PORTRAITS (values); CardDef, PortraitId
emotes/session.ts        emoteGate (value); EmoteGate, EmoteId, PlayerId
emotes/ui.tsx            EMOJI_EMOTE_IDS, VOICE_EMOTE_IDS, isVoiceEmote (values); EmojiEmoteId, EmoteId, VoiceEmoteId, EmoteGate
emotes/useEmotes.ts      DEFAULT_PORTRAIT (value); EmoteGate, EmoteId, PlayerId, PortraitId
fx/FxLayer.tsx           PlayerId, PlayerView
fx/brand.ts, fx/castOnDraw.ts, fx/entrances.ts   GameEvent
fx/cardFx.ts, fx/stage.ts   GameEvent, PlayerView
fx/chaos.ts, fx/memory.ts   GameEvent, PlayerId
fx/cues.ts               hasKeyword (value); GameEvent, PlayerView
fx/manaMarks.ts          PlayerId, PlayerView, SideView
fx/types.ts              CardType, GameEvent, PlayerId, PlayerView, Rarity, Row, Tag
game/ActivateControl.tsx fillParams (value); ActivationView, CardView
game/Backrow.tsx         BackrowView
game/Board.tsx           CardView, GameEvent, GameEventType, LibraryView, PlayerId, PlayerView, Row
game/Card.tsx            CardMark, CardType, CardView, PlayerId, UnitView
game/DrawOffer.tsx       ActionBody, GameEvent, PlayerView
game/Game.tsx            ActionBody, Aim, EmoteGate, EmoteId, GameEvent, PlayerId, PlayerView, PortraitId
game/Hand.tsx, game/glow.ts   CardView
game/Hero.tsx            DEFAULT_PORTRAIT (value); HeroPowerView, PlayerView
game/Log.tsx             GameEvent, LibraryOverflowOutcome, PlayerId, PlayerView, PromptKind
game/OverflowNotices.tsx GameEvent, GameEventType, LibraryOverflowOutcome, PlayerView
game/Prompt.tsx          ActionBody, CardView, MulliganView, PendingView, PlayerId, PlayerView, PromptKind, Selection
game/Result.tsx          GameOverReason, PlayerId, PlayerView
game/TheirHand.tsx, game/reveal.ts   CardView, PlayerView
game/Zone.tsx            PlayerView, Row
game/actions.ts          Action, ActionBody, ActivationView, PendingOption, PendingView, PlayerId, PlayerView, Row, Selection, ZoneChoice
game/aim/OpponentAim.tsx aimKey (value); Aim, PlayerView
game/aim/aim.ts          ActionBody, Aim, AimEnd, PlayerView, Row
game/aim/useAimEmitter.ts aimKey (value); ActionBody, Aim, PlayerView
game/animations.ts       GameEvent, GameEventType, PlayerId, PlayerView, Zone
game/catalog.ts          fillParams (value); CardDef, CardDefs, CardType, CardView, PlayerView, Rarity, Tag
game/contract.ts         EmoteGate, EmoteId, GameEvent, GameEventType, PlayerId, PlayerView, PortraitId, Row
game/deckbuilder/CardBrowser.tsx   Tag
game/deckbuilder/DeckEditor.tsx    portraitOrDefault (value)
game/deckbuilder/DeckSidebar.tsx   CardCost, CardDef
game/deckbuilder/FilterBar.tsx, savedBrowse.ts   CardType, Rarity, SetName, Tag
game/deckbuilder/PoolGrid.tsx, deckCode.ts   CardDef
game/deckbuilder/TrioEditor.tsx    CardCost
game/deckbuilder/filters.ts        SHIPPED_SETS, fillParams (values); CardCost, CardDef, CardType, KeywordKind, Rarity, SetName, Tag
game/deckbuilder/fixtures.ts       CardDef, CardDefs
game/deckbuilder/testids.ts        CardType, Rarity, Tag
game/decks.ts            CardCost, CardDef, CardDefs
game/drag/DragLayer.tsx  ActionBody, CardView, PlayerView
game/drag/model.ts       ActionBody, PlayerView
game/drag/targets.ts     Row
game/engine.ts           Action, ActionBody, CardDefs, GameEvent, PlayerId, PlayerView
game/faces.ts            CardDef, CardType, CardView, PlayerView, UnitView
game/hotseat.ts          opponentOf (value); Action, ActionBody, CardDefs, GameEvent, PlayerId, PlayerView
game/net.ts              isEmoteId, isPortraitId, parseAim (values); ActionBody, Aim, EmoteId, PlayerId, PlayerView, PortraitId, PromptKind
game/promptOver.ts, game/stats/useGameStats.ts, practice/ModifierList.tsx, tutorial/targets.ts   PlayerView
game/runs.ts, game/showcase/CardShowcase.tsx, game/showcase/plan.ts, game/useLogHistory.ts   GameEvent, PlayerId, PlayerView
game/slamResolver.ts     PlayerView, UnitView
game/spent.ts            PlayerId, PlayerView, UnitView
game/unitSlam.ts         hasKeyword (value); UnitView
haptics/haptics.ts       GameEvent, PlayerView
net/api.ts               CardDefs, GameOverReason
patches/PatchFace.tsx, patches/fixtures.ts   CardDef
patches/PatchNotes.tsx, patches/source.ts, routes/almanac.tsx   CardDef, CardDefs
patches/diff.ts          fillParams, keywordKey (values); CardCost, CardDef, CardDefs, CardFace, Param
patches/history.ts       SHIPPED_SETS (value); CardDef, SetName
practice/DeckPreview.tsx CardCost, CardDef, CardDefs
practice/PracticeResult.tsx  GameOverReason, PlayerId, PlayerView
practice/PracticeSetup.tsx, routes/stats.tsx, stats/PlayerStatsCard.tsx   CardDefs
practice/controller.ts   opponentOf (value); ActionBody, CardDefs, PlayerId, PlayerView
practice/core.ts         opponentOf (value); Action, ActionBody, PlayerId
practice/emotes.ts       opponentOf, pickPortrait, portraitOrDefault (values); EmoteId, PlayerId, PlayerView, PortraitId
practice/protocol.ts     Action, ActionBody, CardDefs, DistributiveOmit, PlayerId, PlayerView
practice/saveStore.ts    Action
routes/dev/hotseat.tsx   Action, ActionBody, CardDefs, PlayerId, PlayerView
routes/invite.tsx        readCodeInput (value)
routes/landing.tsx       CardDef
routes/landingFan.ts     SHIPPED_SETS (value); CardDef, CardDefs, Rarity
routes/match.tsx, routes/practice.tsx   ActionBody, CardDefs, PlayerId, PlayerView
stats/model.ts, tutorial/lessons.ts   PlayerId
stats/track.ts           CardView, GameEvent, PlayerView, SideView
tutorial/TutorialHud.tsx, tutorial/TutorialResult.tsx   PlayerId, PlayerView
tutorial/advice.ts, tutorial/scripts/basics.ts (PlayerView, UnitView)   ActionBody, PlayerView, UnitView
tutorial/coach.ts        ActionBody, GameEvent, PlayerView, Row
tutorial/devHandle.ts, tutorial/tracker.ts   ActionBody
tutorial/harness.ts      Action, ActionBody, PlayerId, PlayerView
tutorial/scripts/advanced.ts   ActionBody, PlayerView
tutorial/scripts/spells.ts     ActionBody, PlayerView, Selection, UnitView
tutorial/scripts/traps.ts      ActionBody, CardView, PlayerView
tutorial/steps.ts        ActionBody, CardView, GameEvent, PlayerView, UnitView
```

Shared sources (`packages/shared/src`, 2,477 lines): `view.ts` 439 (PlayerView), `events.ts` 498 (GameEvent), `actions.ts` 97, `catalog-types.ts` 406, `codes.ts` 358, `stats.ts` 442, `emotes.ts` 139, `aim.ts` 90. **The Rust engine's serde output must match `view.ts`, `events.ts` and `actions.ts` exactly**, because the whole React layer is typed against them.

### 7.4 Imports of `apps/server/src/config.ts` from web and e2e (coupling to break first)

- **Web (non-test)**: `auth/codeInput.ts`, `auth/validation.ts`, `game/Clock.tsx`, `game/deckbuilder/{ImportPanel.tsx, TrioImportPanel.tsx, deckCode.ts, sync.ts, testkit.ts, trioCode.ts}`, `main.tsx`, `net/{api.ts, auth.ts, gate.ts, session.ts}`, `routes/{Rematch.tsx, SeriesBanner.tsx, account.tsx, invite.tsx, landing.tsx, login.tsx, play.tsx, privacy.tsx, series.tsx}`. Also `game/clockConstants.ts` and many tests. Symbols: `MAX_SAVED_TRIOS`, `MAX_SAVED_DECKS`, `DECK_NAME_MAX_LENGTH`, `SERIES_POLL_SECONDS`, `SERIES_WINS_NEEDED`, `SERIES_MAX_GAMES`, `GATE_SLOW_NOTICE_SECONDS`, `DECK_AUTOSAVE_DEBOUNCE_MS`, `AUTH_PASSWORD_MIN/MAX_LENGTH`, `TURN_CLOCK_MS`, `MULLIGAN_CLOCK_MS`, `AUTH_SESSION_REFRESH_MARGIN_SECONDS`, `TRIO_CODE_VERSION`, `SERIES_PICK_SECONDS`, `ROOM_CODE_LENGTH`, `MATCH_FOUND_NAV_DELAY_MS`, `MATCH_CEILING_MS`, `MATCH_ACTION_RETENTION_DAYS`, `INVITE_CODE_FORMAT`, `DECK_CODE_VERSION`, `CODE_ATTEMPT_RETENTION_DAYS`, `CODE_ALPHABET`, `AUTH_SIGN_OUT_WAIT_SECONDS`, `AUTH_PENDING_ADDRESS_TTL_SECONDS`, `AUTH_EMAIL_RESEND_COOLDOWN_SECONDS`, `API_REQUEST_TIMEOUT_SECONDS`, `DECK_AUTOSAVE_RETRY_SECONDS`, `CATALOG_NUMBER_SET_OFFSETS`, `DECK_CODE_*`/`TRIO_CODE_*`, `PLAYER_SETTINGS_*`, `TUTORIAL_*` (some multi-line imports were not captured by the one-line grep, so grep the files when planning).
- **e2e**: `cypress/component/deckbuilder-layout.cy.tsx`, `cypress/component/landing-and-code-field.cy.tsx`, `cypress/e2e/{05-reconnect, 06-room-code, 10-invite-gate, 14-landing-and-sign-in, 18-deck-workshop, 19-queue-modes-and-series, 20-mulligan-concede-draw, 26-trio-codes}.cy.ts`.
- **packages**: `packages/shared/test/fixtures/code-input-cases.ts`.

### 7.5 e2e imports of packages and web source

- `e2e/cypress/component/*.cy.tsx` import web source (`apps/web/src/game/Game.tsx`, `test/fixtures.ts`, `cards/*`, `audio/*`, `settings/index.ts`, `routes/*`, `net/api.ts` types, `net/session.ts`, `auth/testids.ts`), plus `packages/cards/src/catalog-data.ts` (`CATALOG`, `CATALOG_VERSION`) and `packages/shared/src/catalog-types.ts` (`KEYWORD_KINDS`, types), `packages/shared/src/view.ts` (`type UnitView`).
- `e2e/support/tasks/replay-runner.ts` (79): dynamic `import("../../../packages/cards/src/index.ts")` (`CATALOG`, `registerAll`) and `import("../../../packages/engine/src/replay.ts")` (`fold`, `hashState`). It folds the browser's `(seed, decks, log, handicaps?, dealt?)` and **hashes the browser's raw `state` JSON with the TS `hashState`**. Spec 01's replay check needs a Rust equivalent (a CLI, or WASM under Node), and the browser must provide the state in a form Rust can hash, or provide its hash directly.
- `e2e/support/tasks/lessons-runner.ts` (61): dynamic import of `apps/web/src/tutorial/lessons.ts` and `packages/engine/src/config.ts`.
- `e2e/support/tasks/wsPlayer.ts` (389): Node `WebSocket` with `?token=&matchId=`, sends `hello` and `{type:"action", action}`.
- `e2e/support/config.ts`: fixture accounts and tokens (`e2e-token-p1`/`p2`/`pending`), mirroring `apps/server/src/api/e2e.ts`.

### 7.6 `window.__jackioh*` dev handles (outside `MODE === "production"`; `pnpm build:e2e` = `vite build --mode development`)

| Handle | Set in | Members | Members e2e reads |
| --- | --- | --- | --- |
| `window.__jackioh` (hotseat) | `routes/dev/hotseat.tsx:385` | `state` (**raw engine `GameState`**), `seed`, `seat`, `log` (Action[]), `decks`, `handicaps`, `dispatch(action & {playerId?}) → {events, error?}`, `view()`, `legal()`, `hash()`, `setSeat(seat)` | `state` (`.result`, `.turn`, `.active`, `.phase`, `.pending{id/choiceId,kind,playerId}`, `.mulligan[p]{prompt{id,kind},keep}`, `.players[p].hand[].id`, `.players[p].units[lane][0].id`), `seat`, `seed`, `log`, `decks`, `handicaps`, `dispatch` |
| `window.__jackioh` (networked) | `game/net.ts:770` `installDevHandle` | `state: ViewDerivedState|null` (from `PlayerView`: seed `""`, turn, active, phase, result, `pending{choiceId,kind,player}`, `players{[viewer]: view.you, [opp]: view.opponent}`), `seed`, `seat`, `dispatch` | same reads |
| `window.__jackiohE2E` | written by `cy.seedGame` (`e2e/support/commands.ts:463`) | `{decks: Record<id,string[]>, seed, handicaps?}`, mirrored to localStorage `jackioh.e2e.decks` | – |
| `window.__jackiohPractice` | `routes/practice.tsx:576` | `snapshot() → Promise<PracticeDebug>` (raw state JSON + hash), `aiSeat`, `thinking`, `view` | `snapshot()`, `view` |
| `window.__jackiohTutorial` | `tutorial/devHandle.ts:44` | `lessonId`, `display`, `suggested: ActionBody|null` | `suggested` |
| `window.__jackiohAudio` | `audio/debug.ts:39` | audio debug (`contextsCreated()`, …) | `contextsCreated()` |

**What this means for the WASM bindings.** A WASM `GameState` handle must offer `toJSON()` (or a getter) returning the same field names e2e reads (`players[p].hand`, `players[p].units`, `pending.playerId`, `mulligan`, `result`, `turn`, `active`, `phase`, `seed`), and `hashState` must be callable on it in the browser.

### 7.7 How the catalog reaches the client

- **Fetched** `GET /api/catalog` → `{version, defs}` (`net/api.ts:510` `getCatalog`) in `routes/decks.tsx:129`, `routes/play.tsx:211` and `routes/match.tsx:119`.
- **Bundled** `@jackioh/cards/catalog.json` in `routes/almanac.tsx`, `routes/landingFan.ts`, `routes/stats.tsx`, `stats/PlayerStatsCard.tsx`, `audio/voiceData.ts`, `emotes/portraits.ts`. `@jackioh/cards/flavour.json` in `cards/flavour.ts`.
- **From the engine**: hotseat `EnginePort.catalog()` = `registeredCatalog()`; practice worker `started.defs` / `catalog.defs` = `registeredCatalog()`.
- **Patch snapshots** via Vite `import.meta.glob` over `packages/cards/patches/*.json` (`patches/source.ts`), not via `/api/catalog/:version`.
- **`CatalogContext`** (`game/catalog.ts:51`) is a React context of `CardLookup = (defId, radiant) → CardInfo`, built with `lookupFromDefs(defs)`. Providers: `routes/dev/hotseat.tsx:434`, `routes/practice.tsx:909`, `routes/match.tsx:439`. `Game.tsx:300` and the board components consume it, and `withMatchDefs(lookup, view.defs)` adds the match-made defs from the view.
- `VITE_CATALOG_VERSION` exists in the env contract; the decks save uses the server's `catalogVersion` from `/api/decks` / `/api/collection`.

### 7.8 Vite and workers

`apps/web/vite.config.ts`: `plugins: [react()]`, `server.port 5173`, **`worker: { format: "es" }`** (module worker), with nothing else special. The worker is built from `new Worker(new URL("./practice.worker.ts", import.meta.url), {type:"module"})`. The engine is code-split by the dynamic `import("./engine.real.ts")`. `build:e2e` = `vite build --mode development` (keeps the dev handles). The architecture doc says the build emits to a repo-root `dist/` for Vercel. Adding WASM will need either `vite-plugin-wasm` (+ top-level-await) or wasm-bindgen `--target web` with `?url` / `new URL("…_bg.wasm", import.meta.url)` loading, in both the worker bundle and the main bundle. `apps/web/package.json` dependencies: `@jackioh/{ai,cards,engine,shared,validator}`, `json5`, `react`/`react-dom` 19.3.0; dev deps `vite` 8.3.0, `@vitejs/plugin-react`, `jsdom`, testing-library.

### 7.9 Minimal engine, AI and validator API the WASM build must expose (derived from 7.1–7.6)

- **Hotseat**: `createGame({seed, decks, handicaps?})`, `beginGame`, `reduce(state, Action) → {state, events, error?}`, `legalActions(state, seat)`, `viewFor(state, seat)`, `hashState(state)`, `registeredCatalog() → CardDefs`, and state JSON for the dev handle.
- **Practice worker**: all of the above, plus `createGame({…, lastBoards?, dealt?})`, `lastBoardFor(state, seat)`, `seatPlayedBy(state, homeSeat)`, `state.result`, `createRng(seed, cursor?)` + `rng.cursor`, `buildAiDeck(rng, size, {banned, manaCap?})`, `aiToAct(state, seat)`, `decide(state, seat, {rng, budget, shouldStop}) → {action}|null` (`shouldStop` is a JS wall-clock callback; for WASM, pass a deadline in ms or call `performance.now()` via js-sys), constants `AI_BUDGET`, `AI_GATE_BUDGET`, `AI_DIFFICULTY`, `AI_TUTORIAL`, `DECK_SIZE`, `CATALOG_VERSION`. Simplest design: move all of `createPracticeCore`'s `handle(request) → response` into Rust, and let the worker only forward JSON. Lesson decks (`tutorial/lessons.ts` `lessonById`) and presets (`practice/decks.ts` `presetById`) are TS data and must then be passed in `PracticeStartConfig` or moved into Rust.
- **Main thread (no game)**: engine config constants (`DECK_SIZE`, `MAX_COPIES`, `HERO_HEALTH`, `GLITCH_DEF_ID`, `AI_DIFFICULTY`, `DIFFICULTIES`, `DRAWS_PER_TURN`, `HUMAN_HANDICAP`, `TURN_CAP_PLAYER_TURNS`, `MAX_MANA`, `COIN_DEF_ID`); emote personas (`createEmotePersona`, `pickPersona`, `AI_EMOTE`); validator (`checkDeckDraft`, `checkTrioDraft`, `checkImportRoom`, `normalizeName`, `trioConflicts`, `validateDeck`, `validateTrio`, `LOADOUT_DECKS`). SPEC §9.4 requires "one validator module shared by client and server", so the Rust validator compiled to WASM should replace `@jackioh/validator` on the client. The shared helpers the Rust server must reproduce bit-for-bit are `canonicalCode`/`normalizeCodeText` (invite and room codes), `emoteGate` (client and server run the same gate), `parseAim`/`aimKey`, `isEmoteId`/`isPortraitId`, `pickPortraitFromSeed`, `portraitOrDefault`, and `cardStats`/`parseGameRecord`/`parseGameRecordLines`/`recordMatches`/`sourcesOf`/`DEV_RECORD_ID_PREFIX` (the full server-side list from `@jackioh/shared` is in §10's final note).

---

## 8. Client networking (`apps/web/src/net/*`, `game/net.ts`)

| File | Lines | What |
| --- | --- | --- |
| `net/api.ts` | 870 | REST client: `apiBaseUrl()` (`VITE_SERVER_HTTP_URL`, default `http://localhost:8787`), `matchSocketUrl()` (`VITE_SERVER_WS_URL`, default `ws://localhost:8787/ws/match`), `apiRequest<T>(path, {method, token, body, signal})` (Bearer, JSON, 75 s timeout, `ApiRequestError{status, code, details}`, `ApiUnreachableError`), one typed function per route (§2) |
| `net/auth.ts` | 1264 | Supabase GoTrue over plain `fetch` (**no `@supabase/supabase-js`**): `/auth/v1/token?grant_type=password|pkce|refresh_token`, `/signup`, `/resend`, `/recover`, `/user` (GET/PUT), `/logout?scope=local`, `/otp`, `/verify`, `/factors` (+ challenge/verify), OAuth authorize URL; `VITE_SUPABASE_URL` + `VITE_SUPABASE_PUBLISHABLE_KEY` |
| `net/session.ts` | 283 | localStorage session `jackioh.session` (and `jackioh.e2e.session` for E2E), pending email/reset keys |
| `net/gate.ts` | 472 | `useAccount()` over `GET /api/auth/me`, token renewal after 401 (`callWithRenewal`), rate-limit messaging |
| `net/navigate.ts` | 155 | SPA paths: `/`, `/login`, `/reset-password`, `/invite`, `/decks`, `/play`, `/account`, `/practice`, `/privacy`, `/terms`, `/accessibility`, `/patch-notes`, `/almanac`, `/leaderboard`, `/stats`, `/dev/hotseat`, `/match/:id`, `/series/:id` |
| `net/return-to.ts` | 45 | Post-login return path |
| `game/net.ts` | 844 | Match WebSocket client `createMatchClient`: URL `socketUrlFor(base, token, matchId)` → `?token=&matchId=` (line 446); sends `hello {token, matchId}` on open; `send(body)` → `{type:"action", action:{...body, nonce}}`, nonce `c<base36 time>-<n>-<random>`; emote and aim frames; parses `view` (+`legal`), **`legal` (a standalone frame the server never sends)**, `ack`, `error`, `prompt`, `clock`, `portraits`, `emote`, `aim`; reconnect backoff `[250,500,1000,2000,5000]`; final on 4401/4403/4404; closed on 4410; `setToken` for the next socket; `remainingMs` from `clock.now` + a monotonic delta; networked dev handle |
| Deploy-config tests in `net/` | – | `cloudflare-config.test.ts`, `cloudflare-serve.test.ts`, `deploy-routes.test.ts`, `vercel-ignore.test.ts`, `ci-scope.test.ts`, `promote-production.test.ts`, `env-production.test.ts` (repo plumbing, not networking) |

Supabase Auth is used **only by the browser directly** (REST, publishable key). The server uses `@supabase/supabase-js` only for the admin `getUserById` / `deleteUser` (and an unused password client), plus a raw `fetch` to `/auth/v1/user`.

---

## 9. Deployment: what a Rust server on Render must preserve

From `render.yaml` and `docs/architecture.md` §8–§10:

- **Service**: `type: web`, `name: jackioh-server` (deploy-watch.yml looks the service up by this name), `plan: free` (sleeps after about 15 min idle and takes about 50 s to wake; matches survive via fold), `region: ohio` (same region as Supabase), `branch: main`. Currently `runtime: node`, with `buildCommand: pnpm install --frozen-lockfile --prod=false`. A Rust service would use `runtime: rust` (`cargo build --release`) or `runtime: docker`. **There is no Dockerfile in the repo today.**
- **Port**: listen on `$PORT` (Render injects it; local default 8787). HTTP and WS on the same port. WS path `/ws/match`.
- **Health check**: `healthCheckPath: /api/catalog`. It must return 200 only once the catalog is loaded and after `release` (migrate + seed) has succeeded. It must carry `x-deployed-commit` from `RENDER_GIT_COMMIT` (read by `.github/workflows/deploy-watch.yml`, which compares it with the pushed SHA and the body's `version` with render.yaml's `CATALOG_VERSION` value).
- **Start command semantics**: (1) `CATALOG_VERSION` = the last `.version` in `packages/cards/patches/patches.json` (today via `node scripts/catalog-version.mjs`; **a Rust runtime image may have no Node**, so do this inside the binary or the shell). (2) `db:migrate` (append-only, checksum-compatible ledger, `pg_advisory_xact_lock(0x6a61636b)` per file, as a non-superuser role). (3) `db:seed-catalog` at that version (one tx, upsert `public.cards`, write `app.settings.catalog_version`). (4) Serve. Any failure must exit non-zero so Render keeps the previous deploy.
- **Env vars**: `NODE_ENV=production` (E2E must be refused with it), `CATALOG_VERSION` (render.yaml literal `v0.2.11`, which CI tests hold equal to patches.json and the start command overrides), `TRUSTED_PROXY_HOPS=1`, `CYPRESS_INSTALL_BINARY=0`, `NODE_VERSION=24.19.0` (Node-only), and the dashboard secrets `SUPABASE_URL`, `SUPABASE_SECRET_KEY`, `DATABASE_URL`, `CODE_PEPPER`, `PUBLIC_ORIGINS`. Optional: `SUPABASE_JWKS_URL`, `SUPABASE_JWT_SECRET`. Render-set: `PORT`, `RENDER_GIT_COMMIT`.
- **Files read at runtime**: `packages/cards/catalog.json`, `packages/cards/patches/patches.json`, `packages/cards/patches/<version>.json`, `apps/server/src/db/migrations/*.sql`. Embed them (`include_str!`/`include_dir!`) or ship them with the binary.
- **Memory**: the rehearsal enforces a budget under the 512 MB free instance.
- **Rehearsal**: `apps/server/test/deploy/rehearse.sh` executes render.yaml's `startCommand` literally, so it must change with it.
- **Bring-up checklist** (`docs/architecture.md` §10, steps 1–16): Supabase project; auth settings (Site URL, redirect URLs, PKCE, magic link template, TOTP, OAuth); fill env; `db:migrate` (expect 25 public tables, all RLS); verify invariants (`test:sql`); `db:seed-catalog` (expect 318 cards); `codes:mint`; start; calibrate `TRUSTED_PROXY_HOPS` from `api.forwarded_for {fewestEntries}` log lines; sign up two accounts and check 403 while pending; redeem codes; build decks; create a room (`POST /api/rooms {mode:"bo1",deckId}`) and join; play; kill the server and reconnect (fold); finish and check one `results` row.
- **Logs** relied upon operationally: JSON lines `{level, event, ...}`, especially `api.forwarded_for`, `server.listening`, `server.e2e_mode` (alert), `match.fold.errors`, `game.record.failed`, `match.voided`, `reaper.resolved`, `codes.breaker_open`, `season.open_failed`.
- **Client side**: `apps/web/.env.production` points at `https://jackioh-server.onrender.com` and `wss://jackioh-server.onrender.com/ws/match`. Cloudflare (production) and Vercel (staging) must be in `PUBLIC_ORIGINS`.

---

## 10. Exceptions and special cases worth deleting (or deciding on) in a rewrite

| # | Where | What | Note |
| --- | --- | --- | --- |
| 1 | `apps/server/src/index.ts:62` (`STORE_EXPORT_CANDIDATES`), `loadStore`, `StoreUnavailableError`; `src/db/store.ts:1107-1108` (`createStore`, `postgresStore` aliases) | Dynamic import of `./db/store` with three accepted export names, from before the store existed | Delete |
| 2 | `src/match/engine.ts:150` `REQUIRED_ENGINE_EXPORTS`, `EngineUnavailableError`, `loadEnginePort` (string-specifier dynamic import), `setEnginePort` | Lets the server boot without the engine; test injection | Replace with a trait plus a fake impl in tests |
| 3 | `apps/web/src/game/engine.ts:53,100` + `engine.real.ts` | Same pattern client-side (`REQUIRED_ENGINE_EXPORTS`, lazy import) | The WASM loader replaces it |
| 4 | `src/match/protocol.ts:62` `JoinRoomMessage`, `actor.ts:636` | A client frame accepted only to answer `unsupported` | Delete |
| 5 | `protocol.ts:50` `HelloMessage.token/matchId/roomCode` | Parsed and ignored. The web still sends `{type:"hello", token, matchId}` (`net.ts:569`) | Keep `hello` but ignore its fields, or change the client too |
| 6 | `protocol.ts:504` `raw.nonce ?? parsed.nonce` | Two spellings of the nonce position | Pick one (the client puts it inside `action`) |
| 7 | `wsServer.ts:124-135` `tokenFrom` | Three token paths. The comment calls `?token=` legacy, but **the browser (`net.ts:446`) and the e2e `wsPlayer` use it** | Either keep `?token=` or move the client to the subprotocol in the same change |
| 8 | `apps/web/src/game/net.ts:222,260,514` | Client accepts a `{type:"legal"}` frame the server never sends | Delete |
| 9 | `api/decks.ts:499-545` `readModeChoice`; `queue.ts:503`; `rooms.ts:202` | Legacy R257 body: no `mode` means bo1, and `deckIndex` instead of `deckId`. Used by `e2e/cypress/e2e/99-online-smoke.cy.ts:163,179,264,279` | Delete if the smoke spec is updated |
| 10 | `api/auth.ts:766,785` `/api/auth/signup`, `/api/auth/signin` | Always 503 in production (no publishable key is passed, `index.ts`); signin is used only by E2E fixture auth | Delete, or keep E2E-only |
| 11 | `api/catalog.ts:268` `GET /api/catalog/:version` | Unused by the web (it uses `import.meta.glob`) and by e2e | Candidate to drop |
| 12 | `api/catalog.ts:109` `versionOf` (`c1-<sha>`) fallback; `catalog.ts:24` error text "100 cards plus 9 tokens" | Dead fallback (env requires `CATALOG_VERSION`) and a stale message | Delete |
| 13 | E2E mode: `src/api/e2e.ts` (301), `src/api/e2e-store.ts` (767), `index.ts:126` `E2E_ENV_DEFAULTS` (hard-coded `CATALOG_VERSION: "v0.2.11"`, which the patch tool keeps in step), `index.ts:158` `browserOrigins`, `queue.ts:63` `seedOverrideOf` + module `e2eSeedByTicket` (`queue.ts:92`), `rooms.ts:62` `e2eSeedByRoom` | The whole test-server mode runs on an in-memory store | Decide: a Rust in-memory `Store` impl, or run e2e against a Docker Postgres. The fixed tokens and codes are a contract with `e2e/support/config.ts` |
| 14 | `src/api/memory-stores.ts` (866) | The in-memory store shared by E2E and the unit fakes | Rewritten or dropped together with #13 |
| 15 | `api/rematch.ts` module `offersByMatch` | Rematch offers are in-process memory (lost on restart) | Behaviour to preserve or move to the DB |
| 16 | `registry.ts` (no removal on finish) | Finished actors stay in memory until restart or the reaper; rematch presence depends on it | Make this explicit in the Rust design |
| 17 | `api/deps.ts:57` `roomCodeTtlMs: 15 * 60 * 1000` | Unnamed constant | Add `ROOM_CODE_TTL_SECONDS` |
| 18 | `ports.ts:254` `ProfileStore.setRating` | No caller in `src/api` (rating writes go through `setGlicko`) | Delete |
| 19 | `src/db/migrate.ts:47` `REWRITTEN` (0013) | Legacy checksum accepted | **Must keep** for existing databases |
| 20 | `actor.ts:169` portraits default `vanilla` "for a match that predates them" | Back-compat for old rows | Keep (old live rows) or drop after a migration |
| 21 | `registry.ts:52` `dealtFor(modeOf)` and `matches.mode` (0023, rematch only) | The mode is derived from tickets, room or series for older rows | Keep (fold determinism for live rows) |
| 22 | Migration 0003 `loadouts*` tables and `app.save_loadout` | Retired by R254, kept "so nothing saved is lost" | Keep the schema; no Rust code needed |
| 23 | `src/db/store.ts:2875` KNOWN DIVERGENCES | Fake vs Postgres differences | Disappears if E2E uses Postgres |
| 24 | `apps/server/src/config.ts` client-only constants | `AUTH_*` (client half), `GATE_SLOW_NOTICE_SECONDS`, `DECK_AUTOSAVE_*`, `MATCH_FOUND_NAV_DELAY_MS`, `SERIES_POLL_SECONDS`, `CODE_STATUS_RECHECK_FLOOR_SECONDS`, `API_REQUEST_TIMEOUT_SECONDS` live in the server package and are imported by web/e2e by relative path (§7.4) | Move to `packages/shared` (or generate a TS file from Rust) **before** removing the TS server |
| 25 | `apps/server/README.md` "Stubbed or pending" | Says `engine.real.ts` is excluded from tsconfig and the Store is unimplemented, but both are done | Stale doc |
| 26 | `docs/architecture.md` §2 ("JWT in the hello frame"), §5.1, the §13 file map (`auth/jwt.ts`, which does not exist) | Stale doc | Rewrite with the Rust plan |
| 27 | `apps/web/src/practice/emotes.ts` | Main-thread `@jackioh/ai` import (contradicts "core.ts alone imports @jackioh/ai") | Decide: WASM export or keep a small TS persona module |
| 28 | `api/stats.ts` `(r.mode as string) !== "tutorial"` filters | Filters a mode value that `GameRecord.mode` does not declare | Review when porting |
| 29 | Router: wrong method → 404 not 405 (`http.ts`) | Quirk | Preserve or change consciously (tests assert it) |
| 30 | `wsServer.ts` 1009 on oversized frames comes from `ws` `maxPayload`, while `parseClientMessage` also checks length | Double guard | One check in Rust |

**Note on `@jackioh/shared` used by the server** (all must be reimplemented in Rust with matching output): `canonicalCode`, `normalizeCodeText` (crypto.ts), `isPortraitId` (decks.ts), `pickPortraitFromSeed` (queue.ts, rooms.ts, rematch.ts), `portraitOrDefault` and `PLAYER_IDS` (registry.ts, actor.ts, store.ts), `aimKey`, `emoteGate`, `isEmoteId`, `parseAim` (actor.ts, protocol.ts), `cardStats`, `winRate` (stats.ts), `cardStats`, `formatCardStats`, `DEFAULT_CARD_STATS_FILTER`, `GAME_MODES`, `PILOT_FILTERS`, `SOURCE_FILTERS` (card-stats CLI), `DEV_RECORD_ID_PREFIX`, `parseGameRecordLines`, `recordMatches`, `parseGameRecord`, `sourcesOf` (records). From `@jackioh/validator`: `checkDeckDraft`, `checkImportRoom`, `checkTrioDraft`, `normalizeName`, `TRIO_DECKS`, `validateDeck`, `validateLoadout`. From `@jackioh/ai`: `buildAiDeck` (All Random deals, which must stay seed-identical for fold).
