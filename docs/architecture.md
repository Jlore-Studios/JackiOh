# JackiOh server architecture

**Scope.** How to take a fresh Supabase project and a checkout of this repo and end up with two
browsers playing a room-code match. This document is derived from SPEC.md §9 and §10.8 and from
BUILD.md M6–M7; where it says something SPEC §9 does not, it says so and names the SPEC §11 row that
records the decision (§11 below collects all nine, R104–R112). SPEC is the contract — if this document
and SPEC disagree, SPEC wins and this document is the bug.

**Audience.** A developer executing BUILD M6 and M7. Nothing here is a game rule; rules live in SPEC
§§2–8 and are implemented once, in `crates/engine`.

---

## 1. Component map

```mermaid
flowchart TD
  B["Browser<br/>apps/web (static bundle)"]
  CDN["Static host / CDN"]
  AUTH["Supabase Auth<br/>email + password, verification"]
  PGR["Supabase Data API (PostgREST)<br/>role: authenticated"]
  API["jackioh-server HTTP routes<br/>codes, collection, decks, trios, queue, rooms, series, tutorial"]
  ACT["jackioh-server match actor<br/>one per live match"]
  PG[("Supabase Postgres<br/>20 tables + private app schema")]
  ENG["crates/engine<br/>reduce / view_for / fold"]
  CAT["crates/cards<br/>catalog.json + scripts"]

  CDN -.serves.-> B
  B -->|HTTPS, publishable key + user JWT| AUTH
  B -->|HTTPS, publishable key + user JWT| PGR
  B -->|HTTPS, Bearer user JWT| API
  B -->|WSS, Bearer user JWT| ACT
  PGR -->|RLS enforced| PG
  API -->|DATABASE_URL, service_role| PG
  ACT -->|DATABASE_URL, service_role| PG
  API --> ENG
  ACT --> ENG
  ENG --> CAT
  B -.->|"WASM: hotseat, practice"| ENG
```

This is SPEC §9.2's topology with each box named. The only departure from the diagram in SPEC is that
the "API functions" and the "Match actor" are two responsibilities of **one process**
(`jackioh-server`, `crates/server`) rather than two deployment units; §5.3 below explains why, and the split is a
deployment decision that can be made later without touching the code.

| Component | Where it runs | Stateful? | Owns |
| --- | --- | --- | --- |
| `apps/web` | Static bundle on any CDN | No | Rendering `viewFor`, composing intent, the bundled catalog; hotseat and practice run the engine and the AI there as WebAssembly (`crates/wasm`), with no server |
| Supabase Auth | Supabase | Managed | Signup, password hashing, email verification, sessions, JWTs |
| Supabase Postgres | Supabase | Yes (durable) | The 13 tables of BUILD M6 plus `decks`, `trios` and `series` (R250–R263, R330–R341), `tutorial_progress` (R320) and `game_records` (R376), RLS, the private `app` schema |
| Supabase Data API | Supabase | No | Read-only projections to the browser, RLS-enforced |
| `jackioh-server` HTTP routes | One process (a Docker image on Render) | No | Redemption, collection reads, deck and trio saves and trio imports, enqueue in three modes, room create/join, the Conquest series and its sweeper, the tutorial's account copy (R320) |
| `jackioh-server` match actor | The same process | **Yes (in memory)** | `GameState`, two WebSockets, the turn clock, the action log |
| `crates/engine` + `crates/cards` | Linked into both of the above | No (pure) | Every rule, `reduce`, `view_for`, `fold` |

---

## 2. What the client is allowed to do

SPEC §9.1, restated as channels rather than domains:

| Channel | Credential | What it carries |
| --- | --- | --- |
| Browser → Supabase Auth | publishable key (`sb_publishable_…`) | signup, login (password, an emailed link or code, an OAuth provider), email verification, an authenticator app's enrolment and codes, token refresh (R664–R666) |
| Browser → Data API | publishable key + the user's JWT | **reads only**: own profile row, own collection, own decks and trios, own tickets, own results, own tutorial progress, the `cards` projection |
| Browser → server HTTP | the user's JWT as `Authorization: Bearer` | intent: "redeem this code", "save this deck", "enqueue Conquest with this trio", "pick this deck for game 2", "import this trio", "create a room", "join ABC234", "merge this device's tutorial progress" |
| Browser → server WebSocket | the user's JWT in the `hello` frame | intent: one `Action` at a time; receives `viewFor` and nothing else |

One rule, from SPEC §9.1: **the client sends intent, never state.** "Play instance 7 in zone 3 with
target 12", never "my deck contains these 20 cards", never "I own a Mythic".

---

## 3. Trust boundaries

Five boundaries. Each row names what crosses it, what enforces the crossing, and the failure mode if
the enforcement is missing.

| # | Boundary | What crosses | Enforced by | Failure mode if absent |
| --- | --- | --- | --- | --- |
| 1 | Browser → Supabase Auth | credentials | Supabase Auth | password handling done badly by us instead of them (SPEC §9.4: "managed auth provider") |
| 2 | Browser → Data API | read queries as `authenticated` | **RLS on every table**, default deny | a player reads another player's collection, or any invite code |
| 3 | Browser → server HTTP | intent + a JWT | JWKS signature verification; the profile id is the verified `sub` and never a request field | account takeover by claiming someone else's id |
| 4 | Browser → server WS | `Action` objects | `reduce` itself (SPEC §9.3: "`reduce` refuses illegal actions itself"); actor-side nonce dedupe and rate limit | illegal plays, action flooding (SPEC §9.8) |
| 5 | Server → Postgres | SQL as `service_role` / `postgres` | the secret key and `DATABASE_URL` are server-only env; the private `app` schema is not exposed | a leaked secret key bypasses every policy in the project |

### 3.1 Three things that must never reach a client

SPEC §9.1 and §10.8 name them; SPEC §9.8 lists the vector. They drive concrete policy decisions:

1. **Library order.** Consequence: `matches.seed` is not client-readable, and neither is
   `match_actions`. `(seed, log)` reconstructs the shuffle (SPEC §9.3), so handing a player the log is
   handing them every future draw. This is why `matches` and `match_actions` have RLS enabled and
   **no** SELECT policy at all, and why SPEC §9.5 says "Reconnect gets a fresh full view, never a log
   replay."
2. **The opponent's hand.** Consequence: only `viewFor` crosses the WebSocket, and the opponent's hand
   is a count (SPEC §10.8). BUILD M6-T4 makes this a protocol-level test.
3. **Face-down traps and the other player's pending options.** Consequence: `viewFor` renders the
   opponent's backrow as face-down markers and an open prompt held by the opponent as
   `{ forYou: false }`.

### 3.2 RLS matrix

`anon` gets nothing anywhere. Everything below is for the `authenticated` role, further narrowed by
the policy in the third column. `service_role` bypasses RLS and is the only writer.

| Table | Client read | Policy | Client write |
| --- | --- | --- | --- |
| `profiles` | own row | `id = auth.uid()` | **none** — identity is server-owned (§9.1) |
| `invite_codes` | **none** | no policy | none — a readable code row defeats the gate (§9.8) |
| `code_attempts` | **none** | no policy | none |
| `cards` | all rows | `true` | none — the catalog is static data (§9.4) |
| `collection` | own rows | `profile_id = auth.uid()` | **none** — §9.4: "no client path writes either" |
| `collection_grants` | own rows | `profile_id = auth.uid()` | none; append-only by trigger |
| `loadouts` | own row | `profile_id = auth.uid()` | **none** — retired by R254, kept unread so nothing saved is lost |
| `loadout_decks` | own rows | `profile_id = auth.uid()` | none |
| `loadout_deck_cards` | own rows | `profile_id = auth.uid()` | none |
| `decks` | own rows | `profile_id = auth.uid()` | **none** — the server upserts a draft by the id the client minted (R250, R256) |
| `trios` | own rows | `profile_id = auth.uid()` | none (R252) |
| `tickets` | own rows | `profile_id = auth.uid()` | none — enqueue freezes a deck (§9.5) |
| `matches` | **none** | no policy | none — holds `seed` and both decks (§3.1) |
| `match_actions` | **none** | no policy | none — append-only by trigger (§9.3) |
| `results` | rows you played in | `auth.uid() in (p1_profile_id, p2_profile_id)` | none |
| `series` | **none** | no policy | none — holds both frozen trios and the hidden picks (R259) |
| `tutorial_progress` | own row | `profile_id = auth.uid()` | none — the server merges a device's progress into it, never removing a lesson (R320) |
| `game_records` | **none** | no policy | none — holds both hands and both decklists of every recorded game (R376) |

Three Supabase-specific traps this schema avoids on purpose:

- **Views bypass RLS.** There are no views in the exposed schema. If one is added it must be
  `create view … with (security_invoker = true)`.
- **`SECURITY DEFINER` functions in an exposed schema are reachable over HTTP.** Every one of ours
  lives in the private `app` schema, which is not in the Data API's exposed schema list, so
  `app.redeem_invite_code`, `app.upsert_deck`, `app.upsert_trio` and `app.merge_tutorial_progress` have **no HTTP path at all** — they are reachable
  only over `DATABASE_URL`.
- **`user_metadata` is user-editable** and can appear in `auth.jwt()`. No policy or function reads it.
  Authorization comes from `profiles.status`, which only the server writes.

### 3.3 The gate

SPEC §9.4: `profiles.status ∈ pending | active | banned`, and "a pending account can log in, verify
its email and see the code screen, and nothing else: no collection, loadout, queue or match."

Two layers, both required:

- **Database.** `app.profile_is_active()` guards the write functions; a `pending` profile simply has no
  `collection` rows yet, because the launch grant fires on the `pending → active` transition.
- **API.** Every route but `/api/codes/redeem` and `/api/me` asserts `status = 'active'` and returns
  403 otherwise (BUILD M6-T1 acceptance).

---

## 4. Why the match actor cannot be a stateless function

SPEC §9.2: "The match actor (Durable Object or equivalent) is the only stateful component: it holds
the state in memory, one WebSocket per player, an alarm for the turn clock, and it appends every
resolved action to the log."

The binding constraint is the clock. SPEC §9.5 and R79 require a 75-second turn clock, a separate
30-second prompt clock for a prompt held by the non-active player, a 60-second disconnect grace and a
60-minute match ceiling. ARCHITECTURE-CCG §2.1 states the consequence plainly: **a stateless function
cannot run a timer** — it only reacts to requests, and "the opponent never sends one" is exactly the
case the clock exists to handle. A player who closes their laptop mid-turn must lose the turn, then
the match; nobody is going to send the request that makes that happen.

Three further properties push the same way:

- **Ordering.** A single-threaded actor makes action ordering free. Two players acting in the same
  tick through separate stateless invocations would need a lock or a compare-and-set on every action.
- **Cost per action.** A stateless function reloads and re-persists `GameState` on every action, or
  re-folds the log. The actor holds it in memory and writes one append-only row.
- **Push.** After every change the server pushes `viewFor` to both sockets (SPEC §9.3, §10.8). That is
  a server-initiated write to a connection the function does not own.

### 4.1 Specifically: Supabase Edge Functions cannot host it

Edge Functions are request-scoped Deno isolates. They have no durable per-match identity, no alarm or
timer that survives the response, and no way to hold two WebSockets open for the life of a match with
in-memory state between them. There is no Supabase primitive equivalent to a Durable Object. So the
actor is the one piece of this system Supabase does not supply.

ARCHITECTURE-CCG §2.1 offers three options and this design takes the third:

| Option | Verdict here |
| --- | --- |
| Stateful actor per match (Durable Objects) | The reference design. Available if the deployment target becomes Cloudflare; the actor's clock, store and socket calls would be re-pointed (§5.2). |
| Stateless functions + external state + scheduler | Rejected: two storage round trips per action, plus a separate scheduled job for every clock. |
| **Long-running process hosting one actor object per live match** | **Taken.** One `jackioh-server` process, a registry of live actors by match id (`crates/server/src/actor/registry.rs`), `tokio::time` for the clocks, axum's WebSocket for the sockets. ARCHITECTURE-CCG: "Simple to reason about, but you're operating a server." |

What "operating a server" costs us, and the mitigation each cost already has in SPEC:

| Cost | Mitigation |
| --- | --- |
| A restart drops every live match | `(seed, log)` rebuild when the first socket for the match arrives, starting the grace of a seat that has not come back (R744) — SPEC §9.5: "A crashed actor rebuilds its state by folding `(seed, log)`". `app.live_matches()` is the reaper's query. |
| One process is one point of failure | The hard ceiling and the reaper (§9.5) guarantee no match and no player is stuck forever, whatever happens to the process. |
| Horizontal scale needs match affinity | Out of scope at this population (ARCHITECTURE-CCG §6.1: "single digits at 3am"). When it is needed, `matches.status` plus a claim column is the smallest change. |

### 4.2 The two deployment configs

`vercel.json` (client) and `render.yaml` (server) are the two halves §3's table describes. `vercel.json`
cannot hold a comment, and three things in them are not obvious:

- **The client build emits to a REPO-ROOT `dist`**, not `apps/web/dist`. A build that wrote the
  latter failed with `No Output Directory named "dist" found after the Build completed` — Vercel
  looked at the root and did not apply `outputDirectory`. Emitting to the root makes the vite
  preset's default and `outputDirectory` name the same folder, so which one wins stops mattering.
- **`vercel.json` rejects unknown keys.** The `"// name"` comment idiom this repo uses in
  `package.json` is fine for npm and pnpm and is a hard error here, which is why this note is in
  Markdown instead of beside the setting it explains.
- **Vercel deploys on the last push, and only when the push can change the site.** The Hobby plan
  allows 100 deployments a day and counts one for every push to every branch; `[skip ci]` stops
  GitHub Actions and does nothing here, and the night bot's `bot-state` commits plus its PR branches
  used the day up (every PR then showed a failed Vercel check: `api-deployments-free-per-day`). Two
  settings in `vercel.json` hold it down. `git.deploymentEnabled` creates no deployment at all for
  the branches machines push to (`bot-state`, `bot/**`, `claude/**`, `copilot/**`, `dependabot/**`,
  `patch/**`, `patches/**`, `polish/**`, `production` and `promote/**`, which
  `promote-production.yml` moves and opens pull requests from, `squishy-state`, `squishy/**`, `wt/**`); main is never listed. `ignoreCommand` runs
  `scripts/vercel-ignore.sh` for everything else: a commit message containing `[vercel]` builds on
  any branch (the flag for a preview); any other branch is cancelled; and main builds unless every
  file changed since the last commit Vercel built (`VERCEL_GIT_PREVIOUS_SHA`, and only when that
  commit is an ancestor of HEAD) is one the web bundle never reads: `bot/`, `.harness/`, `.squishy/`, `.github/`, `docs/`, `reviews/`,
  `e2e/`, `spec/`, `training/`, `scripts/` (but `scripts/build-wasm.sh`), `crates/server/` and `crates/tools/`
  (but their manifests, which cargo reads), `render.yaml`, the root docs, a crate's `tests/`, and the
  client's test files (`*.test.ts(x)`, `apps/web/src/test/`), `apps/web/scripts/` and its README. The
  bundle is `apps/web` plus the WASM module compiled from `crates/engine`, `crates/cards`, `crates/ai`
  and `crates/wasm`, and `crates/cards/src/scripts/` holds the card scripts, which are compiled in, so
  the list never names a crate's `src/`. Diffing against the last build, not the previous
  commit, means a commit whose build was cancelled or failed is never skipped past. Any doubt builds:
  no previous sha, a shallow clone without it, an empty diff, a path off the list. A cancelled build
  still counts against the 100, which is why the machine branches are switched off outright rather
  than left to the script. Vercel reads the config from the commit that is pushed, so a branch cut
  before this landed deploys until it merges main, and `bot-state` is an orphan branch with no
  `vercel.json` of its own: it needs a root `vercel.json` of `{ "git": { "deploymentEnabled": false } }`,
  which the bot's `ensure()` does not restore if it ever recreates the branch. The quota resets
  about 24 hours after the cap was hit; redeploy from the Vercel dashboard (or push a new commit)
  after it does. Vercel still builds main in parallel with CI, not after it: making it wait for every
  CI job would take a deploy hook called from `ci.yml` and `deploymentEnabled: { "main": false }`.
  `apps/web/src/net/deploy-routes.test.ts` holds the branch list in place and `vercel-ignore.test.ts`
  the script's decisions, over diffs in a throwaway repo.
- **`render.yaml` builds a Docker image and names no command.** `runtime: docker` builds
  `crates/server/Dockerfile` with the repository root as its context, because the server crate builds
  against the engine, cards and AI crates beside it: `rust:1.97-slim-bookworm` stages run `cargo
  build --release -p jackioh-server` after cargo-chef (pinned) has compiled its dependencies from the
  workspace's manifests alone, in a layer a change to the sources does not rebuild, and a
  `debian:bookworm-slim` stage holds the one binary with CA certificates and `tini`. The catalog, the
  patch history and every migration are compiled into the binary, so nothing at run time needs Node,
  pnpm or the repository. The image's own command, `jackioh-server release`, migrates, seeds the
  catalog and serves; `render.yaml` sets no build or start command, and
  `crates/server/tests/deploy/rehearse.sh` fails if it ever does. On Vercel,
  `vercel.json`'s install step installs rustup and the `wasm32-unknown-unknown` target before
  `pnpm install`, because the web build compiles the WASM module.

The SPA rewrite is not boilerplate either: `main.tsx` routes on `window.location.pathname`, so
`/decks`, `/play` and `/match/<id>` are real URLs a player can open or reload, and without it each
is a 404 on a cold load. `assets/` is excluded so a missing bundle stays a 404 rather than being
served `index.html` as JavaScript.

### 4.2 Where each clock lives

| Clock | Value | Held by | Also stored on `matches` | Why stored |
| --- | --- | --- | --- | --- |
| Turn clock | 75 s (`TURN_CLOCK_SECONDS`) | actor timer (`tokio::time`) | `turn_deadline_at` | so both clients render it and a rebuild restores it |
| Prompt clock | 30 s (`PROMPT_CLOCK_SECONDS`) | actor timer (`tokio::time`) | `prompt_deadline_at` | a trap prompt held by the non-active player pauses the turn clock (R79) |
| Mulligan clock | 45 s (`MULLIGAN_CLOCK_SECONDS`) | actor timer (`tokio::time`) | `prompt_deadline_at` (the prompt clock never runs at the same time) | both mulligans are open at once (R265), so one deadline covers both seats and both clients render it; on expiry every seat still owing is timed out (R268) |
| Disconnect grace | 60 s (`DISCONNECT_GRACE_SECONDS`) | actor timer (`tokio::time`) | `grace_deadline_at` | SPEC §9.5: "the grace countdown is stored on the match so both clients show it" |
| Match ceiling | 120 min (`MATCH_CEILING_MINUTES`) | actor + the DB reaper | `ceiling_at` | the reaper must be able to resolve a match whose actor died (§9.5) |

Expiry never mutates state directly. It submits an action — `timeout`, `disconnectExpired`,
`ceilingReached` — through the same `reduce` as a player's click (R79, BUILD M7-T2); the mulligan
clock's expiry is one `timeout` per seat still owing (R268). The engine stays pure; only the actor
knows what time it is.

---

## 5. How the actor calls the pure engine

SPEC §9.3: "`reduce(state, action, rng)` is pure: no I/O, no clock reads, no framework. Timestamps
arrive as action data." The engine's public surface is already what the actor needs, and
`crates/server/src/actor/engine.rs` is the one module that calls it:

```rust
use jackioh_engine::{begin_game, create_game, fold, hash_state, legal_actions, reduce, view_for};
jackioh_cards::register_all(); // the catalog and every card script, once per process
```

### 5.1 The loop

```
on socket open:
   verify the JWT -> profileId; check it is p1 or p2 of this match; attach the socket
   push viewFor(state, seat)

on action frame:
   1. rate-limit the socket                              (SPEC §9.8, MATCH_ACTIONS_PER_SECOND)
   2. if action.nonce was already applied -> return the original ack
                                                          (SPEC §9.3, BUILD M6-T4)
   3. stamp seat from the verified JWT; never trust action.playerId from the wire
   4. { state, events, error } = reduce(state, action)
   5. if error -> ack { ok: false, reason: error }; log the rejection with its reason (SPEC §9.8)
   6. append the action to match_actions via app.append_match_action  (SPEC §9.3)
   7. push viewFor(state, 'p1') to p1 and viewFor(state, 'p2') to p2  (SPEC §10.8)
   8. reset the clocks from the new state; if the game is over -> api::results::record_result
      (crates/server/src/api/results.rs) and close
```

Notes that matter:

- **Step 3 is the whole trust model in one line.** The seat comes from the verified token. An action
  claiming `playerId: "p2"` on p1's socket is rejected before it reaches `reduce`.
- **Step 4 does not need `rng` passed in.** Randomness is `(state.seed, state.rng_cursor)`, advanced
  inside the reducer (SPEC §10.7), so the actor never sources entropy. The pure crates cannot reach
  OS randomness or a clock at all (CLAUDE.md rule 4: their dependency lists and `clippy.toml`).
- **Step 6 appends after `reduce` succeeds.** The log is a log of *resolved* actions (SPEC §9.3:
  "Append-only action log per match"), so a fold of it never has to skip rejects.
- **Step 7 pushes two different objects.** `viewFor` is the filter; there is no "full state" frame and
  no debug mode that sends one.
- **Prompts are state, not callbacks** (SPEC §9.3). A choice mid-resolution sets `state.pending` and
  returns; the answer is another action through the same path. That is why a trap firing on the
  opponent's turn (BUILD e2e `03`) needs no special case in the actor: it is one more `reduce`.
- **Play-time choices travel in the `play` action** (R81), so zone, X, embiggen, Tribute, targets and
  modes do not pause resolution and do not round-trip.

### 5.2 Actor lifecycle

| Event | What the actor does |
| --- | --- |
| Room created | nothing yet; the row is `open` and no state exists |
| Second player joins | `createGame({ seed, decks })` then `beginGame`; status `live`; `started_at` and `ceiling_at` stamped |
| Both sockets attached | push `viewFor` to each; start the turn clock |
| One socket drops | start the grace timer, write `grace_deadline_at`, push the countdown to the other player; **the clock keeps running** (SPEC §9.5) |
| Reconnect inside grace | fresh `viewFor`, never a replay (SPEC §9.5); cancel the grace timer |
| Grace expires | submit `disconnectExpired` → a loss (R79) |
| Terminal state | `api::results::record_result` (`crates/server/src/api/results.rs`) writes one `results` row, applies the rating move when the match is ranked (R604), clears both `profiles.current_match_id` and any queued ticket — all in one transaction — then the actor is dropped from the map |
| Process boot | nothing: no actor is rebuilt until a socket for its match arrives, when `actor/registry.rs` folds `(seed, decks, log)`; the rebuilt clock restarts the turn clock from full and keeps the ceiling from `started_at` |
| First socket on an actor while the other seat has never attached to it (a restart, a no-show) | start that seat's grace at its stored `grace_deadline_at` if the restart left one (never later than a fresh window, at once if already past), else a fresh 60 s; it expires as a loss like any other. A match no socket returns to is the reaper's ceiling draw (R112, R744) |
| Idle | The process has no hibernation; an actor with no sockets and an expired grace has already ended. On Durable Objects this row would read "hibernate". |

The actor reaches the world in four places only: the clock (`tokio::time`), its timers, the store's
`matches_append_actions`, and its two sockets. A Durable Object port would replace those four, not the
actor's logic.

### 5.3 Why the API routes share the process

They do not have to. They are stateless and could be Edge Functions. They share the process because:

- They call `jackioh_engine::validator` and the engine, crates of the same workspace that one binary
  links, so the server and the actor always run the same rules.
- A deck's capped upsert, the redemption transaction, the ticket claim and a series' compare-and-set
  need multi-statement transactions over `DATABASE_URL`, which is a server-only credential either way.
- Matchmaking's opportunistic pairing on enqueue (SPEC §9.5) wants to hand the paired match straight
  to a local actor.

If they are ever split out, nothing in the trust model changes: they would still hold the secret key
and still be the only writers.

---

## 6. Why `(seed, log)` is the source of truth

SPEC §9.3: "Seeded RNG only… `(seed, log)` reconstructs any match." The in-memory `GameState` is a
cache of a fold, not the record.

A Conquest series (R330–R338, R262–R263) sits above its games and is not folded: `series` is a row of
its own, written by compare-and-set on `version`, and each player's sealed pick for the next game is
in it before the pick is acknowledged (R331). Each of its games is an ordinary match with its own
`(seed, log)`, started only once both decks are picked (R338); the series records which match each
game was and which decks have won, and a game's result and the series' record of it commit in one
transaction (`crates/server/src/api/results.rs`). A sweeper runs the pick clock and starts a game whose picks are
in but whose match a restart left unstarted. Conquest reads which decks have won off the games the
row already recorded, so it needed no migration (R330, R337).

What this buys, in the order it will be needed:

1. **Crash recovery.** SPEC §9.5. The process restarts; the first socket for a live match folds
   its log back and re-arms its clocks, starting the grace of a seat that has not come back (R744).
   No snapshots to keep consistent, no half-written state. BUILD M6-T4's acceptance is exactly this:
   "killing the actor mid-game and reconnecting yields the same `viewFor` for both players."
2. **Determinism as a test oracle.** `hash_state(fold(seed, decks, log))` computed twice must match.
   BUILD's e2e `01` compares the final hash from a browser game against `jackioh replay`'s fold of the
   recorded actions; the fuzz gate folds every seeded game back and compares.
3. **Dispute resolution and balance telemetry.** ARCHITECTURE-CCG §2.2. A finished match's log is
   purged after `MATCH_ACTION_RETENTION_DAYS` (migration 0013), so card win rates are not derived
   from it later: once a match's result is in, the server folds the log one more time through the
   engine's `summarize_game` and keeps what the card statistics need as a `game_records` row
   (SPEC §9.11, R376). The fold is the instrumentation; nothing is added to the state or the log.
4. **Replays later for free.** Out of scope (SPEC §9.6) but already paid for.

What the database therefore stores per match, and nothing more:

| Column | Role in the fold |
| --- | --- |
| `matches.seed` | the only entropy in the system |
| `matches.p1_deck`, `matches.p2_deck` | the frozen decklists — SPEC §9.5: decks are frozen into the ticket (or dealt, in All Random, R258), never read from a saved deck at match start |
| `matches.catalog_version` | which card definitions the fold must use |
| `match_actions (match_id, seq, action)` | the ordered log; `seq` is assigned under a row lock |
| `match_actions (match_id, nonce)` unique | server-side dedupe so a retrying client is safe (SPEC §9.3) |

One thing sits beside the fold and plays no part in it: the match's `game_records` row, written once
its result is in (R376), with the mode read off the series, the room or the tickets that made the
match. Nothing on a match's own path reads or writes that table, so a server deployed before
migration 0014 is applied loses records (each logged as `game.record.failed`), never a match.

No snapshots. ARCHITECTURE-CCG §2.2: "Append each action as it resolves; don't persist snapshots.
Periodic snapshots are an optimisation for later if replay gets slow." A 30-player-turn cap (R2) puts
a hard bound on log length, so that day is far off.

`match_actions` is append-only as a **database** property: `app.deny_row_mutation()` is attached
`before update or delete`. `collection_grants` gets the same trigger for the same reason.

---

## 7. The catalog

SPEC §9.4: "catalog is static, versioned, shipped with the client; stale catalog version is rejected
at save and queue."

| Copy | Lives in | Used for |
| --- | --- | --- |
| `crates/cards/catalog.json` | the repo, bundled into `apps/web` and compiled into the server and the WASM module | every card name, cost, stat and rules text the client renders |
| `crates/cards/src/scripts/*` | compiled into the engine wherever it runs: the server, and the browser's WASM module for hotseat and practice | what cards actually do |
| `public.cards` | Postgres | referential integrity for `collection`, and the server-side L6 check ("exists in the current catalog version and is not banned") |

`public.cards` is a projection, loaded by `jackioh-server seed-catalog`, which stamps
`catalog_version` on every row and writes the same value to `app.settings`. It is not a source of
truth for rules: the deck builder is fast because a collection read is a short list of
`(card_id, quantity)` and the card data is already in the bundle.

Version mismatch has one behaviour everywhere: `app.assert_catalog_version` raises `update required`,
the server maps that to a 409 with the same message, and the client prompts a reload. Checked when a
deck is saved (SPEC §9.4); at enqueue the chosen deck or trio is checked against the current catalog
by L6, whatever version it was saved under (R253).

---

## 8. Env-var contract

Two disjoint halves. The rule is mechanical: **anything the browser needs is prefixed `VITE_` and is
public by construction; anything without that prefix must never appear in a client bundle.**

### 8.1 Public — shipped to the browser

| Variable | Value | Where it comes from |
| --- | --- | --- |
| `VITE_SUPABASE_URL` | `https://<ref>.supabase.co` | Dashboard → Project Settings → Data API → Project URL |
| `VITE_SUPABASE_PUBLISHABLE_KEY` | `sb_publishable_…` | Dashboard → Project Settings → API Keys → Publishable key |
| `VITE_SERVER_HTTP_URL` | `https://api.example.com` | wherever `jackioh-server` is deployed |
| `VITE_SERVER_WS_URL` | `wss://api.example.com/ws` | same host, WebSocket path |
| `VITE_AUTH_OAUTH_PROVIDERS` | optional, e.g. `google,github` | R666: the OAuth providers the sign-in screen offers, by Supabase's names (apple, azure, discord, facebook, github, gitlab, google, twitch). Names only; unset offers none. Set it only once each named provider is enabled in the dashboard (§10, step 2) |

The client's catalog version is not a variable: the bundle reads it from
`crates/cards/patches/patches.json` when it is built, as the server compiles it in.

A publishable key is safe in a browser **because RLS is the access control**, not because the key is
secret. It maps to the `anon` role before login and `authenticated` after, and §3.2's matrix is the
whole of what it can reach. Legacy `anon` JWT keys still work and are compatibility only.

### 8.2 Server-only — never in a client bundle, never in git

| Variable | Required | Purpose | Where it comes from |
| --- | --- | --- | --- |
| `SUPABASE_URL` | yes | project URL for admin auth calls and the JWKS default | Dashboard → Project Settings → Data API |
| `SUPABASE_SECRET_KEY` | yes | `sb_secret_…`; maps to `service_role` and **bypasses every RLS policy** | Dashboard → Project Settings → API Keys → Secret key |
| `DATABASE_URL` | yes | direct Postgres; the transactions the Data API cannot express (§9.4 redemption, `saveLoadout`, §9.5 ticket claim) | Dashboard → Project Settings → Database → Connection string → URI |
| `SUPABASE_JWKS_URL` | no | defaults to `${SUPABASE_URL}/auth/v1/.well-known/jwks.json`; verifies browser JWTs (RS256/ES256) | derived |
| `SUPABASE_JWT_SECRET` | no | legacy HS256 fallback; discouraged | Dashboard → Project Settings → JWT Keys |
| `CODE_PEPPER` | yes, ≥32 chars | HMAC pepper for `invite_codes.code_hash` and `code_attempts.ip_hash` | `openssl rand -base64 48`; rotating it invalidates unredeemed codes |
| `PORT` | no (8787) | listen port | — |
| `PUBLIC_ORIGINS` | yes | comma-separated allowed origins for CORS and the WebSocket `Origin` check | your web host |
| `NODE_ENV` | no (`development`) | `development \| test \| production` | — |
| `CATALOG_VERSION` | no | the catalog version this server accepts (§9.4) | the server always serves and stamps the version compiled into the binary, the newest entry of `crates/cards/patches/patches.json` (`cargo jackioh catalog-version` prints it); `release` stamps it on `public.cards`. `render.yaml` carries it and `patches ship` bumps it there; a stale dashboard value only logs a warning, so a card patch never needs a hand edit in Render (#488) |
| `E2E` | no (`0`) | BUILD M8's test-server mode; refused when `NODE_ENV=production` | — |
| `TRUSTED_PROXY_HOPS` | no (`0`) | R190: how many `X-Forwarded-For` entries, counted from the right, this deployment's own proxies write. The per-IP limits (§9.4 step 3, R157) key on that entry and ignore everything the caller wrote to its left; `CF-Connecting-IP` and `X-Real-IP` are never read. `0` to `5`; `0` (the default) ignores the header and keys on the socket's peer address, which is right with no proxy in front | `render.yaml` starts it at `1`, which can only over-group; set it to the count your own request shows in the `api.forwarded_for` log (§10, step 8); leave unset for a local server |
| `RENDER_GIT_COMMIT` | no | the git commit this deploy was built from; Render sets it on every deploy and it is unset anywhere else. `GET /api/catalog` reports it in the `x-deployed-commit` response header (the body is untouched, R163), and `deploy-watch.yml` compares it with each push to `main`, so a Render that stopped receiving pushes is seen even when the catalog version did not change. Only a hex SHA is kept | set by Render; leave unset for a local server |

`crates/server/src/env.rs` loads these from the process environment (the binary reads no `.env`
file), reports **every** missing or malformed variable in one error naming where to get each, and
never logs a secret value — not even truncated. It exports `PUBLIC_ENV_VARS` and
`SERVER_ONLY_ENV_VARS` so a test can assert the halves never cross. Template:
`crates/server/.env.example`. Under `E2E=1` the Supabase, database, pepper, origin and catalog
variables fall back to fixture placeholders, so `E2E=1 jackioh-server` boots with nothing else set.
Nothing in the image needs Node: `NODE_ENV` keeps its name only because the server still reads it,
to refuse `E2E` in production.

Two things that are **not** env vars, deliberately:

- **R79's lifecycle values and §9.12's ranked numbers** — turn clock, prompt clock, grace,
  ceiling, room-code length, the Glicko-2 start and the ladder's tier and season constants
  (R603–R609). They are gameplay, so they are named constants in `crates/server/src/config.rs` and
  change only with a code change and a review. BUILD §2 requires exactly that.
- **The code pepper in Postgres.** Hashing happens in the server, so the pepper never reaches the
  database and `invite_codes` only ever holds `code_hash`. A database dump therefore does not yield a
  single redeemable code.

---

## 9. Local versus hosted

The same four SQL files, the same server, two connection strings.

| | Local | Hosted |
| --- | --- | --- |
| Postgres + Auth + Data API | `supabase start` (Docker) | the Supabase project |
| API URL | `http://127.0.0.1:54321` | `https://<ref>.supabase.co` |
| `DATABASE_URL` | `postgresql://postgres:postgres@127.0.0.1:54322/postgres` | Project Settings → Database → URI |
| Studio | `http://127.0.0.1:54323` | the dashboard |
| Outgoing email | captured locally at `http://127.0.0.1:54324`, nothing is actually sent | your SMTP provider; configure it before inviting anyone, because §9.4 requires a verified email before redemption |
| Keys | printed by `supabase start` | Project Settings → API Keys |
| `jackioh-server` | `cargo run --release -p jackioh-server`, with the environment exported | the Docker image on Render, behind TLS |
| `apps/web` | `pnpm --dir apps/web dev` (Vite, :5173) | static bundle on a CDN |

Notes:

- **The email step is the one real local/hosted difference.** SPEC §9.4 requires a verified email
  before redemption, which means a hosted project needs working SMTP before the first invite goes
  out. Locally, click the link in the captured mail UI. `[auth.email] enable_confirmations` in
  `supabase/config.toml` controls it; leave it **on**, because turning it off locally makes the gate's
  step 1 untestable.
- **Connection modes.** `migrate` takes its advisory lock with `pg_advisory_xact_lock` inside
  each migration's transaction, never as a session lock, so it runs over either a **session-mode**
  connection (the direct `:5432` URI, or Supavisor's session port) or a transaction-mode pooler.
  That matters because the image's `release` (migrate, then seed the catalog) runs before every
  boot on Render over the server's own `DATABASE_URL`, and a session lock taken
  through a transaction-mode pooler stays held on a pooled backend, where the next deploy's runner
  could wait on it forever. The runtime server is fine on either; transaction-mode pooling is the
  cheaper default for it.
- **Migrations run as a role that is not a superuser.** On Supabase the role `DATABASE_URL`
  names is not a superuser, so a migration must not need one: no `set` clause on a function for a
  custom parameter (Postgres 15+ refuses it; set it with `set_config()` in the body, as 0013 does),
  no `alter system`, no extension only a superuser may create. `crates/server/tests/deploy/rehearse.sh`
  applies every migration as such a role; the sql and db suites migrate as a superuser and cannot tell.
- **Deploys bring the database along.** Steps 4 and 6 of the checklist below run on every Render
  boot, so a deploy that ships new migrations or a new card patch applies them and reseeds at its
  `CATALOG_VERSION` before the server listens. Both are idempotent, and a failure keeps the new
  instance from passing its health check, so the previous deploy keeps serving. A deploy that bumps
  the game's **minor** version also opens a new ranked season (R609): the first boot opens it and
  runs the soft reset itself, so the operator's part is only the rehearsal —
  `jackioh-server season-start --dry-run` against a copy of the live data beforehand, which prints
  the report and rolls back (`crates/server/README.md`).
- **Exposed schemas.** Confirm `app` is not in the Data API's exposed schema list — `[api] schemas`
  in `supabase/config.toml` locally, Project Settings → Data API in the dashboard. The default
  (`public`, `graphql_public`) is correct. If `app` is ever exposed, every `SECURITY DEFINER` function
  in it becomes an HTTP endpoint.
- **Supabase CLI users.** The canonical SQL is at `crates/server/migrations/` per BUILD §1. The
  CLI only reads `supabase/migrations/<timestamp>_<name>.sql`, so if you want `supabase db push` and
  `supabase db reset`, add `supabase/migrations/` entries that are symlinks or `\i` includes pointing
  at the canonical files, and keep the ordering identical. The bring-up checklist below uses the
  repo's own runner instead, so this is optional.

---

## 10. Bring-up checklist

From nothing to two browsers in a room-code match. Steps 1–8 are the "hand over the keys" path; a
step that is not yet implemented says which BUILD task delivers it.

1. **Create the Supabase project** (or run `supabase start` for local). Note the project URL, the
   publishable key and the secret key. Confirm Project Settings → Data API exposes `public` and
   `graphql_public` only.
2. **Enable email/password auth with confirmations.** Auth → Providers → Email: enabled,
   "Confirm email" on. Hosted: configure SMTP now. SPEC §9.4 makes a verified email a precondition of
   redemption, so this is not optional.
   Then Auth → URL Configuration, which decides where every emailed link lands:
   - **Site URL:** `https://jackioh.vercel.app`. Left at its default (`http://localhost:3000`), every
     confirmation and reset link pointed at the reader's own machine.
   - **Redirect URLs:** `https://jackioh.vercel.app/**` and `http://localhost:5173/**`. The client
     asks for `<origin>/login` on every mailer (`authRedirectUrl`, B35), and the provider honours it
     only when it matches this allow-list, falling back to the Site URL otherwise.
   The links are PKCE links (R323): the mailers send a `code_challenge`, the link comes back to
   `/login?code=…`, and the browser that asked exchanges the code for a session with the verifier
   it kept. A link opened on another device confirms the address and asks for a sign-in (R324).
   No Supabase setting needs changing for PKCE.

   The further ways in (issue #267, R664–R666) need these dashboard steps, done by a person:
   - **Email sign-in link and code (R664).** Auth → Emails → Templates → **Magic Link**: the
     template must carry both `{{ .ConfirmationURL }}` (the link) and `{{ .Token }}` (the code),
     since the screen offers both; the default template has the link only. For example:
     `<p>Sign in to JackiOh: <a href="{{ .ConfirmationURL }}">open this link in the browser you asked from</a>, or type this code: <strong>{{ .Token }}</strong></p><p>If you didn't ask to sign in, ignore this email.</p>`.
     Auth → Providers → Email: keep **Confirm email** on, and leave the email OTP expiry at its
     default (3600 s) or shorter. Nothing else changes: the link returns to `<origin>/login`, which
     the Redirect URLs above already allow, and it is a PKCE link (R323).
   - **Two-step sign-in (R665).** Auth → Multi-Factor: **TOTP (App Authenticator)** enabled (it is
     on by default on a new project). Leave phone MFA off: the client offers only an authenticator
     app, and the server's `aal2` rule counts only a verified TOTP factor.
   - **OAuth providers (R666),** for each provider wanted: create an OAuth app at the provider
     (Google Cloud console, GitHub developer settings, Discord developer portal, …) whose
     authorised redirect URI is `https://<ref>.supabase.co/auth/v1/callback`; then Auth → Sign In /
     Providers → the provider: enabled, its client id and secret pasted in (they stay in Supabase,
     never in this repository or a `VITE_` variable). Then add the provider's name to
     `VITE_AUTH_OAUTH_PROVIDERS` in `apps/web/.env.production` (and the Vercel project's variables
     for staging) and let the web app rebuild. The provider returns to `<origin>/login`, already in
     the Redirect URLs. Keep Supabase's default **automatic linking** (an OAuth sign-in whose
     *verified* address matches an existing account signs into that account), and enable only
     providers that report whether an address is verified (all eight the client names do). Leave
     "Allow manual linking" off: nothing here uses it.
     Before the first provider goes live, check one thing by hand on the project: sign up with a
     password for an address you own and do not confirm it, then sign in with the provider for the
     same address, then try the password. If the password still signs in, Supabase kept an
     unconfirmed account's password through linking (a pre-account takeover); do not enable
     providers until that is resolved.
3. **Fill the server environment.** `cp crates/server/.env.example crates/server/.env` and set
   `SUPABASE_URL`, `SUPABASE_SECRET_KEY`, `DATABASE_URL`, `CODE_PEPPER` (`openssl rand -base64 48`),
   `PUBLIC_ORIGINS` and `CATALOG_VERSION` (`cargo jackioh catalog-version`). The binary reads only its
   environment, so export the file into the shell that runs it: `set -a; . crates/server/.env; set +a`.
   Then `cargo build --release -p jackioh-server`; the steps below run `target/release/jackioh-server`.
4. **Apply the migrations.** `target/release/jackioh-server migrate`, which applies every file in
   `crates/server/migrations/` (compiled into the binary) in order — `0001_profiles_and_invites.sql` → `0002_collection.sql`
   → `0003_loadouts.sql` → `0004_matches.sql` → `0005` → `0006` → `0007_decks_and_trios.sql` →
   `0008_queue_modes.sql` → `0009_series.sql` → `0010_jlockeed_tag.sql` →
   `0011_tutorial_progress.sql` → `0012_account_deletion.sql` → `0013_retention_purge.sql` →
   `0014_game_records.sql` → `0015_classic_sets_tags.sql` → `0016_catalog_growth_grants.sql` →
   `0017_last_boards.sql` → `0018_player_settings.sql` → `0019_hero_portraits.sql` →
   `0020_plague_tag.sql` → `0021_player_stats.sql` → `0022_ranked_ladder.sql` →
   `0023_rematch.sql` → `0024_glitch_boards.sql` → `0025_patch_retcon.sql` →
   `0026_catalyst_prime_acclaimed_tags.sql` → `0027_lean_newest.sql` → `0028_usernames.sql` — and records them in `app.migrations`. Expected
   result: 25 tables
   in `public`, all with RLS enabled, plus the private `app` schema. On a project that already had
   loadouts, 0007 turns each into three saved decks and a trio named "My trio" (R254) and leaves the
   loadout tables where they are. 0010 only widens the `cards` tag check, so `seed-catalog` can
   write #13 and #14's Jlockeed tag (R278). 0011 adds `tutorial_progress` and its one write path,
   `app.merge_tutorial_progress` (R320); it needs nothing else from the bring-up. 0014 adds
   the server-only `game_records` (R376); `stats-cards` reads it and
   `stats-import` loads an AI development run into it (R377, R378). Patch v0.2.0 adds three: 0015
   widens the `cards` tag check with Book, Pancake and AI (B2.4); 0016 grants every active account
   the cards a new catalog version adds when `seed-catalog` stamps it (R481); and 0017 adds the
   server-only `last_boards` and each match's starting boards for C+ #29 Portal to the Past (R417,
   R565). 0018 adds `player_settings` and its one write path, `app.merge_player_settings` (R633,
   R634), which keeps a player's game settings on the account; like 0011 it needs nothing else from
   the bring-up. 0022 is the ranked ladder (R603–R612): Glicko ratings go fractional and gain the
   deviation and volatility columns, `matches` and `series` gain their `ranked` flag, and the four
   server-only tables land — `seasons`, `season_ranks`, `bot_ratings`, `rated_games`. On a project
   that already has players it also narrows the client grants on `profiles`, `tickets` and
   `results` to column whitelists, because the hidden rating may never reach the client (R612);
   nothing else in the bring-up changes. 0023 adds `matches.mode` and `matches.stake` for
   rematches (R672): only a rematch writes them, older rows keep deriving their mode. 0025 renames
   every row a database filed under the card patches' old names to their new ones (R743); on a new
   project it changes nothing. 0026 only widens the `cards` tag check again, with patch v0.2.Y's
   Catalyst, Prime and Acclaimed, as 0020 did with Plague. 0027 adds `tickets.lean_newest` and
   `matches.room_lean_newest`, where All Random's "More cards from the newest set" waits for the deal
   (R1372); both default to false, so nothing is backfilled. 0028 gives every profile a username
   (R1432–R1436): it replaces `profiles.display_name` with the base name, its key, its tag, the time
   of the last change and whether the prompt was answered, names every existing profile `Player#n`
   in order of sign-up, and names every new one the lowest free `Player#n` by trigger; a client
   reads its own row's username columns and writes none of them. It only adds: `display_name`,
   which no server reads from 0028 on, stays for the deploy before it, which reads it until the new
   one serves, and a later migration drops it.
5. **Verify the invariants before trusting anything.** `sh crates/server/tests/sql/run.sh` runs all of
   §12's checks against a throwaway Docker Postgres, which is the fast way to confirm the migrations
   are intact before you point them at a real project. Against the project itself, in Studio's SQL
   editor:
   - `select relname from pg_class c join pg_namespace n on n.oid = c.relnamespace where n.nspname = 'public' and c.relkind = 'r' and not c.relrowsecurity;` → **must return zero rows.**
   - `select count(*) from public.decks;` → three per loadout that existed before 0007 (R254).
   - `insert into public.collection …` as an `authenticated` user → must be refused. There is no
     policy, so there is no path (§9.4).
6. **Seed the catalog.** `target/release/jackioh-server seed-catalog`. It writes the catalog compiled
   into the binary, `crates/cards/catalog.json` (BUILD M4-T1, M9-T1). Check `select count(*) from public.cards;` → 318
   (268 cards + 50 tokens over Core, Classic and Classic+, patch v0.2.0 and Glitch, issue #170) and
   `select app.catalog_version();` → your `CATALOG_VERSION`, the latest card patch's version (R388).
7. **Mint an invite code.** `target/release/jackioh-server mint-code`. It generates 16 characters
   from `CODE_ALPHABET`, formats them `XXXX-XXXX-XXXX-XXXX`, HMACs with `CODE_PEPPER` and inserts
   only the hash (§9.4). The plaintext goes to stdout **once** — the database cannot give it back —
   and the metadata to stderr, so `jackioh-server mint-code > code.txt` captures the code alone. `--max-uses=N`
   (default 1, R161) and `--expires-in-days=N` (default never) are the options; for two accounts on
   one code, `--max-uses=2`. Check `select count(*) from public.invite_codes;` → 1.
   `mint-code` (`crates/server/src/cli/mint_code.rs`) validates the whole environment through
   `load_env` rather than the two variables it reads, because a `CODE_PEPPER` that differs from the server's mints a well-formed
   code that nobody can ever redeem.
8. **Start the server and the client.** `target/release/jackioh-server` and
   `pnpm --dir apps/web dev`. The server must refuse to start with a missing env var, naming it.
   Behind a proxy (Render), calibrate `TRUSTED_PROXY_HOPS` (R190) from the `api.forwarded_for` log
   lines. The server logs one each time a request carries fewer `X-Forwarded-For` entries than any
   before it (`fewestEntries`, next to the configured `trustedProxyHops`, and never an address). A
   caller can add entries on the left but can never remove the ones the deployment's proxies
   append, so the lowest count is the number of hops those proxies add. So:
   1. Right after a deploy, send a request of your own that carries no `X-Forwarded-For`
      (`curl https://<service>/api/catalog`) and read the `fewestEntries` its line shows.
   2. Set `TRUSTED_PROXY_HOPS` to exactly that count, and redeploy. Render's edge alone gives 1; a
      CDN in front of it (Cloudflare) adds its own entry, which gives 2. The blueprint's 1 is only
      the safe starting point: with 2 hops and 1 trusted, every caller behind one CDN edge shares one
      per-IP bucket, so §9.4 step 3's limit could refuse strangers.
   3. Keep an eye on later lines. A `fewestEntries` below the configured count means a path with
      fewer proxies (lower the setting to it). A higher count on its own proves nothing: a caller can
      write extra entries, so never set the hops above the lowest count any request has shown, since
      trusting a caller's entries hands the per-IP key back to the caller.
9. **Sign up two accounts** (BUILD M6-T1). Each gets a `profiles` row at `pending` from the
   `auth.users` trigger. Verify both emails. Confirm a pending account gets 403 from
   `/api/collection`, `/api/decks` and `/api/queue`.
10. **Redeem a code on each** (BUILD M6-T1). `status` flips to `active`, the activation trigger fires
    the launch grant, and `select count(*) from public.collection;` shows every non-token card for
    both profiles (§9.1: "Everyone owns every card at launch; keep the ledger anyway"). Confirm the
    three failure kinds — missing, expired, exhausted — return the identical message.
11. **Build a deck on each** in `/decks` (R250). It saves as you go, whether or not it is finished;
    `/play` shows whether it can be queued and, if not, the validator's reason, naming the deck and
    the card. `jackioh-server seed-accounts` gives its accounts three starter decks and a trio instead.
12. **Create a room** (BUILD M6-T4). `POST /api/rooms { mode: "bo1", deckId }` → a 6-character code
    from `CODE_ALPHABET` (R79). A `matches` row appears at `status = 'open'` with p1's frozen deck.
13. **Join it from the second browser.** `POST /api/rooms/:code/join { mode: "bo1", deckId }` claims
    the open row atomically, sets p2 and makes it `live`. A Conquest room makes a series instead,
    and both players pick their first deck on `/series/:id` (R331, R264).
14. **Play.** Both sockets connect with their JWTs, the actor calls `createGame` and `beginGame`, and
    each player gets their own `viewFor`. Every action appends one `match_actions` row and pushes two
    views. **This is the milestone: a working room-code match.**
15. **Prove the log is the truth.** Kill the server mid-match and restart it: both players reconnect
    to the same `viewFor`, rebuilt by folding `(seed, log)` (BUILD M6-T4 acceptance).
16. **Finish the match** and confirm one `results` row, both ratings recorded unchanged (a
    room-code match is unranked, R604; a queue-paired one would show them moved by the Glicko-2
    update, R603), both `profiles.current_match_id` cleared and both players queue-eligible
    again (BUILD M7-T2).

---

## 11. Decisions this document made that SPEC §9 does not state

Moved to docs/history/architecture.md.

---

## 12. Verifying the schema

Moved to docs/history/architecture.md.

## 13. File map

Moved to docs/history/architecture.md.
