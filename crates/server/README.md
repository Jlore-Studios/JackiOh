# `jackioh-server`

The authority for everything that is not presentation: identity, usernames (R1432–R1436) and the
invite gate, the collection ledger, saved decks and trios, custom cards (R915008), an active account's tutorial progress
(R320) and settings (R633), each profile's last board (R417), matchmaking in three modes, the
Conquest series, the ranked ladder and the ladder bots it plays itself when the queue is empty
(R1453, R1454, R1451, R1452), and the match itself: one actor per match holding the
`GameState` in memory, one WebSocket per player, `reduce` on every action and `view_for` pushed to
each player after every change (spec §9.1–§9.5, §10.8). Every match that ends leaves a game record
for the card statistics (§9.11, R376–R378). Play together lives here too: friends, blocks and
presence (§9.4), friend challenges, watching a friend's game, shared replays and chat (§9.5), and
Duos co-op, two players against an AI team this server plays, one co-op actor per match (§13).

The client sends intent and renders `view_for`; it never enforces a rule and never sees hidden
information (CLAUDE.md rule 7). Deployment, the schema, RLS and the env contract are in
[`docs/architecture.md`](../../docs/architecture.md).

## How it fits together

```
browser ──HTTPS──▶ Supabase Auth           (sign up, sign in, email confirmation)
browser ──HTTPS──▶ jackioh-server  ──▶ Postgres
browser ──WSS────▶ match actor     ──▶ jackioh-engine (reduce, view_for, legal_actions, fold)
```

The browser authenticates against Supabase Auth directly and sends the access token, which this
server verifies (JWKS, or the HS256 fallback). It never brokers a password outside E2E mode, except
a sign-in by username, which it makes with the account's address and answers without it (R1443).

| Layer | Modules | Rule |
| --- | --- | --- |
| Boot and routes | `main.rs`, `app.rs`, `loops.rs` | the route table (`ROUTES`, in the order the router tries them), the background loops (each started through `loops::spawn_supervised`, which logs a panic as `loop.crashed` and restarts the loop), `serve`; on SIGTERM or Ctrl-C, `serve_until`'s drain (loops aborted, requests finished, actors stopped with 1001, store closed) inside `SHUTDOWN_DRAIN_SECONDS` |
| Handlers | `api/*.rs` | `pub async fn <name>(app: &Arc<App>, req: Req) -> ApiResult`; no SQL of their own |
| Match | `actor/match_actor.rs`, `clock.rs`, `registry.rs`, `rooms.rs`, `ws_server.rs` | state in memory; time from `tokio::time` |
| Wire | `actor/protocol.rs`, `actor/contracts.rs` | every WebSocket frame |
| Persistence | `db/store.rs` (`Db`, `Tx`), `db/pg.rs`, `db/fake.rs`, `db/migrate.rs` | one method per store operation, over Postgres or the in-memory fake |
| Ranked | `ranked/*.rs` | Glicko-2, the ladder and seasons, pure |
| Ladder bots | `bots/persona.rs`, `seat.rs`, `fallback.rs`, `pick.rs`, `pool.rs`, `calibrate.rs` | the R645 personas ported from the web, a bot's move and think time, which waiting ticket gets which bot, its picks in a Conquest series, the pool, and the strengths' measured seed ratings (R1453, R641000, R1454, R1452, R748000) |
| Usernames | `username/` | a username's form, the profanity filter (its blocklists compiled in) and the case-fold key (R1432–R1434) |
| Play together | `social/` | friends, requests, blocks and presence, friend challenges, chat (masked by the usernames' filter), replay shares and the watch socket (§9.4, §9.5) |
| Duos co-op | `duos/` | co-op rooms, the co-op actor (a `DuosState` in memory, the AI team's seats played by `jackioh_ai::duos_decide`, the round clock, team pings) and `live_match_of` (§13) |
| Config | `env.rs`, `config.rs` | every environment variable and every server number |

There are no traits: `Db`, `Tx` and `Auth` are enums with one variant per implementation (`Pg` and
`Fake`, `Supabase` and `E2e`). A transaction is `let mut tx = app.db.begin(sub).await?; …;
tx.commit().await?`.

**The seams to the pure crates.** `actor/engine.rs` is the one path to the engine's game functions
(`create_game`, `begin_game`, `reduce`, `legal_actions`, `view_for`, `fold`) and deals All Random's
decks with `jackioh_ai::build_ai_deck` (R258), leaning a seat's deck on the set its player picked,
"newest" being the newest set that ships (`deal_random_deck(seed, lean_of(lean))`, R1372, R924000); it calls
`jackioh_cards::register_all()` first. A ladder bot's moves come from `jackioh_ai::decide_styled`
(`bots/seat.rs`), run on the blocking pool so no search holds up the runtime (R1452). Every whole-log
fold on an async path (a rebuild's three, a replay's opening, the reaper's, a game record's and `timing-backfill`'s) runs there too,
one job per call, through `actor::engine::off_runtime`, its thread set up by `App::engine_thread` (#1035). The
deck and trio rules are `jackioh_engine::validator`, the same module the client runs through WASM;
no rule is restated here. A co-op match calls the engine's Duos functions (`duos_create`,
`duos_begin`, `duos_reduce`, `duos_legal_actions`, `duos_seats_to_act`, `duos_view_for`,
`duos_fold`) and the AI's `duos_decide` the same way, deciding nothing of its own (§13). The
catalog and its version are compiled in from `crates/cards`.

## Running it

```
cargo build --release -p jackioh-server
E2E=1 target/release/jackioh-server                  # the test-server mode e2e uses: :8787, fixture accounts, in-memory store
target/release/jackioh-server --help                 # every subcommand
```

Against a Supabase project, copy `crates/server/.env.example` to `crates/server/.env`, fill it in,
and export it: the binary reads the process environment and nothing else.

```
set -a; . crates/server/.env; set +a
target/release/jackioh-server migrate           # 1. the schema: every migrations/*.sql in order, ledger in app.migrations
target/release/jackioh-server seed-catalog      # 2. public.cards at the compiled-in catalog version (again after every patch)
target/release/jackioh-server mint-code         # 3. an invite code (--max-uses=N, --expires-in-days=N)
target/release/jackioh-server seed-accounts     #    or active test accounts that skip the gate; refused under NODE_ENV=production
target/release/jackioh-server                   # 4. serve (the default subcommand)
```

`release` runs 1, 2 and then serves; it is what the Docker image runs on Render (`Dockerfile`,
`render.yaml`), so a deploy migrates and reseeds the database itself. Both steps are idempotent.

- **Migrations are append-only.** `migrate` refuses to start when a file the database already
  applied has changed (its FNV-1a checksum in `app.migrations`). Change the schema with a new file.
  `tests/store/migrations_pinned.rs` pins every checksum, and a new migration adds its line there.
- **Invite codes.** `mint-code` is a thin wrapper over `api::codes`'s mint, the only thing that
  creates one: the plaintext goes to stdout once, the database stores its keyed hash. It validates
  the whole environment, because a `CODE_PEPPER` that differs from the running server's mints a code
  nobody can redeem.
- **Invite clusters.** `invite-clusters [--window=<seconds>] [--min-accounts=<N>]` is SPEC §9.8's
  watch for accounts sharing an IP hash: it reads `code_attempts` (`DATABASE_URL`) and prints a
  Markdown table of every hash that at least N distinct accounts (default `INVITE_CLUSTER_MIN_ACCOUNTS`,
  6) made an attempt from in the window (default `INVITE_CLUSTER_WINDOW_SECONDS`, a week; at most the 30
  days the table is kept), with its account and attempt counts. It is read-only and advisory: nothing
  is banned or disabled, since a campus or a carrier's NAT puts many honest players behind one hash,
  and a hash is printed shortened to `INVITE_CLUSTER_HASH_PREFIX_LEN` characters.
- **Moderation.** `moderate ban <profile-id>` / `moderate unban <profile-id>` ban an account and
  restore it through `profiles_set_status`: a banned account's next Active request answers
  `account_banned` and its `/ws/match` socket is refused, while the running actor is untouched
  and the reaper owns the in-flight game (the account-deletion posture, so a ban lands at once
  even mid-match). `moderate revoke-code <CODE>` revokes an invite code by its hash, which then
  redeems exactly as an exhausted one. `moderate redemption off` / `moderate redemption on`
  pause and resume redemption through §9.4's database-side circuit breaker, touching no account
  or session. Each lever is idempotent and prints what changed (`banned <id>`, `already banned
  <id>`, `revoked <CODE>`, `redemption paused`, …); an unknown profile or code is refused
  without writing anything, and a pending account is never unbanned — that would skip the
  invite gate and R111's launch grant, so ban pending accounts only to stop abuse. Like
  `mint-code` it reads the whole environment, because a `CODE_PEPPER` that differs from the
  running server's revokes a hash redemption will never look up. Stdout is the audit trail:
  the CLI has no log pipeline, so the line it prints is the record of the lever.
  `moderate chat-reports` is the one read among them, and writes nothing: every chat report the
  store holds (B53), oldest first, each with when and why it was made, the reporter's and the
  sender's profile ids and the message as stored. A report goes with its message after
  `CHAT_RETENTION_DAYS`, so `docs/compliance.md` §6 reads them weekly.
- **Seasons.** `season-start --dry-run` walks R609's season open and soft reset and rolls it back,
  printing the report; without the flag it commits. Re-running is safe.

## Environment

`env.rs` validates everything at boot and refuses to start on any missing or malformed value, naming
each. Under `E2E=1` the Supabase, database, pepper, origin and catalog variables default to fixture
placeholders (`app::load_server_env`).

| Variable | Required | What it is |
| --- | --- | --- |
| `SUPABASE_URL` | yes | `https://<ref>.supabase.co` |
| `SUPABASE_SECRET_KEY` | yes | `sb_secret_…`; bypasses every RLS policy, server only |
| `DATABASE_URL` | yes | the Postgres connection string for the transactions of §9.4 and §9.5 |
| `CODE_PEPPER` | yes | ≥ 32 characters; keys the HMAC over invite codes and IP addresses |
| `CATALOG_VERSION` | no | the server always serves the version compiled into the binary (`cargo jackioh catalog-version`); a different value only logs a warning, so a stale host value never stops a deploy (R105, R388, #488) |
| `SUPABASE_JWKS_URL` | no | defaults to `${SUPABASE_URL}/auth/v1/.well-known/jwks.json` |
| `SUPABASE_JWT_SECRET` | no | the HS256 fallback; discouraged |
| `PORT` | no | default 8787 |
| `PUBLIC_ORIGINS` | no | allowed browser origins, for CORS and the WebSocket `Origin` check (R162) |
| `NODE_ENV` | no | `development`, `test` or `production`; `E2E` with `production` is refused |
| `E2E` | no | BUILD M8's test-server mode: fixture auth, accounts and codes, the in-memory store |
| `TRUSTED_PROXY_HOPS` | no | R190's `X-Forwarded-For` entries the deployment's proxies append, 0 to 5, default 0 |
| `RENDER_GIT_COMMIT` | no | set by Render; `GET /api/catalog` reports it as `x-deployed-commit` |
| `LADDER_BOTS` | no | `on` seats a ladder bot against a ranked ticket of any mode left alone past its fallback time (a Conquest ticket's in a series, R641000), and keeps the bot pool (R1453, R1454, R1452); off unless `on` (unset or `off`), and any other value refuses the boot. The bots decide in this process, so turning it on wants a host with room for the AI's search, such as a paid Render instance |

`mint-code`, `moderate` and `seed-accounts` read the same environment (`seed-accounts` also
`SEED_ACCOUNTS_PROJECT` and `SEED_ACCOUNTS_PASSWORD`); `stats-cards`, `stats-import`, `stats-fold`,
`stats-export`, `balance-watch` (without `--file`), `timing-backfill` and `timing-fit` read `DATABASE_URL`. The client's half is `VITE_*` (`apps/web/.env.example`); `env.rs`'s
`PUBLIC_ENV_VARS` and `SERVER_ONLY_ENV_VARS` are disjoint by design.

## HTTP surface

Every response is JSON; errors are `{ "error": { "code", "message", "details"? } }` with the codes
of `ApiErrorCode` (`api/http.rs`). Each route declares its auth level in `app.rs`'s `ROUTES`: `None`
(open), `User` (a verified token and a profile of any status: the code screen) or `Active` (also
`status = 'active'`; a pending account gets 403). A wrong method on a known path is 404; a body over
65,536 bytes or not a JSON object is 400.

| Method | Path | Auth | What |
| --- | --- | --- | --- |
| `POST` | `/api/auth/signin` | none | E2E mode only: a fixture account's session |
| `POST` | `/api/e2e/reseed` | none | E2E mode only, elsewhere a path nobody serves (404): R144's reseed on demand, which spec 10 calls first; answers `{ profiles, grantedCards, codes }` |
| `POST` | `/api/auth/username-signin` | none | a username and password: the session, never the address; limited per address and per username (R1443) |
| `GET` | `/api/auth/me` | user | profile status, whether a code is still needed, the current match and series, the caller's `username: { name, nextChangeAt, promptOwed }` (R1435), `ageCheckOwed` and `termsOwed` (R757000, R757001), and `queued` (R897000) |
| `GET` | `/api/profile` | active | the account screen: who the caller is and their record |
| `GET` | `/api/username/preview?name=…` | active | the exact username a save would give, tag included, or why it is refused (R1432–R1435) |
| `PUT` | `/api/username` | active | save the previewed username; 409 with a fresh preview when the outcome changed (R1434, R1435) |
| `POST` | `/api/username/skip` | active | answer the username prompt by keeping `Player#n` (R1435) |
| `DELETE` | `/api/account` | user | delete the caller's account and every row only it owns; 409 in a live match, series or co-op match |
| `POST` | `/api/account/consent` | user | `{ termsVersion, birthYear?, birthMonth? }`: the age screen's answer (an account under `MIN_ACCOUNT_AGE_YEARS` is deleted, 403 `under_age`) and agreement to `TERMS_VERSION` (409 `update_required` for any other); stores only that the age check passed, never the birth date (R757000, R757001) |
| `GET` | `/api/catalog` | none | `{ version, defs }`, the bytes the client ships (R163); `x-deployed-commit` |
| `POST` | `/api/codes/redeem` | user | §9.4's six-step redemption; first 403 `consent_owed` for an account owing the age check or the terms, costing no attempt (R757000, R757001) |
| `GET` | `/api/codes/status` | user | whether redemption is paused, and the caller's attempts left (R192) |
| `GET` | `/api/collection` | active | the ledger; there is no write route |
| `GET`, `PUT`, `DELETE` | `/api/decks`, `/api/decks/:id` | active | saved decks as drafts, by client-minted id (R250, R256) |
| `PUT`, `DELETE` | `/api/trios/:id` | active | trios (R252) |
| `POST` | `/api/trios/import` | active | a trio code's decks and trio in one transaction (R340, R341) |
| `GET`, `POST` | `/api/custom-cards` | active | the caller's custom cards, oldest first, with `limits`; a new one (201), its id minted and at version 1 (R915008) |
| `GET`, `PUT`, `DELETE` | `/api/custom-cards/:id` | active | one of the caller's cards; a save naming the version it replaces (409 with the current one when another save came first); a delete (R915008) |
| `POST`, `DELETE` | `/api/queue` | active | enqueue `{ mode: "bo1", deckId }`, `{ mode: "bo3", trioId }` or `{ mode: "random" }` (R257, R253), or leave; a ticket unheard from for `QUEUE_TICKET_STALE_MS` is cancelled before it can pair (R897000) |
| `GET` | `/api/queue/population` | user | the queue readout: each mode's expected wait and the last hour's games (R640002) |
| `POST` | `/api/rooms`, `/api/rooms/:code/join` | active | a room code in a mode, and its atomic claim (R264) |
| `GET`, `POST` | `/api/series/:id`, `/pick`, `/forfeit` | active | a Conquest series as its player may see it, a sealed pick, a forfeit (R330–R336) |
| `GET` | `/api/matches/:matchId/series`, `/ranks` | active | a match's series; both seats' ranks, each with its `profileId` and `username` (R604, R612, R1436) |
| `GET`, `POST` | `/api/matches/:matchId/rematch` | active | rematch offers (R672) |
| `GET` | `/api/ranked`, `/api/leaderboard` | active | the caller's season and rank; the ladder; every player shown with their `profileId` and `username` (R608, R612, R1436) |
| `GET`, `PUT` | `/api/tutorial`, `/api/settings` | active | the account's tutorial progress (R320) and settings (R633, R634), merged; the settings carry `allowSpectators`, whether friends may watch (§9.5) |
| `GET` | `/api/stats/cards`, `/api/stats/cards/:id`, `/api/stats/players` | none | public card and player statistics, a player's with their `profileId` and `username` (R654, R1436) |
| `GET`, `PUT` | `/api/stats/player` | active | the caller's own tracked statistics and privacy setting (R654) |
| `GET` | `/api/replays`, `/api/replays/:matchId` | active | the caller's finished matches whose log is kept, each with the opponent's `profileId` and `username` (R1436); one seat's steps of one (R768), or the sharer's seat's to a friend holding a share (§9.5) |
| `GET` | `/api/friends` | active | the caller's friends with their status (`offline`, `online`, `inMatch`) and whether they may be watched, the requests in and out, and the players the caller blocks (§9.4), read in one transaction by a fixed number of set-based queries whatever the number of friends |
| `POST`, `DELETE` | `/api/friends/requests`, `/:id/accept`, `/:id/decline`, `/api/friends/requests/:id`, `/api/friends/:profileId` | active | a request by exact `name#tag` (an unknown name and a blocked pair answer the same 404; `self`, `already_friends`, `already_requested`, `friend_cap` are 409; past `FRIEND_REQUESTS_PER_HOUR` is 429; a mutual request is accepted at once), its answer or its cancel, and removing a friend (which deletes the replay shares between the two, either way, and lets go a watch of either by the other) |
| `POST`, `DELETE` | `/api/blocks`, `/api/blocks/:profileId` | active | block (removing the friendship, every request and every replay share either way, and letting go a watch of either by the other) and unblock |
| `POST` | `/api/presence` | active | the heartbeat, every `PRESENCE_HEARTBEAT_MS`; a friend reads as online for `PRESENCE_TTL_MS` after it; records nothing while the account owes the terms (R970002) |
| `GET`, `POST`, `DELETE` | `/api/challenges`, `/:id/decline`, `/api/challenges/:id` | active | challenge an online, idle friend with `POST /api/rooms`'s body plus `friendId`, making the room and the challenge (409 `busy`, `offline`, `already_challenged`); the open challenges in and out; decline, cancel; `CHALLENGE_TTL_MS` frees an unanswered one's room, and the sweeper then deletes every challenge row that old; past `CHALLENGES_PER_HOUR` an hour a challenger gets 429 |
| `GET`, `POST` | `/api/chat/:friendId`, `/api/chat/unread`, `/api/chat/messages/:id/report` | active | a page of a conversation (`CHAT_PAGE`, newest last, marked read), a message to a friend (1 to `CHAT_MAX_CHARS` once trimmed, 422 `empty` or `too_long`, masked, `CHAT_MESSAGES_PER_MINUTE` a sender), unread counts, a report |
| `GET`, `POST`, `DELETE` | `/api/replays/shared`, `/api/replays/:matchId/shares`, `/api/replays/:matchId/shares/:friendId` | active | the replays shared with the caller; share a finished match the caller held a seat in with a friend, and unshare it |
| `POST`, `GET`, `DELETE` | `/api/duos/rooms`, `/api/duos/rooms/:code`, `/api/duos/rooms/:code/join` | active | a co-op room with a difficulty and the host's deck or a random one (`201 {code}`); the room; the guest's join, which starts the match (host A1, guest A2, the AI B1 and B2; 409 `full`, `own_room`, `update_required` for a host's deck frozen under another catalog; `conflict` for a host's fourth open room, R1047002); the host's cancel before anyone joins. A room nobody joins lapses after `DUOS_ROOM_TTL_MS`; to a blocked pair, and once joined to anyone but its two people, a room reads as an unknown code (404) (§13.9) |
| `GET` | `/api/duos/live`, `/api/duos/matches/:matchId/replay` | active | the caller's live co-op match, for the menus' way back (R765); a human seat's own Duos views of a finished co-op match, step by step, folded once on the blocking pool under the match's catalog version and kept as the 1v1 replays are, 404 to anyone else, 409 while it is live, 410 once its log is purged, 422 `replay_unavailable` (`earlier_patch`, `rules_changed`) when it cannot be folded to its recorded end, held to `REPLAY_REQUESTS_PER_MINUTE` (§13.9) |
| `GET` | `/healthz` | none | liveness: `{ status, version }` and `x-deployed-commit`; reads nothing from the store (#861) |
| `GET` | `/readyz` | none | readiness, render.yaml's `healthCheckPath`: the same once the store answers `select 1`, else 503 `unavailable` (`READINESS_PROBE_TIMEOUT_MS`) |

Deck and trio codes are the client's business; their format versions (`DECK_CODE_VERSION`,
`TRIO_CODE_VERSION`) are server numbers in `config.rs` (R255, R339).

**Custom cards** (SPEC §12.9, R915008, migration 0037). An active account keeps up to
`CUSTOM_CARDS_PER_ACCOUNT` (50) custom cards in `public.custom_cards`, whose row-level security lets
only the owner read its rows; no client role writes one, and the server writes through
`app.create_custom_card` (the cap, counted under the profile lock, against the smaller of its own
number and `app.settings.max_custom_cards`) and `app.update_custom_card` (compare-and-set on the
version). Each card is the engine's `CustomCard`: `POST` takes `{ frame, source, builder? }` and
mints the id (`custom-` and 32 lowercase hex digits) at version 1; `PUT` takes `{ version, frame,
source, builder? }`, `version` the one it replaces, and answers the card at that version and one, or
409 `conflict` with `details.version` when another save came first; a refusal of the engine's
validator (`custom::validate_custom_card`, R915001) is a 400 whose message is the first issue's and
whose `details.issues` are its `CustomIssue`s; the cap is a 409 with `details.limit`; another
profile's card answers as a missing one (404). A body is read under `API_MAX_BODY_BYTES` like every
other, which a card at every limit fits (the router reads a body before it picks the handler, so
there is no cap of the routes' own). **The server never compiles or runs a script**: it does not
link `jackioh-luau` (#902 V4), and a source is text it stores; the creator checks a script with the
browser's Luau module before it saves it. A deck naming a custom id saves as a draft (D3, R250), and
every queue, room and rematch path refuses it with the validator's L7 (R915005): the queue and the
rooms through `freeze_choice`, and a Best-of-1 rematch, which replays frozen decks, through
`decks::assert_no_custom_cards`.

## WebSocket surface

One socket per player per match, at `/ws/match` on the same port. The token comes from `?token=`; the
server echoes the `jackioh.v1` subprotocol when offered. The message union is `actor/protocol.rs`:
the client sends `hello` and `action`, the server sends `hello`, `view`, `ack`, `error`, `prompt` and
`clock`, and the cosmetic `portraits` (R642), `emote` (R643) and `aim` (R738) frames ride beside them,
never part of an action or a view. The `portraits` frame also carries the receiving account's own emote
hand, eight of the pool dealt from the match seed (R1341, R1342), and the actor drops an emote outside
the sender's hand as silently as a rate-limited one. A frame over `MAX_FRAME_BYTES` closes the socket with 1009.
The pump pings every `WS_PING_INTERVAL_SECONDS` and drops a socket that has sent no frame for
`WS_IDLE_TIMEOUT_SECONDS`, which starts its seat's grace; a socket's outgoing queue holds
`WS_OUTBOX_MAX_FRAMES` frames, and the one that overflows it is closed (R1441).

What the actor holds, each with a test named after it:

- Every action carries a client nonce and is deduped: a reused nonce returns the original ack, with no
  second reduce, log row or view (§9.3). A client nonce starting `srv-` is refused (R270).
- `reduce` refuses illegal actions and the actor relays the reason; it never re-implements a rule.
- The server stamps `playerId` from the authenticated seat, through `seat_played_by` after a Glitch
  swap (R677).
- The only per-player payload is `view_for`; no frame carries a `GameState`.
- Each home seat keeps an `actor::guard::SeatGuard`: its action window (`MATCH_ACTIONS_PER_SECOND`,
  R137), the count of its refusals that raises `match.rejections.unusual` once (R835000) and its
  malformed-frame budget, past which its socket is closed with 1008 (R835002).
- Every resolved action is appended to `match_actions`. A rebuilt actor folds `(seed, decks, log)`
  with the last boards, Glitch boards and dealt seats frozen into the match at its start (R417, R678,
  R433), and a reconnect gets a fresh view, never a log replay (§9.5).
- Clocks (R79, R268) are `config.rs` constants and live here, never in the engine: an expiry is a
  server-only action (`timeout`, `disconnectExpired`, `ceilingReached`) that goes through `reduce` and
  the log like any other. Every terminal reason writes exactly one `results` row and moves the rating
  once when the match is ranked (R604); a voided game writes none (R679).
  A clock's server action whose row the store fails to write is tried again `MATCH_SERVER_ACTION_RETRY_MS`
  later, its clock run out once more, until it lands; an append the store refuses as
  `StoreError::SeqClash` makes the actor let go as an unrecorded result does (R1437), for the next
  socket to rebuild it from the log (R776000).
- A seat a ladder bot holds has no socket: the actor plays it itself (`drive_bots`), its moves
  server actions with `srv-bot-<seq>` nonces through the same path a client's take, its seat never
  disconnected and so never given a grace, and its persona's emotes relayed as a player's are (R1452).
  It counts as attached as the actor arms, so a person who has not opened the match is away from
  then (R744), and it reads as present (`presence`) until the game ends. As for any match, a
  restart's actor is rebuilt when the person's socket comes back.

After a result commits, `api::results` files the game in `game_records` (`api::game_records`, the
engine's `summarize_game`) under the match's mode and the compiled-in newest patch, as `source:
"live"`, and the store folds it into the card statistics' totals in the same transaction (R1044000);
a failure is logged as `game.record.failed` and the result stands.

### The co-op socket (§13)

A co-op match has its own socket, `/ws/duos`, authenticated as `/ws/match` is. The client sends
`hello {matchId}`, `action {nonce, body}` (a `DuosActionBody`: a duel action, a pass or the team's
concede) and `ping {ping, target}`; the server sends `view {view, legal}` (the seat's `DuosView` and
its legal actions), `ack {nonce, seq}`, `error {code, message, nonce?}` (`malformed`,
`illegal_action`, `forbidden`, `rate_limited`, `match_over`, `internal`), `clock {now, roundDeadline,
graceDeadlines}`, `teammate {connected, aiPlaying}` and the teammate's `ping`. Before its hello the
socket waits behind `actor::guard::HelloGate`: a frame that is not the hello is answered `malformed`
and past `MALFORMED_FRAMES_PER_WINDOW` of them the socket is closed with 1008, and a hello that does not
come within `WS_IDLE_TIMEOUT_SECONDS` is refused (R835002). The co-op actor holds what the match actor
holds, with four seats:

- the seat is stamped from the verified token, the host's A1 and the guest's A2; B1 and B2 are the
  AI's, played here through `duos_decide`, one action every `DUOS_AI_STEP_DELAY_MS`, whenever they owe
  one;
- every applied action goes to `duos_match_actions` and pushes fresh views to both human seats; an
  illegal action answers `illegal_action` with its nonce and changes nothing; a rebuilt actor folds the
  log with `duos_fold` on the blocking pool, only for a profile that holds a seat, and the end writes
  the result and `ended_at` once, unranked, for both humans; every engine call runs under the catalog
  version the match was dealt from (`DuosMatchOptions::catalog_version`, R767000);
- a nonce starting `srv-` or `duos:` (the engine's round's own) is `malformed`, and a `hello` repeated
  while one from the same socket is still queued is absorbed (R835001's rule);
- each human seat keeps the same `actor::guard::SeatGuard` as a 1v1 seat: `MATCH_ACTIONS_PER_SECOND`
  (R137, the overflow answered `rate_limited`), the malformed-frame budget and its 1008 close (R835002),
  and the count of refusals, the flood's included, that raises `match.rejections.unusual` once per
  seat per match (R835000). Only a person's own refusals count: the AI's `srv-` moves never do;
- the round clock (`DUOS_ROUND_CLOCK_MS`) answers every unended round seat's prompts by the AI and
  ends its turn; a prompt held off the round has `DUOS_PROMPT_CLOCK_MS`; the four mulligans share
  `DUOS_MULLIGAN_CLOCK_MS`, whose expiry keeps each owing person's whole hand (R268); a move the AI
  decided under one hold of a seat is dropped if the hold changed before it applies;
- the match's ceiling is `MATCH_CEILING_MS` after its row's `created_at` (R1047000): the actor sends the
  engine's `ceilingReached` as one server action, and the `ceiling` draw ends the game and writes its
  result, also when a rebuilt actor arms past it; the sweep ends a live match it cannot fold back once
  that deadline has passed, with the same result, which frees both people (R1047001);
- a human seat disconnected past `DISCONNECT_GRACE_SECONDS` is played by the AI until it reconnects,
  its teammate told `teammate {aiPlaying: true}`; a team never loses to a disconnect;
- a ping is shape-checked (a `hand` target names the sender's own team's side and an index inside that
  hand), limited to `PING_RATE_LIMIT` per `PING_RATE_WINDOW_MS` (the rest dropped silently) and
  relayed to the teammate's socket alone; it never enters the log, a replay or a view. A co-op match
  deals no emote hand and relays no emote or aim.

### The watch socket (§9.5)

A friend watches at `/ws/watch`, authenticated as `/ws/match` is: `hello {matchId, friendId}` for a
friend's live, unranked match whose player allows watching (`profiles.allow_spectators`) and has fewer
than `SPECTATORS_PER_MATCH` watchers. The watcher gets `view {view, legal: []}`, the friend's seat's
`view_for`, each `SPECTATE_DELAY_MS` after the state it shows; anything else is `error forbidden` and a
close. Before its hello the socket waits behind the same `actor::guard::HelloGate` as `/ws/duos`: a
frame that is not the hello is answered `malformed`, past `MALFORMED_FRAMES_PER_WINDOW` of them it is
closed with 1008, and with no hello within `WS_IDLE_TIMEOUT_SECONDS` it is refused (`forbidden`, 4403)
(R835002). Every frame a watcher sends after `hello` is ignored, and leaving changes nothing in the match.
Both players' own match sockets get `watchers {count}` when the count changes. A watcher already in
gets the same `error forbidden` and close the moment the friend turns watching off (`PUT
/api/settings`), or the two unfriend or either blocks the other (`MatchActor::revoke_watchers`).

## Card statistics (§9.11, R376–R378, R1044000, R1044001)

```
target/release/jackioh-server stats-cards                                   # live games of every mode and patch, their human seats
target/release/jackioh-server stats-cards --mode=random --patch=v0.2.5 --pilot=human
target/release/jackioh-server stats-cards --card=core-002 --json
cargo jackioh stats --patch v0.2.5 --out v0.2.5-dev.jsonl                   # a pre-release AI run
target/release/jackioh-server stats-import v0.2.5-dev.jsonl                 # loaded; refused whole unless every line is a dev record
target/release/jackioh-server stats-cards --source=dev --patch=v0.2.5       # read beside the same patch's live games
target/release/jackioh-server stats-export --out=live.jsonl                 # the records still held in full, a line each, in id order
cargo jackioh stats report live.jsonl --json                                # the same figures with no database
target/release/jackioh-server stats-fold                                    # fold the records written before migration 0041; the retention loop does it too
```

Every figure, `stats-cards`, `balance-watch`, the public routes and the ladder bots' decks, reads the
totals each record is folded into as it is written (migration 0041: `record_totals`, `card_totals`,
`decklist_totals`; `Tx::stats_totals_load`, `Tx::stats_decklists`), by the engine's `StatsTotals`, the
counting a record file is read with, so a report is what the records gave. `stats-import` writes and
folds `STATS_FOLD_BATCH_RECORDS` (500) records to a transaction. A record written before migration
0041 is not folded until the retention loop folds it at boot or `stats-fold` does (R1044000). Once
folded, a record's detail is emptied `GAME_RECORD_RAW_RETENTION_DAYS` (7) after it was written by the
retention purge; its row and id stay, so the same game is never written or counted again (R1044001).

`stats-cards` takes `--source=live|dev|all` (live unless told otherwise), `--mode=bo1|bo3|random`,
`--patch=<version>`, `--pilot=human|ai|unified`, `--card=<id>` and `--json`. With no `--pilot` it
counts the human seats when it reads live data alone and every seat when it reads development data,
so a ladder bot's side is read only with `--pilot=ai` or `--pilot=unified`, or with `--source=all`,
which counts every seat of both sources unless a pilot is named (R1451).
The training box's gather lane (`training/README.md`, The gather lane) runs `cargo jackioh stats` without
end under its own series and loads each finished day with `stats-import`, so `stats-cards --source=dev
--patch=<version>` reads its games.

`stats-export` writes the records a `stats-cards` call with the same `--source`, `--mode` and `--patch` reads
whose detail is still held (the last `GAME_RECORD_RAW_RETENTION_DAYS`, R1044001), one `GameRecord` (R376)
per line in id order, to the file `--out=<file>` names (required, resolved against
the directory the command was started in, an existing file replaced). Every seat of every game is in the file,
so `--pilot` and `--card` are not its flags: they are the reader's. The file is what `cargo jackioh stats --out`
writes, so `cargo jackioh stats report` reads it, `analysis/` loads it and `stats-import` takes it when it holds
only development records (a live record is refused, R378). It reads
`DATABASE_URL` like `stats-cards`, writes nothing to the database, and holds live games unless `--source=dev`
or `--source=all` asks for a development run's (R378).

`balance-watch --patch=<version> [--source=live|all] [--file=<records.jsonl>]…` is the offline, advisory outlier watch whose
Markdown pastes into #699: per card it prints the in-deck win rate with its games, z-score against
50% and played delta, flagging too-strong and too-weak bands (`BALANCE_WATCH_Z_THRESHOLD`, 2.5, at
`BALANCE_WATCH_MIN_GAMES`, 40 games) and large played deltas (`BALANCE_WATCH_PLAYED_DELTA_THRESHOLD`,
15 points). Cards under `CARD_STATS_MIN_SAMPLE` games are listed as not enough games and never
flagged. Below `PUBLIC_STATS_MIN_LIVE_GAMES` live games the report says provisional; seat counting
matches the public endpoint (live records count human seats only, development runs every seat,
R1451). It reads `DATABASE_URL` or, with `--file` (repeatable), those record files (what `stats-export` and
`cargo jackioh stats --out` write, the gather lane's `~/training-out/gather/*.jsonl`), so the played delta, the
detector for a card the AI cannot use, runs over a development corpus with no database. The files are resolved
as `stats-import` resolves its file, filtered to the patch and source the query names, one record per id, so it
prints the database path's report; a file it cannot read, or a line that is not a record, is an error naming
the file and line. It changes no card and files nothing.

```
target/release/jackioh-server balance-watch --patch=v0.3.6
target/release/jackioh-server balance-watch --patch=v0.3.6 --file=2026-10-13.jsonl --file=2026-10-14.jsonl
```

## Play telemetry (§9.11, R1442)

```
target/release/jackioh-server timing-backfill   # the action logs still held, folded into action_timings
target/release/jackioh-server timing-fit        # human think times and habits fitted, rows to paste into config
```

Once a live match's result commits, the actor writes how it was played in one transaction
(`actor::telemetry`): `action_timings` (each person's move, its think time from the push that made
it the seat's move, the clock left, the rank bucket), `emote_events` and `match_signals` (concedes,
draws, rematches and timeouts), each keyed by match and seat with no profile id and no client grant
(migration 0029). A failure is logged as `telemetry.write_failed` and the result stands.

`timing-backfill` folds every finished match whose log `match_actions` still holds and that has no
action timing yet, the way a rebuilt actor replays it, into the rows the live path writes, less the
clock and the rank bucket the log does not hold; a match today's engine no longer folds whole is
skipped. `timing-fit` groups the human moves (`pilot = 'ai'` left out) by rank bucket, action kind
and first move of the turn, fits each bucket of at least `TIMING_FIT_MIN_SAMPLES` with a log-normal
and prints one `ThinkTimeFit` row per bucket (with its moves' mean `ln(legal_count)`), to paste by
hand into `config.rs`'s `THINK_TIME_FITS`, as `cargo jackioh sweep` prints its rows. It then prints
the ladder bots' other rows (R639000), from people's timings, emotes and signals beside their
matches' results (`play_telemetry_emotes`, `play_telemetry_outcomes`): the complexity slope and the
lethal shift a think time grows by, the concession, draw-answer, timeout and rematch rates, the two
emote delay windows and the persona weights, each a `pub const` to paste over the row of its name,
or a comment keeping the hand-set prior when fewer than `TIMING_FIT_MIN_SAMPLES` observations back
it. Neither takes an option.

## Ladder bots (§9.5, §9.12, R1453, R1454, R1451, R1452, R639000, R641000, R747000)

With `LADDER_BOTS=on`, a ranked ticket of any mode that has waited past its fallback time (15–40 s,
drawn from its id and a key the server keeps to itself) with no person inside its window is seated
against a ladder bot, which this process plays itself: in a match for Best of 1 and All Random, in a
series for Conquest. Off, no bot is seated and the pool is never sized; a bot already seated in a
series still makes its picks.

- `bots::fallback` is pure but for that key: which waiting ticket may have a bot (`fallback_due`)
  and which bot it gets (`choose_bot`: in the pool, on its schedule, in no match and a side of no
  series that is not over, not one of the player's last `BOT_RECENT_OPPONENTS`, for Best of 1
  holding decks and for Conquest a trio dealt for this catalog, the closest in rating; none for a
  player rated above every bot). The queue's sweep (`api/queue.rs`, `try_pair`) pairs every two
  people it can first, then claims a ticket and its bot in one statement (`tickets_claim_for_bot`,
  single-shot like a pair's claim) and starts the match, ranked, the person p1 and first to act as
  the older ticket of a pair is (R1453); a start that fails lets both go. A Conquest ticket and its
  bot (R641000) are claimed and made a series in one transaction (`start_bot_series`:
  `tickets_claim_for_bot_series`, which puts the bot in no match, then `start_series`): between
  games the series is what holds the bot, so every bot claim checks, after it locks the bot, that
  it is a side of no series that is not over (`lock_free_bot` in `db/pg.rs`, a statement of its own
  so it reads what an earlier claim committed). The person is series p1; the series is ranked and
  rated once as it ends, the bot's side as the bot it is (`plan_series_rating`).
- `bots::pick` is a bot's Conquest pick (R641000), which the series sweeper makes (`sweep_series`,
  `bot_picked`) once `pick_due_at` has come: `BOT_PICK_DELAY_MIN_MS` to `BOT_PICK_DELAY_MAX_MS` after
  the pick phase opened, before the clock runs out. The pick (`pick_slot`) follows the rule
  (`rule_slot`: the fastest deck by mean cost on the play, the slowest on the draw, of those that have
  not won) `BOT_PICK_RULE_SHARE` of the time and is uniform otherwise, and goes through `pick_deck`,
  sealed as a person's. Its streams are `{seed_base}:bot-pick-delay:{seat}:{game}` and
  `{seed_base}:bot-pick:{seat}:{game}`, from the series seed no client holds. A series game draws
  its bot no rematch plan (`MatchActorInput::series_game`), as a series game offers no rematch.
- `bots::seat` is a bot's move: `decide_styled` at the bot's strength (`BOT_STRENGTHS`: a search
  budget, a softmax temperature, a chance to miss a lethal and, R747000, a skill) on the spec's
  resources, then a
  think time from `THINK_TIME_FITS` or the hand-set priors, times the bot's speed, inside its
  acting clock, the search's deadline too; a move whose write failed is decided again
  `BOT_WRITE_RETRY_MS` later. A move decided before the person acted still goes while
  it is the same turn and the engine takes it, so the person's own actions never hold it back. Its streams (`{seed}:bot-ai:{home}`, `{seed}:bot-timing:{home}`) are the server's
  own, never the match rng, so a match with a bot replays from `(seed, log)` like any other, and a
  rebuilt actor resumes the bot's seat with its streams started again (R1452). R639000: the think
  time also grows with the move's legal actions (`THINK_COMPLEXITY_SLOPE`) and on a lethal decision
  (`THINK_LETHAL_SHIFT`), and a mulligan, a draw answer and a concession have priors of their own
  (`BOT_THINK_PRIORS`); at the start of its own turn the bot may concede, and it answers a draw
  offer by `bots::signals` rather than the AI's refusal (R188 holds for practice), never offering one
  (`never_played`). A rebuild reads the turn each bot last moved in off the log
  (`match_actor::bot_moved_turns`), so a restart in the middle of the bot's turn does not begin that
  turn again. R747000: the skill is rolled first, on the bot's AI stream (`jackioh_ai::plays_best`,
  no draw at full or zero skill); a decision the roll does not give to the AI is the random policy's
  (`random_action`) with no search, whose draw-offer answer is still the bot's own roll's, and `r188_forbidden`
  filters either.
- `bots::signals` draws a ladder bot's other habits from `config.rs`'s fitted rows (R639000), each
  on a stream of its own from cursor 0 (`{seed}:bot-concede:{home}`, `bot-draw`, `bot-idle`,
  `bot-rematch`, `bot-aim`), so the actor keeps no cursor and a rebuilt actor draws the same: its
  concession temper (`BOT_CONCEDE_FIT`, by turn and its deficit, only while it trails), a draw answer
  (`BOT_DRAW_ANSWER_FIT`, only while it trails), an idle turn its clock times out
  (`BOT_TIMEOUT_FIT`), its plan once a ranked match ends (`BOT_REMATCH_FIT`: how long it stays, whether it offers and whether
  it meets an offer, and when) and its aim's lead. Where it stands (`standing`) is read through
  `redact` on one `determinize`d world (`{seed}:bot-standing:{home}:{turn}`), since the evaluation
  cannot look up a face-down placeholder: the deficit on R1442's scale, which the concession's
  threshold is fitted on, and how far it trails its opponent, centred on an even board, which both
  guards read.
- `bots::aim` is the server's mirror of the web's aim (`apps/web/src/game/aim/aim.ts`): the aim a
  person's client would show for a targeted play, Activate, hero power or attack. The actor relays it
  through R738's relay (`receive_aim`, judged by `aim_is_public`) `aim_lead_ms` before the move and
  takes it down after.
- Once a ranked match a bot played ends, the bot reads as present (`presence`) until its plan's
  linger ends (the plan is drawn under the lock that ends the game, so there is no gap while the
  result is written), offers a normal rematch when its plan says once the result has landed, and
  meets a person's offer at their stakes when its plan accepts and it is still there, staying until
  its answer lands (`MatchActor::on_rematch_offer`, told by `api/rematch.rs`'s `offer_as` through
  `Registry::get`); its offers go through the route's own path (`offer_for_bot`). A rematch with a bot claims it in one statement (`bots_claim_for_rematch`: free
  and not retired, whatever its schedule) and lets it go if the start fails.
- `bots::persona` is the server's port of the practice AI's personas (`apps/web/src/practice/personas.ts`,
  which stays for practice), its tables `config.rs`'s `AI_PERSONAS` and `AI_EMOTE`, drawing as the
  web's does on its own stream (`{seed}:bot-persona:{home}`) and within the bot seat's dealt hand
  (R645, R1344). A ladder bot is dealt its persona by `BOT_PERSONA_WEIGHTS` (`pick_bot_persona`) and
  waits out its emotes inside `BOT_EMOTE_DELAY_FIT` and `BOT_EMOTE_REPLY_DELAY_FIT`
  (`EmotePersona::with_delays`), the rows `timing-fit` refits; the caps and R643's rate limit are
  unchanged (R639000).
- `bots::pool` sizes the pool as the server boots and every `BOT_POOL_REFRESH_INTERVAL_SECONDS`
  (`run_bot_pool`): `clamp(BOT_POOL_MIN, round(BOT_POOL_PER_WEEKLY_ACTIVE × weekly active people),
  BOT_POOL_MAX)` active bots, retiring the youngest accounts and reinstating the most recently
  retired first.
  A new bot draws a strength (its R610 rating seeded from it), a speed, a persona, a schedule, an
  account age and a username or the default `Player#n`; its two or three Best-of-1 decks are the
  patch's human Best-of-1 decklists, then decks dealt as All Random deals a player's
  (`deal_random_deck`: no shadow ban, no soft gate), each checked legal, and they are dealt
  again when the catalog version changes (R1454). Its Conquest trio (`draw_trio`, R641000) is its
  own decks that share no card, in order, then decks dealt with the cards taken left out
  (`deal_random_deck_excluding`), checked as a trio (`validate_trio`), and dealt again with them. The queue's sweeper lets go a bot whose claimed
  match was still unstarted `BOT_STALE_START_SECONDS` after the claim, with its player
  (`release_stale_starts`), which is what a restart mid-start leaves; it does so whatever
  `LADDER_BOTS` says, since a restart may have turned it off, and long before the reaper's
  open-row sweep (R1450) would delete the skeleton.
- `bots::calibrate` measures the seed ratings `BOT_SEED_RATINGS` holds (R748000,
  `docs/bot-calibration.md`). `jackioh-server bot-calibrate` plays every pair of `BOT_STRENGTHS`
  `BOT_CALIBRATION_GAMES_PER_PAIR` games as the bot seat plays them (the skill's roll, then
  `decide_styled` or the random policy, and no clock), fits each strength's Elo offset from the
  strongest and prints the two config rows; it reads no database, and its anchor is the seed of
  record shifted by the AI promotions in `training/history` since `BOT_CALIBRATION_GENERATION`. The
  seeds in `BOT_SEED_RATINGS` are the skill curve's (R747000, "Skill to Elo" below), which the
  calibration checks for the strengths as built; a row pasted over them goes over the curve's too. `bot-calibrate-live [--since=<epoch ms>]` reads the strongest bot's
  rated games against people (`ranked_bot_games_since`) and prints the rating they say it performs
  at, to give `bot-calibrate` as `--anchor`. Run it again after each AI promotion.

### Skill to Elo (R747000)

A strength's `skill` is the chance each of a bot's decisions follows the AI, the rest being a random
legal action (`crates/ai/README.md`, Matches). `expected_elo_for_skill(skill)` in `config.rs` is the
Elo a bot at that skill is expected to hold: `SKILL_ELO_TABLE` read by straight lines between its
rows, clamped to its ends, with the full-skill reference at `SKILL_ELO_ANCHOR` (1200: the strongest
bot's old seed, so that a player who has climbed still finds a bot to be paired with, R1453). Each
strength's `seed_rating` is that function at its skill, and a test holds them equal. The table is a
prior: a bot's live Glicko-2 rating (R610) is the calibration that goes on.

| Skill | Elo | Gap to the reference | Against the reference | Against random |
| --- | --- | --- | --- | --- |
| 0 (the random policy) | 570 | −628 | 21 of 800 | the null check: 107 wins, 1 draw, 92 losses of 200 |
| 0.25 | 740 | −461 | 14 of 200 (−449) | 142 of 200 (+155, −472) |
| 0.5 | 900 | −300 | 32 of 200 (−288) | 172 of 200 (+315, −312) |
| 0.75 | 1080 | −123 | 66 of 200 (−123) | not run |
| 1 (the reference) | 1200 | 0 | | |

The games are `cargo jackioh arena` between this build's AI at each skill and the reference, and
between each skill and the random policy, in a release build, on seeds a run's own name gives
(`elo-747/…`), with the seats alternating. "Against the reference" is the wins of the skill's side;
the figures in brackets are the Elo gap those games give and, against random, the gap to the
reference after adding the random policy's. The 0 row is the random policy exactly (skill 0 plays
`random_action` and nothing else), so its row is the reference's games against `random`: 779 wins of
800 for the reference. The reference against 0.0 is not run again, and 0.75 against random, which
the AI wins nearly always, resolves nothing, so it is not run. A cross-check ladder of adjacent
skills, 100 games each, agrees with the table to within its noise: 0.5 beat 0.25 73–27 (+173, the
table's +160), 0.75 beat 0.5 73–27 (+173, the table's +180), the reference beat 0.75 74–26 (+182,
the table's +120).

**Re-measuring** (after a new best AI, or when a set ships, and the table pasted over):

1. `cargo build --release -p jackioh-tools`, then, with the AI whose curve is wanted as `self`:
   `cargo jackioh arena --a self --b random --games 200 --seed "elo-<run>/random-vs-ref"`, four
   times with the seeds `…/random-vs-ref`, `…-b`, `…-c` and `…-d` (the shut-out needs the games);
   then for each skill
   `S` in 0.25, 0.5 and 0.75, `--a self --a-skill S --b self --games 200 --seed "elo-<run>/sS-vs-ref"`
   (the duel lines and the tally are on stderr, with no `--out`), and for 0.25 and 0.5,
   `--a self --a-skill S --b random --games 200 --seed "elo-<run>/sS-vs-random"`.
2. Score each run from the measured side: `p = (wins + draws / 2) / games`, held inside `[1 / 2N,
   1 − 1 / 2N]` for N games (so a shut-out reads as a bound, not infinity), and the gap
   `400 · log10(p / (1 − p))`. The random policy's gap to the reference is its own run's. A skill's
   gap is the mean of the gaps it has: its games against the reference, and its games against
   random plus the random policy's gap.
3. A row's Elo is `SKILL_ELO_ANCHOR` plus the gap, rounded to the nearest 10. Paste the rows over
   `SKILL_ELO_TABLE` and `BOT_SEED_RATINGS`, the strengths' `seed_rating`s (the test
   `r747000_strength_seed_ratings_match_skill_curve` fails until they agree), and the new numbers
   over this table.

A run of 200 games near an even match is good to about ±50 Elo (95%), and a run of 50 to about ±95,
which is as fast a check that an AI change moved the curve as is worth running; a shut-out run
(the random policy against the reference) is a bound, and why it has four times the games. The
curve is the AI at the arena's budget (`agent::decide_with`, the browser's): the budgets,
temperatures and missed-lethal chances of the strengths' rows are not measured by it, which live
ratings take in.

Migration 0032 adds `profiles.kind` (`human`, or `bot` for a ladder bot; no client role may read
it), the server-only `bots` table (one row per bot's profile: strength, speed, persona, schedule,
decks and when it retired; RLS on with no policy and no client grant), and swaps the profile's
foreign key to `auth.users` for triggers: inserting a person's profile still needs its auth user
and deleting an auth user still deletes its profile, but a bot's profile needs none; and it limits
the launch collection's backfill (`app.grant_launch_collection_all`, which every catalog stamp
runs) to people, since a bot owns no collection. A bot's rating
is a `bot_ratings` row (0022, R610), and its rank a `season_ranks` row its ranked games move as a
person's move theirs, read against people's ratings alone (R640000). Nothing a client reads carries
a profile's `kind`, a seat's pilot or a bot's nonces: `GET /api/leaderboard` and
`GET /api/matches/:matchId/ranks` show a bot's row as a player's, and Jlorious and the percentiles
read people's rows only, so a bot that would qualify stays Mythic Grape (R640000, R1452). The human
side of a bot game counts in every statistic like any live game's; the bot side is filed as `ai`,
with its bot id in `rated_games`, and is read only when a query asks for it (R1451).

Migration 0033 adds `bots.trio` and `bots.trio_catalog_version`: the Conquest trio the pool dealt
the bot, frozen as a ticket's is, and the catalog it was dealt for (R641000). The table stays
server-only.

Migration 0038 adds Play together's tables (§9.4, §9.5): `friend_requests`, `friendships` (one row
per pair, `a < b`), `blocks`, `presence` (server-only), `challenges`, `chat_messages` (1 to 500
characters), `chat_reports` (one per message and reporter) and `replay_shares`, each with RLS on and a
select policy for its own players only, and `profiles.allow_spectators` (on by default). The retention
loop deletes chat messages older than `CHAT_RETENTION_DAYS`. Migration 0039 adds the co-op tables
(§13.9): `duos_rooms` (the code, host, guest, difficulty, each side's deck choice, the match once it
starts, and its expiry), `duos_matches` (the seed, both humans, the difficulty, the create options,
the result) and `duos_match_actions` (the log, unique by seq and by nonce); the seed and the log are
the server's alone, as a 1v1 match's are: a room's host and guest may read it (but its frozen deck
choices), and no client role reads a co-op match or its log (no grant, no policy). The retention
loop deletes a co-op match's log `MATCH_ACTION_RETENTION_DAYS` after the match ended, as a 1v1
match's (`duos::purge_expired_logs`); the match row and its result stay.

Migration 0041 adds the card statistics' totals (R1044000): `record_totals`, `card_totals` and
`decklist_totals`, server-only like `game_records` (RLS on, no policy, no grant), and
`game_records.folded`. A record's `record` may now be null, and only once it is folded. The retention
loop first folds any record not yet folded (`api::game_records::fold_pending`), then
`app.purge_expired_rows`' fourth cutoff empties the detail of each folded record written more than
`GAME_RECORD_RAW_RETENTION_DAYS` ago (R1044001). The store folds under a transaction-level advisory
lock per patch and source, so two folds into the same rows never interleave.

## Tests

```
cargo test -p jackioh-server                          # the whole suite, on the in-memory store and fixture auth
cargo test -p jackioh-server --test server actor::    # one area: api::, actor::, store::
sh crates/server/tests/sql/run.sh                     # schema, RLS and trigger invariants (Docker)
sh crates/server/tests/db/run.sh                      # the store contract against Postgres (Docker; KEEP_DB=1 keeps it)
sh crates/server/tests/deploy/rehearse.sh             # Render's deploy rehearsed: the image, a non-superuser migration, a production boot (Docker)
```

`tests/server.rs` is one binary over `tests/{api,actor,bots,loops,store,support}/`. `support/deps.rs` builds a
test `App` on `Db::Fake` and `Auth::E2e` and calls the router with `tower`'s `oneshot`;
`support/engine.rs` installs test cards (`test-prompt-self`, `test-prompt-enemy`, `test-lethal`)
through the engine testkit's override; `support/socket.rs` is an in-memory socket; time is
`tokio::time::pause()` and `advance()`. The fake store is strict where Postgres is strict (a
transaction restores every table when it fails, claims are single-shot). `tests/store/contract.rs`
runs every case against `Db::Fake` always and against `Db::Pg` when `DATABASE_URL` is set, which is
what `tests/db/run.sh` sets, so a change to the store goes there. RLS and triggers are a property of
real Postgres, so only the three Docker suites check them (CI's `db` job).

`tests/export_config.rs` writes `apps/web/src/wire/serverConfig.ts`, the constants the client and e2e
read; CI fails when a run changes it.

## Conventions

- Server numbers live in `config.rs`, rules numbers in `crates/engine/src/config.rs`; nothing else
  states a number (CLAUDE.md rule 9).
- Logs are `tracing` JSON lines to stdout with stable `event` names (`server.listening`,
  `api.forwarded_for`, `match.fold.errors`, `game.record.failed`, …).
- Anything the spec does not decide is marked `// NOT IN SPEC:` where it is decided.
