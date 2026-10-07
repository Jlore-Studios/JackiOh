# v0.3.0 (part 19 of 40): server 2: the match lifecycle and the WebSocket

Part 19 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | nothing (the bot works from `main`) |
| Branch | the night bot's own branch and pull request to `main` (new files only; the orchestrator merges `main` into `staging`) |
| Builder | the night bot, `difficulty:hard` (Opus builds and reviews) |
| Notes | `.fullsend/notes/part-19.md` and `.assumptions` (SURFACE §16) |

Queued for the night bot by #306's orchestrator (`bot:build`, `difficulty:hard`). The Plan below is between the night bot's plan markers: build from it without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port the match lifecycle of `apps/server` to `crates/server`: the WebSocket at `/ws/match` (handshake checks, close codes, frames), the protocol, one actor per match (nonce dedupe, seat stamping, views and legal actions per seat, prompts, clocks, grace, flood limits, emote and aim relays, seat swap, absent-seat grace, terminal handling, results and voids), the registry (start, attach, rebuild by `fold` after a restart), the clock, rooms, the queue and matchmaker, Conquest series and their sweeper, rematch, results and the reaper, and game records.

**How you work (fullsend builder rules, in priority order; full text in `.claude/skills/fullsend/agents/builder.md`).**
1. Never run `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or any test runner. Nothing on `staging` compiles until Wave 3; every error you would see is someone else's unwritten file. Count any you ran as `BUILDS-RUN`.
2. Write only the files in your Files to touch table and your two notes files. Read anything in `packages/`, `apps/`, `docs/`, `SPEC.md` (the TypeScript is your spec); do not read other parts' Rust under `crates/`: it is half-written.
3. Match SURFACE.md exactly at every boundary: paths by §4.1, names by §4.2, types by §4.3, the shapes of §5–§14.
4. No stubs: no `todo!()`, `unimplemented!()`, placeholder bodies or `// TODO`. Port the real body; if you truly cannot, leave the function out and list it under GAPS.
5. Duplicate on purpose: a small helper another part probably writes, write your own private copy.
6. Call what you wish existed: another module's function by its TS name snake_cased at its TS module's Rust path.
7. Don't ask questions: decide, and record the decision in your notes.
8. Stop when Done when holds.

**Delivering (night bot).** Build on your own branch and open your pull request to `main` as you always do. Never push to `staging` and never create it: #306's orchestrator merges `main` into `staging`. Commit only the files in your table and your two notes files. Your pull request adds new files and changes none, so `main`'s TypeScript CI stays green. Do not run `cargo` (builder rule 1): `main` has no Cargo workspace, and the Rust is expected not to compile until Wave 3 on `staging`. The plan's documents are not on `main`. Read them from the plan branch without committing them: `git fetch origin claude/relaxed-archimedes-i5fdah && git show origin/claude/relaxed-archimedes-i5fdah:docs/v0.3.0/SURFACE.md` (likewise `README.md`, `PORT-MAP.md`, `port-map.tsv`, `research/*.md` and `parts/<your brief>.md`).

### 2. Files to touch

| File | new/change/delete | What |
|---|---|---|
| `crates/server/tests/support/engine.rs` | new | the test cards of test/fakes/engine.ts (`test-prompt-self`, `test-prompt-enemy`, `test-lethal`) as real engine scripts installed with the testkit override (SURFACE §8, §11.2); ported from `apps/server/test/fakes/engine.ts` (659 lines); no ScriptedEngine |
| `crates/server/tests/support/socket.rs` | new | a fake Socket over an mpsc channel (← test/fakes/socket.ts); port of `apps/server/test/fakes/socket.ts` (91 lines). |
| `crates/server/src/api/game_records.rs` | new | port of `apps/server/src/api/game-records.ts` (85 lines). |
| `crates/server/src/api/queue.rs` | new | port of `apps/server/src/api/queue.ts` (578 lines). |
| `crates/server/src/api/rematch.rs` | new | port of `apps/server/src/api/rematch.ts` (276 lines). |
| `crates/server/src/api/results.rs` | new | port of `apps/server/src/api/results.ts` (325 lines). |
| `crates/server/src/api/series_rules.rs` | new | port of `apps/server/src/api/series-rules.ts` (654 lines). |
| `crates/server/src/api/series.rs` | new | port of `apps/server/src/api/series.ts` (570 lines). |
| `crates/server/src/actor/match_actor.rs` | new | port of `apps/server/src/match/actor.ts` (917 lines). |
| `crates/server/src/actor/clock.rs` | new | port of `apps/server/src/match/clock.ts` (299 lines). |
| `crates/server/src/actor/contracts.rs` | new | port of `apps/server/src/match/contracts.ts` (158 lines). |
| `crates/server/src/actor/engine.rs` | new | port of `apps/server/src/match/engine.ts` (198 lines) and `engine.real.ts` (103 lines) as plain `pub fn`s that call `jackioh_engine` directly (the port's methods: create, reduce, per-seat snapshot with view and legal actions, fold, last boards); no `Engine` trait, no `EnginePort`, no dynamic import, no `EngineUnavailableError` (SURFACE §11.3) |
| `crates/server/src/actor/protocol.rs` | new | port of `apps/server/src/match/protocol.ts` (532 lines). |
| `crates/server/src/actor/registry.rs` | new | port of `apps/server/src/match/registry.ts` (195 lines). |
| `crates/server/src/actor/rooms.rs` | new | port of `apps/server/src/match/rooms.ts` (350 lines). |
| `crates/server/src/actor/ws_server.rs` | new | port of `apps/server/src/match/wsServer.ts` (341 lines). |
| `crates/server/tests/api/game_records.rs` | new | port of `apps/server/test/api/game-records.test.ts` (236 lines). |
| `crates/server/tests/api/queue.rs` | new | port of `apps/server/test/api/queue.test.ts` (1241 lines). |
| `crates/server/tests/api/rematch.rs` | new | port of `apps/server/test/api/rematch.test.ts` (638 lines). |
| `crates/server/tests/api/results.rs` | new | port of `apps/server/test/api/results.test.ts` (579 lines). |
| `crates/server/tests/api/series_rules.rs` | new | port of `apps/server/test/api/series-rules.test.ts` (799 lines). |
| `crates/server/tests/api/series.rs` | new | port of `apps/server/test/api/series.test.ts` (661 lines). |
| `crates/server/tests/actor/match_actor.rs` | new | port of `apps/server/test/match/actor.test.ts` (1991 lines). |
| `crates/server/tests/actor/aim.rs` | new | port of `apps/server/test/match/aim.test.ts` (232 lines). |
| `crates/server/tests/actor/clock.rs` | new | port of `apps/server/test/match/clock.test.ts` (451 lines). |
| `crates/server/tests/actor/dealt_deck.rs` | new | port of `apps/server/test/match/dealt-deck.test.ts` (128 lines). |
| `crates/server/tests/actor/engine_real.rs` | new | port of `apps/server/test/match/engine.real.test.ts` (198 lines). |
| `crates/server/tests/actor/glitch.rs` | new | port of `apps/server/test/match/glitch.test.ts` (324 lines). |
| `crates/server/tests/actor/last_boards.rs` | new | port of `apps/server/test/match/last-boards.test.ts` (225 lines). |
| `crates/server/tests/actor/recovery.rs` | new | port of `apps/server/test/match/recovery.test.ts` (622 lines). |
| `crates/server/tests/actor/rooms.rs` | new | port of `apps/server/test/match/rooms.test.ts` (704 lines). |
| `crates/server/tests/actor/series_recovery.rs` | new | port of `apps/server/test/match/series-recovery.test.ts` (488 lines). |
| `crates/server/tests/actor/ws_server.rs` | new | port of `apps/server/test/match/ws-server.test.ts` (157 lines). |

Do **not** touch: any `Cargo.toml`, `lib.rs`, `mod.rs` or `main.rs` (part 1's), other parts' files, `packages/` and `apps/server/` (read only; part 37 deletes them).

### 3. Steps

1. Read SURFACE.md §2, §4 and §11 fully, and `docs/v0.3.0/research/C-server.md` (research C) §2–§4 for the routes, frames and store methods you own.
2. Port each file in your table. Handlers have exactly the shape of §11.2 (`pub async fn <ts name snake_cased>(app: &App, req: Req) -> ApiResult`); JSON bodies, status codes, headers (`cache-control`, `Retry-After`) and error codes are TS's byte for byte.
3. `actor/protocol.rs`: research C §3.2–§3.3's frames as serde types; `parse_client_message` total and whitelisting; the nonce is read only inside `action`; `joinRoom` answers `malformed`; 65,536-byte frame cap; `MAX_NONCE_LENGTH`, `SERVER_NONCE_PREFIX`.
4. `actor/ws_server.rs`: `handle(app, req)` (SURFACE §11.2): Origin check against `PUBLIC_ORIGINS` (raw 403), ≤ 10 sockets per address (raw 429), token from `?token=` only, then verify → profile → active → `matchId` = query or `profile.in_match_id`, close codes 4401/4403/4404/1011 after an `error` frame, `jackioh.v1` echoed, 1009 on an oversized frame. No heartbeats (TS has none).
5. `actor/match_actor.rs`: one tokio task per match with an mpsc inbox; a serialized queue exactly as actor.ts: dedupe nonce (a replayed nonce gets the original ack), refuse `match_over`, stamp `playerId`, `jackioh_engine::reduce`, append the action **before** committing state (`internal` on failure), then views ×2 (with `clockMs` and `legal`), prompts (only on change), clocks, terminal → `record_result` or void (close 4410). The engine is called directly; there is no Engine trait.
6. `actor/registry.rs`: SURFACE §11.2's API. `start` creates the game with `create_game` + `begin_game` before writing the match row; `attach` rebuilds a missing actor with `jackioh_engine::fold` (logging `match.fold.errors`), dedupes concurrent rebuilds, replaces an old socket, clears/starts grace (R744). Finished actors stay until the reaper or a restart.
7. `actor/clock.rs`: turn 75 s (paused while the other seat holds a prompt), prompt 30 s, mulligan 45 s, grace 60 s, ceiling 120 min, expiries as server actions with nonce `srv-<kind>-<seq>` (research C §3.5), persisted with `matches_set_clocks` when changed.
8. `api/queue.rs`, `actor/rooms.rs`, `api/series_rules.rs`, `api/series.rs`, `api/rematch.rs`, `api/results.rs`, `api/game_records.rs`: as TS, with the loop functions SURFACE §11.2 names (`run_matchmaker`, `run_sweeper`, `run_reaper`). All Random decks: `jackioh_ai::build_ai_deck(&mut Rng::new("{seed}:p1-deck", 0), DECK_SIZE, &AiDeckOptions { banned: vec![], .. })` (and `p2`).
9. Port the tests in your table to `crates/server/tests/...` (TS behaviour; porting them here does not ratify your code). Use `support::deps::test_app()` and `support::deps::call(...)` (part 18 writes them) and `#[tokio::test]` with `tokio::time::pause()` where TS used manual timers.
10. Commit after every 5 files on your branch.

### 4. Tests

- The ported match and lifecycle tests in the table (actor, recovery, series-recovery, clock, ws-server, rooms, glitch, aim, dealt-deck, last-boards, engine.real, queue, series-rules, series, rematch, results, game-records).

### 5. Done when

- [ ] Every frame of research C §3.2–§3.3 and every route numbered 17–25, 31–32 has its port; the handlers match §11.2's signature.
- [ ] `.fullsend/notes/part-19.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- Actor ordering (append before commit, prompts only on change) is what e2e specs 05, 06 and 20 observe.
- tokio's paused clock auto-advances when every task is idle; a test that awaits a frame that never comes would fire timers. Drive the actor through its inbox and assert with `try_recv` where the TS test asserted "nothing happened".

<!-- /jackioh-bot:plan -->
