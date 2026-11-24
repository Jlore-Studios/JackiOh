# The six usual causes of a slow app, cost first (#4691)

Measured 2026-11-24 for #4691, which asked:
- to check JackiOh against the six usual causes of a slow app;
- to fix the ones that apply, choosing the cheaper fix whenever speed and cost pull apart;
- to make nothing slower.

The fixes are its six parts, #4709 to #4714. Each section below gives what was measured, what changed and the test that holds it, or why the cause does not apply.

## What the bill is made of

Rates are from Cloudflare's [Containers pricing](https://developers.cloudflare.com/containers/platform/pricing/) and [Workers pricing](https://developers.cloudflare.com/workers/platform/pricing/) pages, read 2026-11-24.

| What | Rate | Included in Workers Paid ($5/month) | Billed on |
|---|---|---|---|
| Container memory | $0.0000025 per GiB-second | 25 GiB-hours a month | provisioned size, while the container runs |
| Container disk | $0.00000007 per GB-second | 200 GB-hours a month | provisioned size, while the container runs |
| Container CPU | $0.000020 per vCPU-second | 375 vCPU-minutes a month | active use only |

- Each site runs one `basic` container (1 GiB, 1/4 vCPU, 4 GB; `wrangler.jsonc`). It costs about **$0.0100 per awake hour** in memory and disk once the included hours are spent, and the two sites share them.
- Worker requests are $0.30 per million and Durable Object requests $0.15 per million. Static files run no Worker code (`assets.run_worker_first`).

So what matters most is every request that **wakes the container or keeps it awake** when nothing needs it. It sleeps after `SLEEP_AFTER`, five minutes (`apps/edge/src/index.ts`; the architecture doc said ten until #4709).

## 4. Slow work in a request, and what keeps the container awake

### What woke it, and what held it

| Source | Before | After |
|---|---|---|
| Daily cron `23 14 * * *` (`/readyz`) | 1 wake a day per site | unchanged (CLAUDE.md rule 13) |
| Hourly cron `7 * * * *` (`/daily-sweep`, #3727) | 24 wakes a day per site, whatever was due | a sleeping server is woken only in an hour when a sweep has something due ([R4691000](../spec/rulings/R4691000.md), #4709) |
| A hidden signed-in tab | quiet after `HIDDEN_QUIET_MS` (2 min, R3576000) | unchanged |
| A shown signed-in tab nobody touches | held the container awake for as long as it stayed open: the open `/ws/social`, a presence beat every 45 s, every `pollWhileVisible` read | quiet after `IDLE_QUIET_MS` (10 min) with no input, as a hidden tab is ([R4691001](../spec/rulings/R4691001.md), #4710) |
| A live match, a watch, a co-op room | sockets open, awake | unchanged: they must be |

### Wakes before and after, on a quiet site

| Case | Before | After |
|---|---|---|
| One idle, shown, signed-in tab | the container awake all the time the tab stays open, about 24 hours a day if it is left overnight; about 80 beats an hour | awake `IDLE_QUIET_MS` + `SLEEP_AFTER` (15 min) after the last input, then asleep; nothing sent while idle |
| After a finished match, nobody online | 24 hourly wakes and 1 daily wake: about 25 × (boot + 5 min), roughly 2.2 hours awake a day per site | 1 daily wake, plus one for each hour a daily game's deadline, reminder, lapse or feed purge comes due |

The second row is #4691's own case:
- With no daily games running, a site sleeps from its last visitor until the daily `/readyz`.
- The runbook (`docs/deploy-cloudflare.md` §6) had priced the hourly cron at about a sixth of a quiet site's hours, up to a dollar or so a month for each container. That saving is nearly all of a quiet site's usage bill past the plan.
- One forgotten shown tab used to cost a whole container-day, about $0.24 at the rate above.

### How the sweep skip works

- Everything the daily sweep does depends on stored times:
  - a running game's deadline (R3727000);
  - a waiting account's next reminder or held email (`decide_notify`);
  - the next invite lapse (R4390000);
  - the oldest feed row's `NOTIFICATIONS_KEEP_DAYS` (R4039000).
- Only a request changes any of them, and a request only runs on an awake server.
- So the server answers `GET /daily-next` with the first of those times (`jackioh_shared::notify::next_notify_at` for the notifications).
- The Worker's `Server` object asks that as `SLEEP_AFTER` runs out (`onActivityExpired`) and keeps the answer. It forgets it whenever the container starts (`onStart`), and the hourly cron's `sweepDaily` leaves a sleeping server asleep until the kept time comes.
- A server stopped any other way, a kept time that has come, or a storage fault: the cron sweeps as before.
- Every loss, notification and purge lands at the same cron it did before.

Held by:
- `r4691000_*` in `crates/shared/tests/daily.rs` and `crates/server/tests/api/daily.rs`;
- `R4691000 …` in `apps/edge/src/index.test.ts`;
- the store contract's next-lapse and oldest-row cases.

### Deferrable work on request paths: checked, not moved

- **Match end** (ratings, records, quests, rewards). It is R263's one transaction, on the actor's own task once the final view is out. Deferring it to a job or a queue would save no container time (the work is the same and the container is awake for the match anyway) and would risk `(seed, log)` recovery (§6, R1437). Part 3 trimmed it instead (cause 1 below).
- **The ladder bot's search.** It already runs in `spawn_blocking`, off the actor's task (`spawn_bot`).
- **The series sweeper and the other loops** (matchmaker, reaper, challenges, co-op, retention, daily, bot pool, db heartbeat). They run only while something else keeps the container awake, and none of them wakes it.
- **The boot.** `release` migrates and seeds the catalog, and the seed writes no card on an unchanged wake (#2926). Fewer wakes make it rarer, and it was not changed.

## 1. Too many queries

### What one ranked match costs

Counted with the fake store's `on_call` hook: one ranked match, random policy on both seats, 41 actions.

| Step | Store calls |
|---|---|
| Start | 16 (the stats totals among them, cached per process after the first match) |
| Both seats attach | 4 |
| An action | 1 (the append) for 27 actions, 2 (append and clocks) for 12, 3 for 1 |
| The end (the last action) | 71 before, 65 after (#4711) |

- In Postgres a one-statement transaction is three round trips. `BEGIN` and the session statement already share one (#3208, pinned by `tests/store/round_trips.rs`).
- **The end's fix, #4711.** The actor re-read the result, the match, the series, both profiles and each person's day totals in a second transaction. All of that recomputed whether the daily cap held pay back, which the write's payout already carries (`GameEconomy::jlore_capped`). A fresh write's caps now come off its payout. A repeat, a rebuilt actor or an economy that is off still reads them.
  - Held by `a_fresh_results_caps_come_off_its_payout` in `tests/actor/match_actor.rs`: one day-totals read per person (was two) and no series read (was one).
- **Not changed: the payout's grants.** They are still one statement each (`economy.wallets.grant` ×7 and `economy.inventory.grant` ×4 at a ranked end). R3212000's batch covers a pack's kinds only. Extending it to a game's Jlore, track, fleet and quest grants would save about ten round trips, but means reworking the ledger's refusal order. Players see those rewards after the final view, so it was not worth the risk here.
- **The browser's direct Supabase reads.** The client calls Supabase only to sign in, through its Auth API (`apps/web/src/net/auth.ts`). Every game read goes through the server, so there is no browser PostgREST traffic to count.

## 3. Huge responses

### Match frames

- Measured in the match above: 183 frames to the two seats over 41 actions, 784 KB in all, 4.3 KB on average and 12.8 KB at most.
- Each frame is a whole `view_for`, held to `VIEW_FOR_BYTES_BUDGET` (15 KB) per pull request by `cargo jackioh perf --smoke` (#4196).
- Not changed:
  - A diff, or `permessage-deflate` (axum's tungstenite does not negotiate it), would spend container CPU, which is billed, to save bandwidth, which is not: Workers bill no bandwidth and container egress has 1 TB included.
  - A whole view is what lets a reconnect, the forecast and the client's renders stay simple (rule 7).

### HTTP responses

- The server compresses nothing itself.
- Cloudflare's zone compresses responses for clients that accept it ([content compression](https://developers.cloudflare.com/speed/optimization/content/compression/)). Whether that covers the responses the Worker hands back from the container was not measured: this environment's network policy blocks the site.

### The versioned catalog, #4713

- `GET /api/catalog/:version` serves a shipped patch's immutable snapshot (R388), about 0.5 MB, yet answered `no-store`, so every read woke the container.
- No screen reads it today (the client bundles its catalogs, #1378), so the saving is in what tools and crawlers wake.
- It now answers `public, max-age` of a day ([R4691002](../spec/rulings/R4691002.md)), and the Worker answers a repeat read from the data centre's cache, never reaching the container.
- `/api/catalog`, whose `x-deployed-commit` the deploys' live check polls, stays `no-store` and always reaches the container (rule 13). So does every other `/api/*` read (#4011).
- Held by:
  - the server's `r4691002_*` tests, which hold the snapshot to the same bytes whatever the request carries, #4011's rule;
  - `R4691002 …` in `apps/edge/src/routing.test.ts` and `index.test.ts`;
  - `config.test.ts`'s narrowed #4011 guard.

### Collection, decks, stats and replay shares

- They are read once per screen visit.
- The shared ones (card and player stats, play rates) already answer `public, max-age` and are cached in the process.
- Not changed.

## 2. One slow query

How it was measured:
- `EXPLAIN (ANALYZE, BUFFERS)` on postgres:16 with `tests/db/bootstrap.sql` and every migration.
- Synthetic data: 20k profiles, 300k finished matches (one player on 11k), 1.85M actions, an 18k-row ladder, 154k chat messages, 31.5k daily games, 440k tickets.
- Times are warm.

Fixed in #4712:

| Query | Before | After | How |
|---|---|---|---|
| `replays_list_with_modes` (`GET /api/replays`, `GET /api/friends/recent`) | the 11k-game player's page: about 165 ms round trip (145 ms executing), its sort spilled to disk; recent opponents 140 ms | about 2 ms round trip (0.35 ms executing); recent opponents 0.85 ms; a typical player's 2 ms becomes 0.3 ms | partial indexes per seat on `(profile, ended_at desc, id desc)` where `status = 'over'`; the page is found one arm per seat, then only its rows are joined, into a narrower `ReplayListRow` without the unused jsonb columns |
| the reaper's live-match read (every 30 s) | about 150 ms round trip and 13.3 MB a pass at 400 live matches, every live match's whole row (win rates included) | about 2 ms: `matches_past_ceiling` reads only the matches past their ceiling, through `matches_status_ceiling_at_idx` | its own store method; `matches_live` is kept but no server code calls it |
| `results_record_for` (`GET /api/auth/me`) | 7.6 ms for 11k games, linear in games | 1.9 ms | covering indexes per seat (the p2 one also carries `p1_profile_id`, so a row seating the player twice still counts once), a sum of two index-only counts |
| `last_boards_sample_others` (every match start) | about 4.3 ms round trip (`order by random()` over the table) | about 0.35 ms | a partial index and a random-key seek, topped up from the start |

The table's before and after come from one run, on the same data with a warm cache. Old and new queries returned identical answers for 300 players across 5 pages (1,495 comparisons) and for all 20,000 players' records. The new migration is `0101_slow_read_indexes.sql`. Held by:
- the SQL suite's CHECK 29, which checks each index's shape and plans each query with sequential scans off;
- the store contract's cases for each rewritten read, on both stores.

Measured and fine:
- the ladder's subset and rank reads;
- recent opponents;
- player stats;
- friends and requests;
- every presence read;
- open challenges;
- chat unread;
- the notification feed's page, unread count and trim;
- `daily_games_of`, `daily_games_due` and `daily_games_running`;
- the queue's ticket reads;
- active series;
- the daily invites' expiry;
- the feed purge;
- the co-op live reads;
- the per-action append (0.23 ms, one round trip).

Not changed:
- **Player search with a rare term.** 46–94 ms at 18k public rows, because `position()` cannot use an index. Sub-millisecond at today's population. The fix needs the `pg_trgm` extension and a trigram index.
- **A long chat conversation's page.** 6.6 ms for a 4,000-message pair; an index on `(from_profile, to_profile, created_at desc, id desc)` and a UNION ALL take the first page to 0.24 ms.
- **The leaderboard.** 115–145 ms for 18k placed players. Both reads are index-backed, but R612 returns every placed player and the body is the same for every viewer, so caching or paging it is a spec decision (`needs decision`).

## 6. A slow page

Measured on the production build (`pnpm --dir apps/web build`).

**Already fine:**
- The catalog, the scene data and every route chunk load lazily.
- Two woff2 faces are preloaded and only those two are fetched at first load.
- No audio is fetched before a gesture.
- Every hashed file is `immutable`, the engine's module included (`MODULE_CACHE_CONTROL`).

**Fixed in #4714:**

| Measure | Before | After |
|---|---|---|
| Entry + modulepreloads, gzip, concatenated as `first-load.test.ts` measures | 383,498 B of a 393,216 B budget | 366,513 B (−16,985 B, −4.4%), budget lowered to 376,832 B |
| Entry + modulepreloads, raw | 1,399,714 B | 1,223,447 B (−176 KB to parse before first paint) |
| The preloaded `cards-*.js` chunk, raw | 488,869 B | 325,862 B; the offers are a lazy 163,972 B chunk |

- `COSMETIC_OFFERS` moved to its own generated module, `wire/generated/cosmeticOffers.ts` (the generator, `crates/server/tests/export_cosmetics.rs`, writes it). `cards/versions.ts` loads it with `loadSceneData()`, and the shop's route chunk loads it too.
- `FullScreenArt` loads the music player when the full-screen view opens, so the player and its data are no longer modulepreloaded.
- The boot starts the engine module's download (3.41 MB stored brotli) at low priority (`startWasmLoad`), so a route's own chunks come first. A screen waiting on the engine shares that download.
- `first-load.test.ts` guards the offers and the music player out of the entry's graph and out of the built first load. `wasm/index.test.ts` holds the low-priority boot fetch.
- The entry chunk itself barely moved (+224 B raw, the loaders): the offers had been in a preloaded chunk.

**Audio:**
- The library is 71 MB: voice is 21 MB in 1,908 files at about 48 kbps, music 51 MB in 255 files at about 82 kbps. Both rates suit a phone speaker, and static files cost nothing to serve.
- The voice prefetch fetches every line after the first board (B58), a few at a time and never under Save-Data, so that no first line waits on the network. It costs players' data, not the bill, and was not changed.

**Small unhashed files with no cache rule:** `favicon.svg` (937 B), `manifest.webmanifest` (724 B) and the two brand SVGs. They take Cloudflare's default revalidation, a few hundred bytes of headers, and were not changed.

## 5. Users far from the server

Does not apply.
- Static files are on Cloudflare's network, and the one shareable API read is now kept at the edge (#4713).
- The container sits in eastern North America beside Supabase's us-east-2, so the server's database round trips stay short.
- A second region or a second container would double the container bill and break the single in-memory match actor (`docs/architecture.md` §4.1, §6).
- Recorded as the finding, as #4691 asked, unless a person decides otherwise.

## Nothing slower

- #3429's budgets and `cargo jackioh gate` are untouched by these changes: no engine, AI or card code changed.
- `cargo jackioh perf --smoke` and the server's perf smoke run per pull request.
