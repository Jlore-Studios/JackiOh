# v0.3.0: the Rust rewrite, the Rust-native AI, the training lanes and the CI trim

The plan for [#306](https://github.com/Jlore-Studios/JackiOh/issues/306), which merges #80 (the
server, engine and AI rewrite) and #133 (spec adherence by structure, not prose), and adds the
AI training lanes (the successor of #55, following #70 and #65) and a shorter CI. It is written so
that a builder who reads only this file, [SURFACE.md](SURFACE.md), [PORT-MAP.md](PORT-MAP.md) and
its own part in [parts/](parts/) has no decision left to make.

- **[SURFACE.md](SURFACE.md)** is the interface freeze: crate layout, pinned dependencies,
  translation rules, wire format, every cross-crate signature, CLI, file formats. It is the fullsend
  `SPEC.md` for this run.
- **[PORT-MAP.md](PORT-MAP.md)** lists every file in `packages/`, `apps/server/` and `ladder/`, the
  Rust file it becomes (or that it is deleted, and why), and the part that owns it.
- **[parts/](parts/)** holds one brief per sub-issue of #306, the same text as the issue's body.

Contents:

1. Goal and non-goals
2. Decisions
3. How the work runs: fullsend over one branch
4. The waves and the parts
5. The orchestrator's runbook
6. Acceptance: the behaviours v0.3.0 is held to
7. CI after v0.3.0: essential per PR, everything daily
8. The AI training lanes
9. The spec graph
10. Exceptions removed on the way
11. Freeze rules for `main` while this runs
12. Risks and abort criteria

---

## 1. Goal and non-goals

**Goal.** At the end, `main` holds:

- a Cargo workspace (`crates/`) with the rules engine, the 318 card scripts, the deck validator, the
  practice AI, the HTTP/WebSocket server and every tool (fuzz, gates, sweep, stats, catalog,
  patches, spec checks, training arena) in Rust, with behaviour identical to v0.2.11's TypeScript,
  proved by golden traces recorded from the TypeScript engine before it was deleted;
- `apps/web` unchanged for players, running the Rust engine and AI as WebAssembly for hotseat and
  practice, and talking to the Rust server over the same wire protocol;
- no TypeScript under `packages/` or `apps/server/`, and no `ladder/`;
- `spec/`: SPEC.md as atomic, linked notes with a structural `spec check` instead of prose tests;
- a per-PR CI under five minutes per job that runs the essential tests, and a daily "super" run
  that runs everything else and opens an issue when something breaks;
- two always-on AI training lanes on the training box, each a Devin agent improving the Rust AI
  against fixed promotion gates and logging every game it plays.

**Non-goals** (each is a later issue, not this one):

- Cloudflare, the `production` branch, a second database and a production Render service (#306:
  "main == staging"; #256). Nothing here changes `wrangler.jsonc`, `promote-production.yml` or
  `docs/deploy-cloudflare.md`; a file shared with them changes only as far as staging needs.
- New rules, cards or player-visible changes. v0.3.0 ships the same game.
- A new wire format, persistent data structures, or any optimisation the profiler has not asked for.
  Rust's plain `clone()` and `serde_json` are the baseline; measure before adding anything.
- The night bot's own code beyond the lane counts and the training box (part 39).

## 2. Decisions

Each line is decided; a builder does not reopen it.

| # | Decision | Why |
|---|---|---|
| D1 | **Rust for everything below the UI; no Go.** | One language lets the server, the WASM build and the tools link the engine crate directly: no FFI, no IPC, no second implementation of the rules. #306 asked for "rust/go"; Go would put a process boundary between the server and the engine. |
| D2 | **Six crates: `engine` (with shared types and the validator), `cards`, `ai`, `wasm`, `server`, `tools`.** | The fewest crates that keep the purity line (engine, cards, ai are pure) and give each lane its own directory. `packages/shared` and `packages/validator` are too small to be crates of their own. |
| D3 | **The web client stays TypeScript/React and reaches the engine and AI through WASM, JSON strings in and out, behind the existing `EnginePort`.** | The client already talks to the engine through one seam (`apps/web/src/game/engine.ts`). JSON at the boundary needs no handle lifetimes; the AI's search never crosses it. |
| D4 | **Exact behavioural parity with v0.2.11.** The RNG ports bit for bit; golden traces recorded from TypeScript (view hashes, event hashes, legal-action sets) must replay identically in Rust. | It makes the "fix the broken branch" phase mechanical: the first divergent step names the bug. It also means every stored match log still replays. |
| D5 | **The client's TS types are generated from Rust (`ts-rs`).** | One source of truth for the wire; `packages/shared` goes. |
| D6 | **Card data moves byte for byte to `crates/cards/` (`catalog.json`, `patches/`).** Its TS tooling becomes `cargo jackioh` subcommands. | The data's home follows its code; formats do not change, so the patch history and the database seed are untouched. |
| D7 | **Every behavioural test is kept and ported; tests of prose, of deleted tooling and of TS internals are deleted.** Card tests live at the bottom of their card's file. | Unit tests are cheap in Rust. CI time goes on fuzz, AI gates and e2e, not on unit tests (§7). |
| D8 | **Server: axum, tokio, sqlx; the same routes, protocol, schema and migrations; a Docker service on Render.** | The wire and the database are the contract (#80). Docker frees Render from Node; the catalog version is compiled in from `patches.json`, which deletes the start-command workaround. |
| D9 | **The AI is ported as is from `packages/ai` at `91cc43c` (the last and only AI that passed its gates: 94/100, 35/50, 47/50), with its 11-card shadow ban.** That port is generation 0. | #306: "port the last successful AI into rust". The ladder (`ladder/`) never produced a champion. |
| D10 | **The training lanes run outside the night bot's harness**: two systemd services on the training box, each looping a Devin session over a standing prompt in `training/`, promoting through `cargo jackioh promote` and a pull request that CI re-verifies. | "Always running" contradicts the harness's queue, idle power-off and 330-minute job budget (research: `bot/machine/setup.sh`, `starter.py`, `.harness/config.json`). |
| D11 | **"Random AI" means SPEC §10.7's random policy** (`subsystems/aiPolicy.chooseAction`), the one the quality gate already measures. | #306 says "full random AI"; the spec has exactly one. |
| D12 | **SPEC.md becomes `spec/`, notes with front matter; `cargo jackioh spec check` replaces `rulings.test.ts` and `rulings-coverage.ts`.** | #133 and #306's "obsidian like graph": checks read ids, never wording; agents read a 30-line note instead of a 2,000-line file. |
| D13 | **Fullsend over one integration branch, `staging`**, built by Claude Code cloud sessions created with `outcome_branch: staging`, one per part. | The bots' harness can only open pull requests to `main` (`bot/harness/deliver.py:1219`, `cfg.default_branch`), and its merges wait on a green CI that a fullsend branch does not have until Wave 3. |
| D14 | **Per-PR CI runs the essentials; a daily "super" run runs everything.** | #306: "only run the essential tests on the CI". |
| D15 | **Ponytail throughout.** No abstraction with one implementation (the server's two-variant enums of D18 are the only polymorphism), no config for a value that never changes, no structural sharing or binary wire format until a profile asks. | #306 and the goal ask for it. |
| D16 | **`loc` stays frozen data.** The catalog's `loc` (a card script's TypeScript line count) is gameplay data read by C+ #44, C+ #45 and Classic #48; it keeps its values, and a new card's count comes from `cargo jackioh catalog loc`. | Recomputing it from Rust would change three cards' behaviour (research A §8 #16). |
| D17 | **No mutable globals and no registration hooks in the rules.** The catalog and script registries are set once; fused scripts are composed on lookup from the game state; per-call flags live in `EngineSink`; the ~25 import-time `register*` hooks become direct calls. | The tools run games on many threads; TS's process-global re-sync (`syncFusedScripts` on every `reduce`) would let games see each other's cards (research A §8 #7, #8, #18). |
| D18 | **The server has no traits:** `Db`/`Tx` and `Auth` are enums with one variant per implementation, and the server calls the engine directly. | Two implementations each; enums avoid generic `App<S, A, E>` signatures and `Send`-bound async-trait puzzles for the builders. |
| D19 | **The state hash stays TS's** (FNV-1a 32 over the UTF-16 code units of the sorted-key JSON). | Practice saves on players' devices store it and are refused if the fold disagrees; a pinned fixture and three e2e specs compare it (research A §3.2). |
| D20 | **Kept, though they look like exceptions:** the `activatePower` alias (stored match logs carry it and `match_actions` rows are immutable by trigger), the E2E server mode (its fixture tokens are e2e's contract), the 0013 migration checksum exception. | Removing them would break stored data or the test harness. |

## 3. How the work runs: fullsend over one branch

This is the fullsend skill (`.claude/skills/fullsend/SKILL.md`) stretched over sub-issues instead of
one session's subagents. Every fullsend rule holds; the mapping is:

| Fullsend | Here |
|---|---|
| `.fullsend/SPEC.md` | [SURFACE.md](SURFACE.md), plus the TS sources it names and the golden traces as the behaviours |
| The feature branch | `staging`, cut from `main` by part 1 |
| Phase 0: spec and freeze | this directory, plus part 1 (which turns the type freeze into compiled Rust) |
| Phase 1: shatter | the parts table (§4) and [PORT-MAP.md](PORT-MAP.md): every file belongs to exactly one part per wave |
| Phase 2: full send | Wave 1: parts 2–30 at once, every builder blind (no `cargo`, no `pnpm`), every one pushing straight to `staging` |
| Spec-testers | the golden traces (part 23) and the test porters (parts 24–27), who port TS tests against SURFACE.md without reading the Rust being written |
| Phase 3: contact | part 31, first half |
| Phase 4: reconcile | part 31, second half |
| Phase 5: green | Wave 3: part 32, then parts 33–36 |
| Phase 6: cull | part 37 |

### 3.1 Who builds a part

Two kinds of builder share Wave 1 (the orchestrator's choice on #306, 2026-10-07):

- **The night bot**, `difficulty:hard` (Opus builds and reviews), takes the 18 parts whose output
  is new files only: 9–16 (cards), 17 (AI), 18–19 (server), 22 (CLI), 23 (golden traces), 24–27
  (engine tests) and 29 (training). Its deliver step opens pull requests to `main`. That is safe:
  the pull request adds files under `crates/` (and `training/`, `scripts/golden/`), which `main`'s
  TypeScript CI neither builds nor reads. The orchestrator merges `main` into `staging` after each
  bot merge. Where part 1 left an empty placeholder at the same path, `main`'s file wins. A bot
  part whose pull request stalls is harvested: its branch's files are merged into `staging` by hand.
- **Subagents of the orchestrator's session**, each in its own git worktree, take every other part
  (1–8, 20, 21, 28, 30 and Waves 2–5). They push to `staging` by §3.2.

The bot parts read the plan's documents from the plan branch (their issues say how), since `main`
does not carry `docs/v0.3.0/` until the cutover. Parts marked
**person** touch `.github/`, `.harness/`, `.squishy/` or `bot/`, which no bot may change; a person
builds them, or runs a session of their own to do it.

The sub-issues are **unlabelled** until a person says they are ready (#306's comment: "hold off
labeling things until it's ready"). No `bot:*`, `squishy:*` or `method:*` label goes on them from
this plan. The Plan section of each is between `<!-- jackioh-bot:plan -->` markers, so that if a
person later queues one for a bot, it builds from that plan without planning again.

### 3.2 The push protocol (every Wave 1–3 part)

```sh
git fetch origin staging && git checkout -B work origin/staging   # start
# … write only the files your part owns …
git add <only your files> .fullsend/notes/part-<NN>.md .fullsend/notes/part-<NN>.assumptions
git commit -m "v0.3.0 part <n>: <what>"
for i in 1 2 3 4 5; do git pull --rebase origin staging && git push origin HEAD:staging && break; sleep $((i*4)); done
```

- **Ownership is the only coordination.** A part writes only the paths its brief lists. Two parts
  never own one file in the same wave, so a rebase never conflicts. A rebase that does conflict
  means a part wrote outside its paths: drop your change to that file (`git checkout --theirs`) and
  list it under `GAPS`.
- **Commit early and often**: at least every 10 files. A session cut off mid-part leaves its work on
  `staging`, and the next session for that part starts with "skip every file that already exists".
- **Never run the build in Wave 1.** No `cargo`, `rustc`, `rustfmt`, `pnpm`, `tsc` or test runner
  (fullsend builder rule 1). Count any you ran in `BUILDS-RUN`.
- The build on `staging` is expected to be broken from part 2's first push until Wave 3.

### 3.3 Notes

Each part leaves `.fullsend/notes/part-<NN>.md` and `.fullsend/notes/part-<NN>.assumptions` (SURFACE.md
§16). Part 31 reads them all. `.fullsend/` stays on `staging` and is deleted by part 37.

## 4. The waves and the parts

Forty parts, one sub-issue each, titled `v0.3.0 (part n of 40): …` (`docs/issues-and-patches.md`). The waves run in order; inside a wave every part runs at once. "Opus" parts carry the cross-cutting design; "Sonnet+" parts are mechanical translations any strong model can do from their brief. Line counts per part are in [PORT-MAP.md](PORT-MAP.md).

- **Wave 0** (part 1): `staging`, the workspace, the module tree, the type freeze. Nothing else starts before it.
- **Wave 1** (parts 2–30, at once): every builder and test porter, blind, pushing straight to `staging`. Part 30 is a person's and may run any time in Waves 1–3.
- **Wave 2** (part 31): contact and reconcile.
- **Wave 3** (part 32, then parts 33–36 at once): green.
- **Wave 4** (part 37, then part 38): cull and docs, then the cutover to `main`.
- **Wave 5** (parts 39 and 40): the bot machines and the training lanes switched on.

| Part | Title | Wave | Starts after | Builder | Mode | Brief | Issue |
|---|---|---|---|---|---|---|---|
| 1 | bootstrap: staging, the Cargo workspace and the type freeze | Wave 0 | — | Opus | setup | [01-bootstrap.md](parts/01-bootstrap.md) | #389 |
| 2 | engine 1: the model and the board | Wave 1 | 1 | Sonnet+ | builder | [02-engine-1.md](parts/02-engine-1.md) | #390 |
| 3 | engine 2: combat, damage and the resolution loop | Wave 1 | 1 | Opus | builder | [03-engine-2.md](parts/03-engine-2.md) | #391 |
| 4 | engine 3: the play pipeline and casting | Wave 1 | 1 | Sonnet+ | builder | [04-engine-3.md](parts/04-engine-3.md) | #392 |
| 5 | engine 4: turn, setup, reduce, view, validator, wire helpers and the testkit | Wave 1 | 1 | Opus | builder | [05-engine-4.md](parts/05-engine-4.md) | #393 |
| 6 | engine 5: effect verbs, first half | Wave 1 | 1 | Sonnet+ | builder | [06-engine-5.md](parts/06-engine-5.md) | #394 |
| 7 | engine 6: effect verbs, second half | Wave 1 | 1 | Sonnet+ | builder | [07-engine-6.md](parts/07-engine-6.md) | #395 |
| 8 | engine 7: subsystems | Wave 1 | 1 | Opus | builder | [08-engine-7.md](parts/08-engine-7.md) | #396 |
| 9 | cards lane 1: Core #1–#48 | Wave 1 | — | night bot (Opus) | builder | [09-cards-lane-1.md](parts/09-cards-lane-1.md) | #397 |
| 10 | cards lane 2: Core #49–#81 | Wave 1 | — | night bot (Opus) | builder | [10-cards-lane-2.md](parts/10-cards-lane-2.md) | #398 |
| 11 | cards lane 3: Core #82–#100 and the Core tokens to Ghoul | Wave 1 | — | night bot (Opus) | builder | [11-cards-lane-3.md](parts/11-cards-lane-3.md) | #399 |
| 12 | cards lane 4: Core tokens Rush to Sheep, Classic #1–#32 | Wave 1 | — | night bot (Opus) | builder | [12-cards-lane-4.md](parts/12-cards-lane-4.md) | #400 |
| 13 | cards lane 5: Classic #33–#68 | Wave 1 | — | night bot (Opus) | builder | [13-cards-lane-5.md](parts/13-cards-lane-5.md) | #401 |
| 14 | cards lane 6: Classic #69–#90 and tokens, Classic+ #1–#18 | Wave 1 | — | night bot (Opus) | builder | [14-cards-lane-6.md](parts/14-cards-lane-6.md) | #402 |
| 15 | cards lane 7: Classic+ #19–#49 | Wave 1 | — | night bot (Opus) | builder | [15-cards-lane-7.md](parts/15-cards-lane-7.md) | #403 |
| 16 | cards lane 8: Classic+ #50–#78 and the AI tokens | Wave 1 | — | night bot (Opus) | builder | [16-cards-lane-8.md](parts/16-cards-lane-8.md) | #404 |
| 17 | the AI in Rust (generation 0) | Wave 1 | — | night bot (Opus) | builder | [17-the-ai-in-rust-generation-0.md](parts/17-the-ai-in-rust-generation-0.md) | #405 |
| 18 | server 1: the HTTP API and auth | Wave 1 | — | night bot (Opus) | builder | [18-server-1.md](parts/18-server-1.md) | #406 |
| 19 | server 2: the match lifecycle and the WebSocket | Wave 1 | — | night bot (Opus) | builder | [19-server-2.md](parts/19-server-2.md) | #407 |
| 20 | server 3: persistence, migrations, CLIs and the Docker deploy | Wave 1 | 1 | Opus | builder | [20-server-3.md](parts/20-server-3.md) | #408 |
| 21 | the WASM bindings and the web client on them | Wave 1 | 1 | Opus | builder | [21-the-wasm-bindings-and-the-web-client-on-them.md](parts/21-the-wasm-bindings-and-the-web-client-on-them.md) | #409 |
| 22 | the jackioh CLI: fuzz, catalog, patches, gates, sweep, stats | Wave 1 | — | night bot (Opus) | builder | [22-the-jackioh-cli.md](parts/22-the-jackioh-cli.md) | #410 |
| 23 | golden traces recorded from the TypeScript engine | Wave 1 | — | night bot (Opus) | oracle | [23-golden-traces-recorded-from-the-typescript-engine.md](parts/23-golden-traces-recorded-from-the-typescript-engine.md) | #411 |
| 24 | engine tests 1: effect verbs and fixtures | Wave 1 | — | night bot (Opus) | tester | [24-engine-tests-1.md](parts/24-engine-tests-1.md) | #412 |
| 25 | engine tests 2: rulings, subsystems, prompts and triggers | Wave 1 | — | night bot (Opus) | tester | [25-engine-tests-2.md](parts/25-engine-tests-2.md) | #413 |
| 26 | engine tests 3: view, turn, setup, combat | Wave 1 | — | night bot (Opus) | tester | [26-engine-tests-3.md](parts/26-engine-tests-3.md) | #414 |
| 27 | engine tests 4: play pipeline and cross-card rules | Wave 1 | — | night bot (Opus) | tester | [27-engine-tests-4.md](parts/27-engine-tests-4.md) | #415 |
| 28 | the spec graph and structural spec checks (#133) | Wave 1 | 1 | Sonnet+ | builder | [28-the-spec-graph-and-structural-spec-checks-133.md](parts/28-the-spec-graph-and-structural-spec-checks-133.md) | #416 |
| 29 | the training arena, the promotion gate and the lane prompts | Wave 1 | — | night bot (Opus) | builder | [29-the-training-arena-the-promotion-gate-and-the-lane-prompts.md](parts/29-the-training-arena-the-promotion-gate-and-the-lane-prompts.md) | #417 |
| 30 | CI: essential checks per pull request, the daily super run | Wave 1 | 1 | person | person | [30-ci.md](parts/30-ci.md) | #418 |
| 31 | contact and reconcile | Wave 2 | 2–29 | Opus | reconciler | [31-contact-and-reconcile.md](parts/31-contact-and-reconcile.md) | #419 |
| 32 | green 1: the engine compiles and replays the golden traces | Wave 3a | 31 | Opus | green | [32-green-1.md](parts/32-green-1.md) | #420 |
| 33 | green 2: the cards | Wave 3b | 32 | Sonnet+ | green | [33-green-2.md](parts/33-green-2.md) | #421 |
| 34 | green 3: the AI, the tools and the training arena | Wave 3b | 32 | Sonnet+ | green | [34-green-3.md](parts/34-green-3.md) | #422 |
| 35 | green 4: the server and the database | Wave 3b | 32 | Opus | green | [35-green-4.md](parts/35-green-4.md) | #423 |
| 36 | green 5: the web client and e2e | Wave 3b | 32 | Sonnet+ | green | [36-green-5.md](parts/36-green-5.md) | #424 |
| 37 | cull: delete the TypeScript, rewrite the docs | Wave 4 | 33, 34, 35, 36 | Opus | culler | [37-cull.md](parts/37-cull.md) | #425 |
| 38 | cutover: staging into main, deploys and the bots' settings | Wave 4 | 37, 30 | person | ops | [38-cutover.md](parts/38-cutover.md) | #426 |
| 39 | the bot machines: six normal lanes and two training lanes | Wave 5 | 38 | person | person | [39-the-bot-machines.md](parts/39-the-bot-machines.md) | #427 |
| 40 | generation 0, the sweep of record, and the lanes switched on | Wave 5 | 38, 39 | Sonnet+ | ops | [40-generation-0-the-sweep-of-record-and-the-lanes-switched-on.md](parts/40-generation-0-the-sweep-of-record-and-the-lanes-switched-on.md) | #428 |

## 5. The orchestrator's runbook

The orchestrator is one person, or one Claude Code session a person runs (it needs the
`create_session` tool of the `claude-code-remote` server). It never writes product code. Its steps:

1. **Before Wave 0.** Post on #306 that the freeze of §11 starts. Merge this plan's branch into
   `main`, or keep it on its branch; part 1 copies `docs/v0.3.0/` into `staging` either way.
2. **Wave 0.** Create one session for part 1 (prompt template below). Wait for it to finish and for
   its Done when (the workspace checks with no error but unresolved names into still-empty modules,
   and the tag `v0.3.0-wave-0` is pushed). Nothing else starts before.
3. **Wave 1.** Merge `main` into `staging` once (`git merge origin/main`, web and docs only, per
   §11). Create every Wave 1 session at once, parts 2–29 (part 30 is a person's). Do not watch them;
   do not correct them mid-flight (fullsend Phase 2). The wave ends when every session has stopped
   or 24 hours have passed. A part whose session stopped short gets one more session with the same
   prompt; it resumes from what is on `staging`.
4. **Wave 2.** Create the part 31 session. It ends with `.fullsend/damage.md` and
   `.fullsend/notes/reconcile-decisions.md` committed.
5. **Wave 3.** Create part 32's session first (the engine must compile before anything downstream
   can be checked); when it is green, create parts 33–36 at once. If a part reaches
   its fourth iteration (fullsend: "the spec has a hole"), stop it, patch SURFACE.md from
   `.fullsend/notes/spec-gaps.md`, and rerun.
6. **Wave 4.** Part 37, then part 38 (the cutover pull request from `staging` to `main`).
7. **Wave 5.** Parts 39 and 40 (39 is a person's).

Session prompt template (`create_session`, with `outcome_branch: "staging"`,
`source_revision: "staging"` (for part 1 only: `source_revision: "main"`, since part 1 creates
`staging`), `permission_mode` as the orchestrator's, the model §4's table names):

```
You are building part <n> of the JackiOh v0.3.0 rewrite (GitHub issue #<sub-issue>, parent #306).
Read, in this order: docs/v0.3.0/README.md §3, docs/v0.3.0/SURFACE.md, docs/v0.3.0/parts/<file>.md
(the same text as the issue) and the research file it cites, if any. Your brief is the last file; its "Files to touch" table is the complete list of paths you may write.
Follow its steps exactly. Push to staging with the protocol of README §3.2. Do not ask questions:
decide, and record the decision in your notes. Stop when your brief's "Done when" list holds.
```

## 6. Acceptance: the behaviours v0.3.0 is held to

These are the run's numbered behaviours (fullsend's B-numbers, called V here so they do not collide
with BUILD.md's). Parts cite them; part 37's coverage check lists any V that no test cites.

| V | Behaviour | Checked by |
|---|---|---|
| V1 | `cargo build --workspace` and `cargo clippy --workspace --all-targets -- -D warnings` pass | CI `rust` job |
| V2 | The pure crates depend on nothing but serde, serde_json, indexmap and each other | `clippy.toml` + Cargo.toml review in part 37 |
| V3 | `Rng::new("golden", 0)`'s first ten draws equal TS's | `crates/engine/src/rng.rs` unit test |
| V4 | Every game in `games.jsonl` replays with identical view hashes, event hashes and legal sets | `crates/engine/tests/golden.rs` |
| V5 | Every TS card test, ported, passes | `cargo test -p jackioh-cards` |
| V6 | Every §11 ruling has a proving Rust test named `r<n>_…`, and every cited R-id exists | `cargo jackioh spec check` |
| V7 | `cargo jackioh fuzz --seeds 1000` finds no invariant breach and every game folds back to its hash | daily super run (CI: 200 seeds) |
| V8 | `cargo jackioh catalog check` and `patches check` pass on the moved data | CI `rust` job |
| V9 | `replay(args, log)` of a game equals the live state's `hash_state` | fuzz + server actor test |
| V10 | The AI gates pass at their `gateNeeded` counts: vs random 100, vs greedy 50, Hard vs Easy 50 | daily super run (`cargo jackioh gate --full`) |
| V11 | No AI decision reads hidden information: two states that differ only in hidden cards redact identically | ported `observe` tests |
| V12 | `crates/ai`'s `SHADOW_BAN` equals TS's 11 entries at cutover | AI unit test |
| V13 | Every REST route answers with TS's status, headers and JSON body for the same request | `crates/server/tests/api/*` |
| V14 | Every WebSocket message has TS's shape; a match is playable end to end over the socket | `crates/server/tests/actor/*` + e2e specs 05, 06 |
| V15 | A match survives a server restart by folding (seed, log) | actor test + e2e |
| V16 | The `Store` contract suite passes against the fake and against Postgres | `cargo test` + `test:db` |
| V17 | Migrations apply unchanged on an empty Postgres and on the live schema; RLS and trigger invariants hold | `test:sql` |
| V18 | The server serves the catalog version of the newest `patches.json` entry, with `x-deployed-commit` | API test + `deploy-watch.yml` |
| V19 | The web client plays a hotseat game and a practice game on the WASM engine | e2e specs 01 and 13 |
| V20 | `apps/web`'s TS types are exactly what `ts-rs` generates | CI diff check |
| V21 | All 35 e2e specs pass on Chrome | daily super run |
| V22 | The Docker image boots on Render's command with the env contract of `docs/architecture.md` | `test:deploy` |
| V23 | No test reads prose from a Markdown file | part 28's grep |
| V24 | Per-PR CI's slowest job finishes under 5 minutes | `ci-duration.yml` |
| V25 | The daily super run opens or updates one issue on failure and closes it on success | its workflow |
| V26 | `cargo jackioh promote` promotes exactly by §8's table, and writes `generation.json` and the history line | `crates/tools` tests |
| V27 | A training lane runs forever, survives a reboot, and logs one `GameRecord` per game it plays | part 39's check on the box |
| V28 | No TypeScript remains under `packages/` or `apps/server/`, and `ladder/` is gone | part 37 |

## 7. CI after v0.3.0

#306 asks whether all the tests are needed. They are kept, but they no longer all run on every pull
request. The cost of today's CI is not unit tests: it is the fuzz gate (1,000 seeds), the AI quality
gates (12 shards), coverage (shards), and 35 e2e specs on two browsers. In Rust the unit and card
tests run in seconds. So:

**Per pull request (required checks):**

| Check | Runs | Target |
|---|---|---|
| `rust` | `cargo fmt --check`; `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace` (engine rules tests, every card test, golden replay, AI unit tests, server tests on the fake store, tools tests); `cargo jackioh catalog check`; `patches check`; `spec check`; `fuzz --seeds 200` | < 5 min with `Swatinem/rust-cache` |
| `web` | `scripts/build-wasm.sh`; `pnpm --dir apps/web typecheck`; `eslint apps/web e2e`; `vitest run --project web`; the `ts-rs` diff check (V20) | < 5 min |
| `e2e smoke` | Chrome only, specs 01 (hotseat game and replay hash), 06 (networked match), 10 (invite gate), 13 (practice against the AI), 19 (queue modes and series), against a release build of `jackioh-server` | < 5 min |
| `db` | `test:sql`, `test:db`, `test:deploy`, only when `crates/server/migrations/**`, `crates/server/src/db/**`, `crates/server/Dockerfile` or `render.yaml` changed (a skip passes the check) | < 5 min |
| `bot selftest` | unchanged | — |

**Daily (`.github/workflows/super.yml`, cron 07:17 UTC, and on demand):** `fuzz --seeds 10000` and
`--handicap`, `gate --full`, coverage with `cargo llvm-cov` over engine, cards and ai (fails under
the floor in `.github/coverage-floor.txt`, 90% at the start, raised by hand as it climbs toward #306's
99%), all 35 e2e specs on Chrome plus the component specs, `test:sql`, `test:db`, `test:deploy`, and
`golden check`. On failure it opens (or comments on) one issue titled `CI: daily super run failed`,
naming each failed job with a link; on success it closes that issue. Part 30 builds it.

**Dropped:** e2e on Electron (it failed on the same shard as Chrome in 8 of the last 10 failing runs,
research D §1.1; Cypress is deprecating it), the 12-shard AI gate per pull request (statistical and
flaky: it failed a `main` push on 2026-10-06), the coverage shards per pull request, and the tests
that only pin prose or deleted tooling (parts 28 and 37).

The bots' `required_checks` lists (`.harness/config.json`, `.squishy/config.json`) are rewritten to
the new check names in the same pull request, or the bots wait forever on checks that no longer
exist.

## 8. The AI training lanes

The successor of #55's ladder, in Rust, run by Devin.

**What runs.** On the training box (EC2 `m7i.xlarge`, 4 vCPU, built by part 39, never powered off),
two systemd services, `jackioh-train@improve` and `jackioh-train@unban`, each as its own Linux user
(`agent-train-improve`, `agent-train-unban`) with its own checkout, Devin login and GitHub token.
Each runs `training/loop.sh <lane>` forever (`Restart=always`):

1. Fetch `main`; reset the lane's branch `ai/<lane>` to it; build `jackioh` from it and keep that
   binary as the parent (`~/parent-jackioh`): the parent is always the AI on `main`.
2. Start one Devin session (`devin -p --prompt-file training/<lane>.md --model "$DEVIN_MODEL"
   --permission-mode dangerous --respect-workspace-trust false --export ~/logs/<lane>-<t>.json`).
   The standing prompt tells Devin to change only `crates/ai/**`, to measure with
   `cargo jackioh promote --lane <lane> --parent-bin ~/parent-jackioh --dry-run`, to iterate, and to
   commit only after a non-dry-run `promote` exits 0.
3. When the session ends with a promotion commit, rebase on `main`; if `main`'s AI changed meanwhile,
   go back to 1 (the parent changed). Otherwise push `ai/<lane>` and open a pull request titled
   `AI gen <N> (<lane>): <one line from Devin>`, with auto-merge on.
4. Sleep 60 s and repeat.

**The gate.** `cargo jackioh promote` (SURFACE.md §14) plays 100 games against §10.7's random policy
and 100 against the parent, seats alternating, the same seeds against both, each seat's deck from all
three sets minus its own AI's shadow-ban list, and promotes on:

| Lane | vs random | vs parent | Shadow bans |
|---|---|---|---|
| improve | ≥ 90 of 100 | ≥ 85 of 100 | — |
| unban | ≥ 90 of 100 | ≥ 75 of 100 | strictly fewer than the parent's |

**CI re-verifies.** A pull request from an `ai/*` branch may change only `crates/ai/**` and
`training/history/**`, and its `generation.json` must be `main`'s plus one; the `training-gate` job
builds `main`'s binary and the branch's and re-runs `promote --lane <lane>` with the same seeds. A
lane's own claim is never trusted. Branch protection's "require branches to be up to date" makes a
lane whose parent moved re-run.

**Stats.** Every game the arena plays, in a promotion run or in Devin's experiments, appends one
`GameRecord` (R376, the format `ai:stats` already writes) to `~/training-out/<lane>/<date>.jsonl`.
Once a day the loop loads the day's file into Postgres with `jackioh-server stats-import` when the box
has `DATABASE_URL`, so `stats:cards --source=dev` reads them. A promotion appends its gate numbers to
`training/history/<lane>.jsonl`, committed with it.

**Generation 0** is the ported TS AI (part 17). Part 40 measures it, writes `generation.json`, runs
the sweep of record over all 268 cards that #65 left undone, and starts both services.

**Feasibility, measured from the TS AI (research for this plan).** The TS AI wins 94.5% against
random, so 90/100 passes with probability 0.98. Against its own parent, 85/100 needs a true win rate
of about 85% (pass probability 0.57 at 85%, 0.13 at 80%); for scale, Hard's whole resource handicap
over Easy wins 91.3%. A successor of the same strength wins about 50% and is never promoted. The
improve lane will promote rarely, and only on real gains. The thresholds are #306's and stay as
written; part 40 reports the first measured win rates on #306 so a person can revisit them.

## 9. The spec graph

`SPEC.md` is 1,986 lines; §11 alone has 561 ruling rows (R1–R752 with gaps). Today an agent reads
all of it, or greps it, and tests pin its wording (#133). After part 28:

- `spec/NN-<slug>.md`: one note per top-level section, its subsections inside.
- `spec/rulings/R0195.md`: one note per ruling, with front matter (`id`, `title`, `sections`,
  `sources`, `proven_in`) and the ruling's text, links written `[[R113]]` and `[[§10.3]]`.
- `spec/INDEX.md`: generated; each id, its title, its note and its proving tests, so a lookup is one
  line.
- `cargo jackioh spec check` (SURFACE.md §15) checks ids, file names and test names only. The
  hand-maintained `packages/engine/test/rulings.test.ts` index (one `provenIn` per row) is replaced
  by each note's `proven_in` list.
- `SPEC.md` at the root becomes a 20-line pointer to `spec/README.md`, so links in old issues still
  land somewhere.
- Every doc and code comment keeps citing `R<n>` and `§x.y`; nothing is renumbered.

Obsidian opens `spec/` as a vault as is: the `[[…]]` links and front matter are its native format,
and its graph view draws rulings, sections and their links.

## 10. Exceptions removed on the way

Each is a special case the TypeScript needed and the Rust does not. Parts delete them; none is a
player-visible change.

| Exception today | Where | Gone because | Part |
|---|---|---|---|
| ESLint purity rules (no `Math.random`, `Date`, timers, `fetch`, `process`, async) for three packages, and the test that lints its own fixtures | `eslint.config.js`, `packages/engine/test/lint-ban.test.ts` | the pure crates cannot name those APIs (SURFACE §3) | 37 |
| ~25 module-scope registration hooks that exist only to break TS import cycles | `registerWorkHandler` ×16, `registerPromptAnswerer` ×4, `registerTargetingHooks`, `registerCastDriver`, `registerDeclarationCheck`, `registerGraveyardRedirect`, `registerDefaultWorkHandler` | Rust modules in one crate call each other directly (SURFACE §6.6) | 2, 3, 4 |
| Process-global registries re-synced on every `reduce`, `legalActions` and `viewFor` (`syncFusedScripts`), a global digest table, module-level re-entrancy flags | `subsystems/fuse.ts:948`, `catalog.ts:181`, `replacements.ts:383`, `subsystems/scorer.ts:225` | fused scripts composed on lookup from the state; flags in `EngineSink` (D17) | 2, 3, 8 |
| `registerAttackBar` (never registered), `StaticFlags.deftDuelist` (read nowhere), `Script.activate` (read nowhere), duplicated helpers (`creationNumber`, `compareText`, `indexRank`) | `restrictions.ts:78-95`, `script.ts:166-170, :334` | not ported; one copy of each helper survives part 31 | 2, 31 |
| Numbers outside `config.ts` (rule 9 gaps) | research A §4 | moved into `config.rs` | 1 |
| The registry generation step in `typecheck` and the test global setup; `missing-tests` as a command | `gen-registry.ts`, `_generated.ts`, `globalSetup.ts`, `missing-tests.ts` | `build.rs` (SURFACE §7.4) | 1, 37 |
| `loc` regenerated from TS source on every `gen` | `gen-loc.ts`, `test/loc.test.ts` | frozen data (D16) | 22, 37 |
| Dynamic-import seams for the engine and the store | `apps/server/src/match/engine.ts`, `src/index.ts` `STORE_EXPORT_CANDIDATES`/`loadStore`, `apps/web/src/game/engine.ts`/`engine.real.ts` | the server links the crates; the client loads one WASM module | 18, 19, 21 |
| The server's port layer (`Timers`, `Logger`, `Ids`, `Hashes`, `EnginePort`, the scripted fake engine) | `apps/server/src/api/ports.ts`, `test/fakes/engine.ts` | tokio time, tracing, plain functions, the real engine with test cards (D18) | 18, 19, 20 |
| Two WebSocket token paths nobody uses, the top-level nonce spelling, the `joinRoom` frame, the client's handler for a `legal` frame never sent | `wsServer.ts:124-135`, `protocol.ts:62, :504`, `apps/web/src/game/net.ts:222, 260, 514` | one path each, the one the clients use | 19, 21 |
| Routes nobody calls (`/api/auth/signup`, 503 in production; `/api/catalog/:version`) and the legacy queue body (`deckIndex`, no `mode`) | `api/auth.ts:766`, `api/catalog.ts:268`, `api/decks.ts:499-545` | dropped; `99-online-smoke.cy.ts` moves to `deckId` | 18, 21 |
| `render.yaml`'s start command deriving `CATALOG_VERSION` with Node, `--prod=false` and `tsx` at runtime | `scripts/catalog-version.mjs`, `render.yaml` | a Docker image; the version compiled in and checked at boot | 20 |
| The hand-maintained rulings index and its source-regex pins | `packages/engine/test/rulings.test.ts`, `rulings-coverage.ts` | `proven_in` front matter and `spec check` (§9) | 28 |
| Tests that pin SPEC wording; glossary text derived by regex from SPEC tables (#133) | `apps/web/src/cards/rules.test.ts`, `glossary.ts`, `audio/spec-rows.test.ts`, `conditionActive.test.ts:770-825` | structural checks only | 26, 28 |
| The Python ladder and its Node bridge | `ladder/`, `packages/ai/scripts/arena-bridge.ts` | the Rust arena (§8) | 29, 37 |
| The emote personas inside the AI package, imported on the main thread | `packages/ai/src/personas.ts`, `apps/web/src/practice/emotes.ts` | they are presentation: moved into the web | 21 |
| The training-box reservation in the harness (`only_labels`, `machine_cap` for training) | `bot/harness/plan.py:481-506` | training lanes run outside the harness (D10) | 39 |
| e2e on Electron | `ci.yml` | duplicates Chrome (§7) | 30 |

The parts' briefs list the smaller ones they meet (legacy aliases, test-only hooks), each with its
file and line.

## 11. Freeze rules for `main` while this runs

From the day part 1 starts until part 38 merges:

- `main` takes **no** change under `packages/engine`, `packages/cards` (catalog, scripts, patches),
  `packages/ai`, `packages/shared`, `packages/validator` or `apps/server`. Every such change would
  have to be ported twice. A person holds or relabels such issues; the night bot is told by keeping
  them out of its queue (no `bot:build`), not by a code change.
- Web-only, docs and bot changes may land on `main`. The orchestrator merges `main` into `staging`
  at the start of each wave; a conflict in `apps/web` goes to part 21's (or 36's) owner.
- A rules emergency on `main` (a crash in a live match) is fixed on `main` and in the Rust on
  `staging` in the same day, with a new golden trace from part 23's recorder if it changes behaviour.

## 12. Risks and abort criteria

| Risk | Sign | Answer |
|---|---|---|
| The type freeze (part 1) is wrong in a way every lane copies | part 31 finds the same seam in most notes | patch SURFACE.md and part 1's types, re-run Wave 2; the lanes' code stays |
| A JS semantic slips past §4.4 | golden divergence in Wave 3 | the divergence names seed, step and hash; `record.ts --dump-step` gives the TS side |
| Card lanes are too big for one session | a session stops with cards left | the next session for that part resumes at the first card whose `.rs` is missing (briefs say so) |
| Wave 3 needs a fourth iteration | fullsend's abort rule | stop, patch SURFACE.md from `spec-gaps.md`, rerun the bucket |
| `main` moves under the freeze | a port of a TS change is owed | §11; the change lands in both, with a trace |
| Vercel has no Rust toolchain | the web build fails on Vercel | `vercel.json`'s install step adds a minimal toolchain; `scripts/build-wasm.sh` fetches the `wasm-bindgen` binary (part 21) |
| WASM is blocked by the CSP | the client cannot instantiate the module | `'wasm-unsafe-eval'` in `script-src` in `vercel.json` (and the Cloudflare headers, held equal by their test) (part 21) |
| Devin's SWE-2 is free only until 2026-10-16 | the lanes' cost changes | `DEVIN_MODEL` is an env in the unit; a person decides the model after that date |
| The improve lane never promotes | `training/history/improve.jsonl` stays empty for weeks | expected at 85/100 (§8); part 40 reports measured rates so a person can decide |

Abort the run (roll `staging` back to the last wave's tag, `v0.3.0-wave-<n>`, which the orchestrator
pushes at each wave's end) if a fullsend failure-table row fires twice, or if nobody can say what
state `staging` is in.
