# `jackioh-server`

The authority for everything that is not presentation: identity and the invite gate, the collection
ledger, saved decks and trios, an active account's tutorial progress (R320) and settings (R633), each
profile's last board (R417), matchmaking in three modes, the Conquest series, the ranked ladder, and
the match itself: one actor per match holding the `GameState` in memory, one WebSocket per player,
`reduce` on every action and `view_for` pushed to each player after every change (spec §9.1–§9.5,
§10.8). Every match that ends leaves a game record for the card statistics (§9.11, R376–R378).

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
server verifies (JWKS, or the HS256 fallback). It never brokers a password outside E2E mode.

| Layer | Modules | Rule |
| --- | --- | --- |
| Boot and routes | `main.rs`, `app.rs` | the route table (`ROUTES`, in the order the router tries them), the background loops, `serve` |
| Handlers | `api/*.rs` | `pub async fn <name>(app: &Arc<App>, req: Req) -> ApiResult`; no SQL of their own |
| Match | `actor/match_actor.rs`, `clock.rs`, `registry.rs`, `rooms.rs`, `ws_server.rs` | state in memory; time from `tokio::time` |
| Wire | `actor/protocol.rs`, `actor/contracts.rs` | every WebSocket frame |
| Persistence | `db/store.rs` (`Db`, `Tx`), `db/pg.rs`, `db/fake.rs`, `db/migrate.rs` | one method per store operation, over Postgres or the in-memory fake |
| Ranked | `ranked/*.rs` | Glicko-2, the ladder and seasons, pure |
| Config | `env.rs`, `config.rs` | every environment variable and every server number |

There are no traits: `Db`, `Tx` and `Auth` are enums with one variant per implementation (`Pg` and
`Fake`, `Supabase` and `E2e`). A transaction is `let mut tx = app.db.begin(sub).await?; …;
tx.commit().await?`.

**The seams to the pure crates.** `actor/engine.rs` is the one path to the engine's game functions
(`create_game`, `begin_game`, `reduce`, `legal_actions`, `view_for`, `fold`) and deals All Random's
decks with `jackioh_ai::build_ai_deck` (R258); it calls `jackioh_cards::register_all()` first. The
deck and trio rules are `jackioh_engine::validator`, the same module the client runs through WASM;
no rule is restated here. The catalog and its version are compiled in from `crates/cards`.

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

`mint-code` and `seed-accounts` read the same environment (`seed-accounts` also
`SEED_ACCOUNTS_PROJECT` and `SEED_ACCOUNTS_PASSWORD`); `stats-cards`, `stats-import` and `stats-export` read
`DATABASE_URL`. The client's half is `VITE_*` (`apps/web/.env.example`); `env.rs`'s
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
| `GET` | `/api/auth/me` | user | profile status, whether a code is still needed, the current match and series |
| `GET` | `/api/profile` | active | the account screen: who the caller is and their record |
| `DELETE` | `/api/account` | user | delete the caller's account and every row only it owns; 409 in a live match or series |
| `GET` | `/api/catalog` | none | `{ version, defs }`, the bytes the client ships (R163); `x-deployed-commit` |
| `POST` | `/api/codes/redeem` | user | §9.4's six-step redemption |
| `GET` | `/api/codes/status` | user | whether redemption is paused, and the caller's attempts left (R192) |
| `GET` | `/api/collection` | active | the ledger; there is no write route |
| `GET`, `PUT`, `DELETE` | `/api/decks`, `/api/decks/:id` | active | saved decks as drafts, by client-minted id (R250, R256) |
| `PUT`, `DELETE` | `/api/trios/:id` | active | trios (R252) |
| `POST` | `/api/trios/import` | active | a trio code's decks and trio in one transaction (R340, R341) |
| `POST`, `DELETE` | `/api/queue` | active | enqueue `{ mode: "bo1", deckId }`, `{ mode: "bo3", trioId }` or `{ mode: "random" }` (R257, R253), or leave |
| `GET` | `/api/queue/population` | user | open tickets, in total and per mode |
| `POST` | `/api/rooms`, `/api/rooms/:code/join` | active | a room code in a mode, and its atomic claim (R264) |
| `GET`, `POST` | `/api/series/:id`, `/pick`, `/forfeit` | active | a Conquest series as its player may see it, a sealed pick, a forfeit (R330–R336) |
| `GET` | `/api/matches/:matchId/series`, `/ranks` | active | a match's series; both seats' ranks (R604, R612) |
| `GET`, `POST` | `/api/matches/:matchId/rematch` | active | rematch offers (R672) |
| `GET` | `/api/ranked`, `/api/leaderboard` | active | the caller's season and rank; the ladder (R608, R612) |
| `GET`, `PUT` | `/api/tutorial`, `/api/settings` | active | the account's tutorial progress (R320) and settings (R633, R634), merged |
| `GET` | `/api/stats/cards`, `/api/stats/cards/:id`, `/api/stats/players` | none | public card and player statistics (R654) |
| `GET`, `PUT` | `/api/stats/player` | active | the caller's own tracked statistics and privacy setting (R654) |

Deck and trio codes are the client's business; their format versions (`DECK_CODE_VERSION`,
`TRIO_CODE_VERSION`) are server numbers in `config.rs` (R255, R339).

## WebSocket surface

One socket per player per match, at `/ws/match` on the same port. The token comes from `?token=`; the
server echoes the `jackioh.v1` subprotocol when offered. The message union is `actor/protocol.rs`:
the client sends `hello` and `action`, the server sends `hello`, `view`, `ack`, `error`, `prompt` and
`clock`, and the cosmetic `portraits` (R642), `emote` (R643) and `aim` (R738) frames ride beside them,
never part of an action or a view. A frame over `MAX_FRAME_BYTES` closes the socket with 1009.

What the actor holds, each with a test named after it:

- Every action carries a client nonce and is deduped: a reused nonce returns the original ack, with no
  second reduce, log row or view (§9.3). A client nonce starting `srv-` is refused (R270).
- `reduce` refuses illegal actions and the actor relays the reason; it never re-implements a rule.
- The server stamps `playerId` from the authenticated seat, through `seat_played_by` after a Glitch
  swap (R677).
- The only per-player payload is `view_for`; no frame carries a `GameState`.
- Every resolved action is appended to `match_actions`. A rebuilt actor folds `(seed, decks, log)`
  with the last boards, Glitch boards and dealt seats frozen into the match at its start (R417, R678,
  R433), and a reconnect gets a fresh view, never a log replay (§9.5).
- Clocks (R79, R268) are `config.rs` constants and live here, never in the engine: an expiry is a
  server-only action (`timeout`, `disconnectExpired`, `ceilingReached`) that goes through `reduce` and
  the log like any other. Every terminal reason writes exactly one `results` row and moves the rating
  once when the match is ranked (R604); a voided game writes none (R679).

After a result commits, `api::results` files the game in `game_records` (`api::game_records`, the
engine's `summarize_game`) under the match's mode and the compiled-in newest patch, as `source:
"live"`; a failure is logged as `game.record.failed` and the result stands.

## Card statistics (§9.11, R376–R378)

```
target/release/jackioh-server stats-cards                                   # live games of every mode, patch and pilot
target/release/jackioh-server stats-cards --mode=random --patch=v0.2.5 --pilot=human
target/release/jackioh-server stats-cards --card=core-002 --json
cargo jackioh stats --patch v0.2.5 --out v0.2.5-dev.jsonl                   # a pre-release AI run
target/release/jackioh-server stats-import v0.2.5-dev.jsonl                 # loaded; refused whole unless every line is a dev record
target/release/jackioh-server stats-cards --source=dev --patch=v0.2.5       # read beside the same patch's live games
target/release/jackioh-server stats-export --out=live.jsonl                 # the records stats-cards reads, a line each, in id order
cargo jackioh stats report live.jsonl --json                                # the same figures with no database
```

`stats-cards` takes `--source=live|dev|all` (live unless told otherwise), `--mode=bo1|bo3|random`,
`--patch=<version>`, `--pilot=human|ai|unified`, `--card=<id>` and `--json`.

`stats-export` writes the records a `stats-cards` call with the same `--source`, `--mode` and `--patch` reads,
one `GameRecord` (R376) per line in id order, to the file `--out=<file>` names (required, resolved against
the directory the command was started in, an existing file replaced). Every seat of every game is in the file,
so `--pilot` and `--card` are not its flags: they are the reader's. The file is what `cargo jackioh stats --out`
writes, so `cargo jackioh stats report` reads it, `analysis/` loads it and `stats-import` takes it when it holds
only development records (a live record is refused, R378). It reads
`DATABASE_URL` like `stats-cards`, writes nothing to the database, and holds live games unless `--source=dev`
or `--source=all` asks for a development run's (R378).

## Tests

```
cargo test -p jackioh-server                          # the whole suite, on the in-memory store and fixture auth
cargo test -p jackioh-server --test server actor::    # one area: api::, actor::, store::
sh crates/server/tests/sql/run.sh                     # schema, RLS and trigger invariants (Docker)
sh crates/server/tests/db/run.sh                      # the store contract against Postgres (Docker; KEEP_DB=1 keeps it)
sh crates/server/tests/deploy/rehearse.sh             # Render's deploy rehearsed: the image, a non-superuser migration, a production boot (Docker)
```

`tests/server.rs` is one binary over `tests/{api,actor,store,support}/`. `support/deps.rs` builds a
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
