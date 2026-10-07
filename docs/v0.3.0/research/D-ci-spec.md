# Research D: CI, test inventory, SPEC structure, prose coupling, doc graph, agent docs, root tooling

Repo: `/home/user/JackiOh` at `91cc43c` (2026-10-06, shallow clone of 54 commits). Read-only research for v0.3.0 (issue #306). All paths are repo-relative unless absolute. Line counts are `wc -l`; "its" = lines matching `^\s*(it|test)(\.each(...))?(\.skip|...)?\(` (anchored, so `.test(` regex calls are not counted).

Issue #306 (open), scope relevant here: Rust rewrite of engine/cards/AI/server (merging #133 and #80), "rework how the spec is handled", an "obsidian like graph to speed up file reads", and "SHORTEN THE AMOUNT OF CI/CD ... a SUPER test pass that cover 99%+ of the code that we run once daily ... then only run the essential tests on the ci". It also says: ignore #256 and anything Cloudflare; main == staging.

Issue #133 (closed 2026-10-04 by jgoetzmann with the comment "moving to version 3"): its plan was **not carried out** (see section 4).

---

## 1. CI

### 1.1 `.github/workflows/ci.yml` (454 lines)

Triggers: `push` to `main`, every `pull_request`. Concurrency `ci-${{ github.ref }}`, `cancel-in-progress` on everything except `refs/heads/main`. Workflow env `CATALOG_VERSION: v0.2.0` (stale: render.yaml/patches are at v0.2.11). Every Node job starts with `actions/checkout@v4` + `./.github/actions/setup` (pnpm/action-setup@v4, setup-node from `.nvmrc` with pnpm cache, `pnpm install --frozen-lockfile`; measured 23–28 s per job).

| Job id | Display name | needs | timeout | Matrix / shard scheme | Command |
|---|---|---|---|---|---|
| `changes` | what the change touches | – | 5 | – | PR: `sh scripts/ci-scope.sh "$BASE" "$HEAD"` → `full=true/false`; push: `full=true` |
| `static` | lint, typecheck, catalog, rulings | changes | 10 | – | `pnpm lint`; `pnpm typecheck`; `pnpm validate:catalog` (step title says "268 cards, 49 tokens"); `pnpm exec tsx packages/cards/scripts/missing-tests.ts`; `pnpm rulings:coverage` |
| `unit` | unit (`<projects>` `<shard>`) | changes | 10 | 7 entries: `ai` 1/2, 2/2; `web` 1/4..4/4; `server shared validator` (no shard) | `pnpm exec vitest run --project <p>... [--shard=k/K] --exclude '**/gate-*.test.ts'` (engine and cards are NOT here; they run in `coverage`) |
| `fuzz` | fuzz (k/4) | changes | 10 | `shard: [1,2,3,4]`; `per = 1000 / job-total`; `JACKIOH_FUZZ_FROM=(k-1)*per+1`, `JACKIOH_FUZZ_SEEDS=per` | `pnpm fuzz` (= `vitest run --project cards fuzz`, runs `fuzz.test.ts` and `fuzz-handicap.test.ts`) |
| `coverage` | coverage (k/3) | changes | 10 | `shard: [1,2,3]` | `pnpm test:coverage --shard=k/3 --coverage.thresholds.lines=0 --reporter=blob --reporter=default`; uploads `.vitest/blob/` as `coverage-blob-k` |
| `checks` | **lint, typecheck, unit, fuzz, coverage** (REQUIRED) | changes, static, unit, fuzz, coverage | 10 | summary, `if: !cancelled()` | asserts every piece `success` (or `full == false`); downloads blobs; `pnpm exec vitest run --merge-reports --coverage --project engine --project cards` (holds the 90% floor from `vitest.config.ts`) |
| `ai-gate-shard` | ai gates (k/12) | changes | 15 | `shard: [1..12]`; env `JACKIOH_AI_GATE_SHARD=k/12`, `JACKIOH_AI_GATE_OUT=…/ai-gate-shards` | `pnpm ai:gate` (= `JACKIOH_AI_GATE=full vitest run --project ai gate-`); uploads `ai-gate-k` |
| `ai-gate` | **ai quality gates (full)** (REQUIRED) | changes, ai-gate-shard | 10 | summary | `pnpm ai:gate:merge ai-gate-shards` (= `tsx packages/ai/scripts/gate-merge.ts`) |
| `sql` | **sql invariants (real Postgres)** (REQUIRED) | changes | 10 | – | `docker pull postgres:16`; `sh apps/server/test/sql/run.sh` (no Node); `docker rm -f jackioh-pg-test` |
| `db` | **store contract (real Postgres)** (REQUIRED) | changes | 15 | – | `docker pull postgres:16`; `pnpm test:db` (`apps/server/test/db/run.sh`); `pnpm test:deploy` (`apps/server/test/deploy/rehearse.sh`) |
| `e2e-chrome-shard` | e2e chrome (k/8) | changes | 10 | `shard: [1..8]` | composite `./.github/actions/e2e-shard` (browser chrome) |
| `e2e-electron-shard` | e2e electron (k/8) | changes | 10 | `shard: [1..8]` | same, browser electron |
| `component` | e2e component (chrome) | changes | 10 | – | `pnpm --dir e2e exec cypress run --component --browser chrome` |
| `e2e-chrome` | **e2e (chrome)** (REQUIRED) | changes, e2e-chrome-shard, component | 5 | summary | asserts shards + component `success` |
| `e2e-electron` | **e2e (electron)** (REQUIRED) | changes, e2e-electron-shard | 5 | summary | asserts shards `success` |

`.github/actions/e2e-shard/action.yml` (85 lines): `pnpm build:e2e`; `nohup pnpm --dir apps/web exec vite preview --port 5173 --strictPort &` and `nohup pnpm --dir apps/server start &` with `E2E=1`; curl-waits on `http://localhost:5173` and `http://localhost:8787/api/catalog` (120 s); `specs=$(cd e2e && node scripts/shard-specs.mjs k K)`; `pnpm --dir e2e exec cypress run --browser <b> --spec "$specs"`; uploads screenshots + logs on failure. Each shard boots its own server because R144 reseeds fixtures at boot (spec 10 spends invite codes).

**Required checks.** `.harness/config.json` and `.squishy/config.json` `required_checks` list 7 names: the 6 bold names above plus `bot selftest`. (CLAUDE.md says "CI reports five required checks" and counts e2e as one; it is two.) Changing names needs a person: branch protection + `.harness/` + `.squishy/` are bot-forbidden paths.

Summary-job contract: each summary runs `if: ${{ !cancelled() }}` and accepts `skipped` only when `needs.changes.outputs.full == 'false'`; a failed `changes` leaves `full` empty → every summary fails.

**Measured durations** (main push run 37497560121 at `91cc43c`, all green; 51 jobs, **110.1 runner-minutes, 8.3 min wall**):

| Job | Time | | Job | Time |
|---|---|---|---|---|
| what the change touches | 0m08 | | coverage 1/3, 2/3, 3/3 | 1m09, 3m15, 2m13 |
| lint, typecheck, catalog, rulings | 1m50 (lint 23 s, typecheck 51 s, rest ≈2 s) | | ai gates 1..12/12 | 0m53–2m12 (sum ≈ 20 min) |
| unit (ai 1/2, 2/2) | 2m15, 0m57 | | ai quality gates (full) | 0m34 |
| unit (web 1..4/4) | 1m21, 2m17, 1m50, 2m14 | | sql invariants | 0m23 |
| unit (server shared validator) | 0m37 | | store contract (incl. deploy rehearsal 11 s) | 1m16 |
| fuzz 1..4/4 | 2m04, 2m04, 2m11, 1m44 (≈93 s of seeds each) | | e2e chrome 1..8/8 | 3m02–4m05 (sum ≈ 28 min) |
| lint, typecheck, unit, fuzz, coverage (summary) | 0m36 | | e2e electron 1..8/8 | 2m54–4m03 (sum ≈ 28 min) |
| e2e component (chrome) | 3m07 (cypress 151 s) | | e2e (chrome)/(electron) summaries | 0m04 each |

Runner-minute split of that run: e2e Chrome+Electron+component ≈ 59 min (54%), AI gates ≈ 21 min (19%), coverage ≈ 6.6 min, fuzz ≈ 8 min, unit ≈ 11.5 min, static ≈ 1.8 min, sql+db ≈ 1.7 min. Install overhead alone ≈ 51 × 25 s ≈ 21 runner-minutes.

Volume: 93 CI runs between 2026-10-05T20:00Z and 2026-10-06T21:00Z (57 pull_request, 36 push; 81 success, 6 failure, 6 cancelled) → on the order of 10,000 runner-minutes/day.

Historic numbers recorded in `ci.yml` comments (#73, #172): before the split, lint→coverage took 19 min, AI gates 15, each browser's e2e 14; web unit alone 4 min, then "7m51s per half"; ai unit alone 7m28s; one hard-vs-easy AI game can take 4–5 min (`gate:v2:hard-vs-easy:12`).

**Which jobs actually failed** (last 25 failing CI runs, 2026-10-05..06, summaries omitted): `unit (web k/4)` 9×, `lint, typecheck, catalog, rulings` 8×, `coverage (2/3)` 8× (always together with static: type errors in engine/cards), `e2e chrome`/`e2e electron` shards 10 runs (Chrome and Electron fail on the same shard index in 8 of them, e.g. `e2e chrome (6/8)` + `e2e electron (6/8)`), `e2e component` 4×, `ai gates` 2× (one on a main push, `ai gates (3/12)` at 19:43 on `f13cb03`), `sql invariants` 1×, `unit (ai 1/2)` 1×. Fuzz shards: 0 failures in that window.

### 1.2 `scripts/ci-scope.sh` (53 lines)

`sh scripts/ci-scope.sh <base> <head>` → prints `full=true|false`. `full=false` only when every file of `git diff --name-only base...head` matches the skip list; empty/unreadable diff or missing arg → `full=true`. Held in place by `apps/web/src/net/ci-scope.test.ts` (131 lines).

Skip list (exact `case` patterns, lines 40–44):
- `bot/*`, `.harness/*`, `.squishy/*`
- `CLAUDE.md`, `AGENTS.md`, `GEMINI.md`
- `.github/workflows/bot-commands.yml`, `bot-night.yml`, `bot-selftest.yml`
- `.github/workflows/squishy-run.yml`, `squishy-commands.yml`
- `.github/workflows/triage.yml`, `deploy-watch.yml`, `ci-duration.yml`

Not on the list (because tests read them): `SPEC.md`, `BUILD.md`, `docs/` (header names `rulings.test.ts`, `spec-rows.test.ts`, `rules.test.ts`). Note `.claude/` and `ladder/` are not on the list either.

### 1.3 `scripts/vercel-ignore.sh` (52 lines)

Vercel's `ignoreCommand` (in `vercel.json`). Exit 1 = build. Rules: (1) commit message containing `[vercel]` builds on any branch; (2) non-main, non-production → skip; (3) main builds unless every file changed since `VERCEL_GIT_PREVIOUS_SHA` (must be an ancestor) is on the skip list. Skip list (lines 95–101): `bot/* .harness/* .squishy/* .github/* docs/* reviews/* e2e/* apps/server/* scripts/*`, `render.yaml`, `apps/web/*.test.ts(x)`, `packages/*.test.ts`, `apps/web/src/test/*`, `apps/web/README.md`, `packages/{engine,cards,ai,validator,shared}/test/*`, `apps/web/scripts/*`, `packages/{engine,cards,ai}/scripts/*`, `packages/*/README.md`, root docs (`CLAUDE.md AGENTS.md GEMINI.md README.md SPEC.md BUILD.md REVIEW.md ARCHITECTURE-CCG.md JackiOh_*.md`). Held by `apps/web/src/net/vercel-ignore.test.ts` (216 lines). A Rust rewrite changes what the web bundle reads (a WASM crate instead of `packages/*/src`), so this list must be rewritten.

### 1.4 `e2e/scripts/shard-specs.mjs` (58 lines)

`node scripts/shard-specs.mjs <k> <K>` from `e2e/`: reads `cypress/e2e/*.cy.ts`, greedy longest-first into the lightest shard, returns shard k's specs sorted, comma-joined. `WEIGHTS` (seconds, CI run 36973335249 at e762937; 29–35 estimated): 01:67 02:124 03:30 04:46 05:20 06:11 07:38 08:170 09:32 10:4 11:26 12:43 13:49 14:22 15:17 16:28 17:55 18:51 19:28 20:28 21:20 22:28 23:11 24:26 25:36 26:11 27:4 28:25 29:40 30:25 31:25 32:15 33:15 34:20 35:20 99:1; `DEFAULT_WEIGHT = 30`. Sum ≈ 1,190 s per browser (≈20 min of spec time). Note: `99-online-smoke.cy.ts` is in the directory and gets sharded too (it self-skips unless enabled).

### 1.5 `.github/workflows/ci-duration.yml` (118 lines)

`workflow_run` on `["CI", "bot selftest", "deploy watch"]` completed. Env `TARGET_MINUTES: "5"`, `ALERT_MINUTES: "7"`, `BOT_LOGIN: jgoetzmann-bot`. Reads job timings via `gh api .../runs/$RUN_ID/jobs`, lists jobs > 7 min (success/failure/timed_out only), opens or comments on one issue titled `CI: a job ran over 7 minutes`, assigns it to the bot with `bot:build` (not for forks). Never checks out code. Will need its workflow list/semantics revisited if CI becomes "essential per PR + daily super pass" (a daily job would legitimately exceed 7 min).

### 1.6 `.github/workflows/bot-selftest.yml` (55 lines) — the 7th required name, `bot selftest`

On every PR and push to main; job `selftest`, timeout 10, measured 2m06. Steps: Python 3.12 and 3.13 `python3.X -m unittest discover -s tests -t .` in `bot/` (41 test files; `bot/` is 28,785 lines of Python); `python3.13 -m harness --help`, `harness window`, `HARNESS_HOME=.squishy harness window`; actionlint 1.7.7 (docker) over `bot-night.yml bot-commands.yml bot-selftest.yml triage.yml bot-status.yml squishy-run.yml squishy-commands.yml`. Out of scope for the Rust rewrite but bot code hard-codes `pnpm`/`vitest` in `bot/harness/easy.py` (7 hits), `bot/harness/work.py` (4) and several `bot/tests/*`; `bot-night.yml:276` and `squishy-run.yml:220` set up pnpm. `.harness/config.json` and `.squishy/config.json` `gates` run: `pnpm lint`, `pnpm typecheck`, `pnpm validate:catalog`, `pnpm exec tsx packages/cards/scripts/missing-tests.ts`, `pnpm rulings:coverage`, `pnpm exec vitest run --changed origin/main --exclude '**/gate-*.test.ts' --exclude '**/fuzz*.test.ts'`; `install` runs `pnpm install --frozen-lockfile`. All of these are bot-forbidden paths (a person must change them).

Note: `ladder/` (issue #55 AI training ladder, Python, 1,473 lines, `ladder/tests/` 5 files) is run by no workflow.

### 1.7 `.github/workflows/deploy-watch.yml` (301 lines)

Push to main + dispatch. Env `SERVER_URL: https://jackioh-server.onrender.com`, `WINDOW_SECONDS: "270"`. Jobs: `window-1` (reads `CATALOG_VERSION` from `render.yaml`, asks Render to deploy this commit via `RENDER_API_KEY` API or `RENDER_DEPLOY_HOOK_URL`, polls with `./.github/actions/poll-live`), `render` (follows the deploy via Bounceapp/render-action), `window-2..4` (chained polls, 5 min each), then `report` (opens/comments the issue "Render: the live server is not serving main's catalog") or `close`. Polls `GET /api/catalog` for version + `x-deployed-commit`. With a Rust server the `/api/catalog` route and header must be preserved or this file changed.

### 1.8 `.github/workflows/patches-ship.yml` (91 lines)

Push to main touching `packages/cards/patches/pending/**` (+ dispatch). Checks out main with full history and `BOT_GITHUB_TOKEN`, runs `pnpm --filter @jackioh/cards run patches ship` (TS tooling, `packages/cards/scripts/patches.ts`), opens `patches/ship-<sha>` PR with auto-merge squash, closes superseded promotions. Depends on TS tooling in `packages/cards/scripts/` (9 files, 2,420 lines).

### 1.9 `.github/workflows/promote-production.yml` (205 lines) — summary only (Cloudflare out of scope)

Hourly cron `7 * * * *`, `issue_comment` (/hold /resume /delay /fast-forward), `workflow_run` after green CI on main, dispatch. Job `promote` merges a green main commit into `production` via PR (logic in `scripts/promote-production.sh`, 482 lines, tested by `apps/web/src/net/promote-production.test.ts`, 923 lines); job `deploy` builds the web app and `wrangler deploy`s to Cloudflare. Per #306 this is to be left alone / set up later.

Other workflows (not CI gates): `bot-commands.yml`, `bot-night.yml`, `bot-status.yml`, `squishy-commands.yml`, `squishy-run.yml`, `triage.yml`.

---

## 2. Test inventory

### 2.1 `vitest.config.ts` (20 lines)

`projects: ["packages/*", "apps/server", "apps/web"]`, `passWithNoTests`, coverage `provider: "v8"`, `include: ["**/src/**"]`, `thresholds: { lines: 90 }`. Per-project configs: `packages/{shared,engine,validator}/vitest.config.ts` and `apps/server/vitest.config.ts` (`include: ["test/**/*.test.ts"]`); `packages/cards/vitest.config.ts` (+ `globalSetup: ["test/globalSetup.ts"]`, regenerates `src/scripts/_generated.ts`); `packages/ai/vitest.config.ts` (+ `setupFiles: ["test/setup.ts"]`); `apps/web/vitest.config.ts` (react plugin, `environment: "jsdom"`, `setupFiles: ["./src/test/setup.ts"]`, `include: ["src/**/*.test.{ts,tsx}"]`). Separate: `apps/server/test/db/vitest.config.ts` (the Postgres `*.spec.ts` run by `test:db`).

### 2.2 Per project

| Project | Test files | its | Test lines | Src files / lines (non-test) | Notes |
|---|---|---|---|---|---|
| shared (`packages/shared/test`) | 4 | 74 | 1,044 | 9 / 2,477 | codes, emotes, events (reads `src/events.ts` source), stats |
| engine (`packages/engine/test`) | 169 | 2,706 | 62,390 | 147 / 36,999 | 30 non-test helper files (`fixtures/`, incl. `fixtures/lint/`); `scripts/` 140 lines |
| cards (`packages/cards/test`, recursive) | 366 | 5,174 | 90,068 | 324 / 17,224 (+ `catalog.json` 221,758 B, `flavour.json` 28,129 B, `patches/` 4.2 MB) | `scripts/` 2,420 lines |
| ai (`packages/ai/test`) | 32 | 314 | 6,757 | 21 / 4,262 | `scripts/` 1,038 lines |
| validator (`packages/validator/test`) | 2 | 40 | 554 | 2 / 540 | |
| server (`apps/server/test`) | 49 `*.test.ts` (+4 `*.spec.ts` Postgres) | 724 (+33) | 19,124 (+869) | 48 / 19,704 | 16 SQL evidence files, 6,123 lines; 26 migrations |
| web (`apps/web/src/**/*.test.ts(x)`) | 233 | 4,324 | 85,786 | 325 / 74,685 | `scripts/` 2,946 lines |
| **Total vitest** | **855** | **≈13,356** | **≈265,700** | | |

Engine groups: `effects-*.test.ts` 50 files / 632 its / 14,393 lines; `rulings*.test.ts` 4 files / 699 its / 10,171 lines (`rulings.test.ts` 4,565, `rulings-a` 1,385, `rulings-b` 1,682, `rulings-c` 2,539); combat 7/77; property/replay (`combat.property`, `control-change.property`, `hotseat.smoke`, `replay`, `replay-scripted`, `generation-replay`, `play-pipeline-b-replay`) 7/18/1,516; fuse/transform/recruit variants 5/58; `lint-ban.test.ts` 1/7 (runs ESLint on `test/fixtures/lint`); other 95/1,215/32,153.

Cards groups: Core card tests `test/NNN-*.test.ts` 102 files / 1,301 its; Classic `test/classic/` 91 / 1,625; Classic+ `test/classic-plus/` 116 / 1,448; tokens `t-*.test.ts` 6 / 96; catalog data (`catalog`, `registry`, `references`, `radiant-standard`, `card-text`, `flavour`, `params`) 7 / 85; patch tooling (`patches`, `patches-ship`, `versions`, `loc`) 4 / 47; fuzz 2 / 4 (but 1,000 games each); cross-card behaviour (`paused-sequences`, `hidden-information`, `preview`, `re-entry`, …) 38 / 568 / 18,070.

AI: `gate-random`, `gate-greedy`, `gate-hard-easy`, `gate-perf` (+`_shard.ts`); 6 `redact-*` hidden-info tests; `decide`, `search`, `lethal`, `evaluate*`, `match*`, `observe*`, `personas`, `puzzles`, `shadowBan`, `answer-key`, `arena-bridge`, `dev-run`, `tutorial-tier`, …

Server dirs: `api/` 23 files / 485 its / 12,059 lines (`account auth catalog client-address code-input-parity codes collection cors decks e2e game-records queue ranked rate-limit redeem-feedback rematch results retention series-rules series settings stats tutorial`); `match/` 11 / 153 / 5,520 (`actor aim clock dealt-deck engine.real glitch last-boards recovery rooms series-recovery ws-server`); `ranked/` 3 / 32; `db/` 10 memory tests / 44 + 4 Postgres specs / 33 (`contract.postgres.spec.ts`, `postgres.spec.ts`, `redeem-race.postgres.spec.ts`, `seed-catalog.spec.ts`); root 2 (`env-deployed-commit`, `validator-single-source`). SQL: `apps/server/test/sql/00..14_*.sql` + `run.sh` (151 lines); `test/db/run.sh` (74); `test/deploy/rehearse.sh` (205).

Web dirs (files / its / lines): audio 22/575/10,622; auth 6/101/1,808; cards 18/335/5,929 (+art 5/94, inspect 3/104, wheel 2/8); emotes 6/63; fx 25/562/9,715; game 41/694/13,980 (+aim 1/8, deckbuilder 10/296/4,806, showcase 5/54); haptics 1/6; net 13/204/4,594; patches 6/64; practice 9/91/2,650; rank 1/4; routes 26/561/13,071 (+dev 1/25); settings 4/52; stats 4/27; test 1/14 (`qa-v020`); test/ux 9/233/4,392; tutorial 9/112 (+scripts 4/30); `src/wording.test.ts` 1/7.
- 69 web test files import `@jackioh/engine|cards|ai|validator` (would need the WASM bindings); 164 import only `@jackioh/shared` or no package.
- 45 web src (non-test) files import those packages (incl. `/config`): engine via `src/game/engine.real.ts`, `src/practice/core.ts`, `src/tutorial/harness.ts`; `@jackioh/engine/config` in 21 files; `@jackioh/validator` in 20 (deckbuilder); `@jackioh/cards` in 4 (`CATALOG`, `registerAll`, `CATALOG_VERSION`, `CardFlavour`); `@jackioh/shared` in 143.

Largest test files: `packages/engine/test/rulings.test.ts` 4,565; `rulings-c.test.ts` 2,539; `packages/cards/test/paused-sequences.test.ts` 2,175; `apps/web/src/routes/login-flows.test.tsx` 2,162; `apps/server/test/match/actor.test.ts` 1,991; `apps/web/src/routes/practice.test.tsx` 1,945; `apps/web/src/fx/cues.test.ts` 1,867; `apps/web/src/audio/engine.test.ts` 1,748.

Slow suites:
- **Fuzz**: `packages/cards/test/fuzz.test.ts` (555 lines; `WAVE_SEEDS = 1000`, `SWEEP_SEEDS = 100`) and `fuzz-handicap.test.ts` (154). `pnpm fuzz` ≈ 100 s locally, ≈ 4 × 93 s in CI; under `pnpm test`/`test:coverage` they run 100 seeds. `docs/ADDING_CARDS.md:134` says `pnpm vitest run --project cards` takes ~10 minutes (it runs the full 1000-seed wave because the npm script is not `test`).
- **AI gates**: `packages/ai/src/gate.ts` `AI_GATE = { seedSeries: "gate:v3", smokeSeeds: 20, fullSeeds: { "ai-vs-random": 100, "ai-vs-greedy": 50, "hard-vs-easy": 50 }, briefRate: {0.95, 0.7, 0.8}, measuredRate: {0.945, 0.68, 0.913} }`; ≈ 20 runner-min in 12 shards; flaky enough to fail a main push.
- **Coverage**: `pnpm test:coverage` (engine + cards with v8) ≈ 6.6 runner-min in 3 shards; the 90% floor is checked only on the merged report.
- **web unit**: ≈ 7.7 runner-min in 4 shards.

### 2.3 E2E specs (`e2e/cypress/e2e/`, 12,861 lines total; support 4,764 lines; config `e2e/cypress.config.ts`)

"Server" = needs `E2E=1 pnpm --dir apps/server dev` (per spec header / `e2e/README.md` "Which spec needs which milestone"); "stub" = `cy.intercept` stubs, no server. Weight = `shard-specs.mjs` seconds.

| Spec | Lines | its | Server | W | Covers |
|---|---|---|---|---|---|
| 01-hotseat-full-game | 214 | 1 | no | 67 | seeded hotseat game to completion; replay hash via `cy.task` fold (uses `e2e/support/tasks/replay-runner.ts` → TS engine) |
| 02-prompts | 542 | 10 | no | 124 | every choice picker rendered and answered |
| 03-trap-opponent-turn | 347 | 2 | no | 30 | trap fires on other player's turn; face-up modifier on both seats |
| 04-combat | 241 | 3 | no | 46 | Taunt, Defense Position, First Strike, Divine Shield |
| 05-reconnect | 572 | 1 | yes | 20 | networked game reloaded mid-prompt (+clock) |
| 06-room-code | 441 | 1 | yes | 11 | networked match browser vs Node WS client (M6 gate) |
| 07-my-pawn-ai | 350 | 1 | no (CLAUDE.md wrongly lists it as networked) | 38 | My Pawn cancels lethal, AI plays rest of turn |
| 08-turn-cap-draw | 145 | 1 | no | 170 | turn cap (60 player-turns) ends as a draw |
| 09-deckbuilder | 479 | 6 | yes | 32 | L1–L6 in builder and at queue |
| 10-invite-gate | 458 | 3 | yes | 4 | pending account, invite codes (spends codes → fresh server) |
| 11-radiant | 313 | 1 | no | 26 | Radiant conversion on hand and field |
| 12-rotation-and-swaps | 529 | 1 | no | 43 | Silly Silas rotation, Pocket Chaos swap |
| 13-practice-vs-ai | 607 | 4 | no | 49 | /practice vs AI worker, replay with handicaps |
| 14-landing-and-sign-in | 830 | 31 | stub | 22 | landing, code field, rate-limit feedback, email links, reset |
| 15-audio | 281 | 4 | no | 17 | audio layer log on hotseat |
| 16-drag-to-play | 241 | 5 | no | 28 | drag gestures |
| 17-card-showcase-and-hovers | 656 | 6 | no | 55 | opponent's play showcase, log, pile browser |
| 18-deck-workshop | 408 | 5 | yes | 51 | drafts, offline autosave, deck codes, trio conflicts |
| 19-queue-modes-and-series | 752 | 4 | yes | 28 | Bo1, All Random, Conquest, rooms |
| 20-mulligan-concede-draw | 619 | 4 | yes | 28 | concurrent mulligan, concede, draw offer |
| 21-radiant-marks | 151 | 3 | no | 20 | computed value, reference, gold Radiant diff |
| 22-tutorial-lesson-one | 236 | 1 | no | 28 | tutorial lesson 1 to a win |
| 23-tutorial-path | 466 | 5 | no | 11 | lesson path progress, Exit, fixed deck (uses `lessons-runner.ts` → TS) |
| 24-library-browse | 265 | 2 | no | 26 | own library list (R310–R313) |
| 25-overflow-animations | 526 | 6 | no | 36 | fatigue/hand-full/library-full notices |
| 26-trio-codes | 183 | 2 | yes | 11 | trio code copy/import/caps |
| 27-account-tutorial-and-email-link | 221 | 4 | yes | 4 | tutorial progress on account |
| 28-patch-011-ui | 252 | 4 | no | 25 | v0.1.1 board/homescreen marks |
| 29-animated-trap | 168 | 2 | no | 40 | Animated Field Trap becomes a Unit |
| 30-activate | 142 | 1 | no | 25 | Activate controls |
| 31-counter-opponent-turn | 138 | 1 | no | 25 | Counter on opponent's turn |
| 32-tribute-full-board | 95 | 1 | no | 15 | Tribute on full row |
| 33-almanac | 126 | 1 | no | 15 | Card Almanac page |
| 34-stats | 229 | 1 | stub | 20 | public Statistics page |
| 35-settings-dialog | 249 | 4 | yes | 20 | settings tabs, resets, account copy |
| 99-online-smoke | 389 | 12 | live deployed stack | 1 | skipped unless enabled |

Server-needing: 05, 06, 09, 10, 18, 19, 20, 26, 27, 35 (10 specs). CLAUDE.md's list "05, 06, 07, 09, 10, 18, 19, 20, 26, 27, 35" is wrong about 07.

Component specs (`e2e/cypress/component/`, 4,472 lines, no server, Chrome only, Vite dev server on 5273, mount `apps/web/src` directly): `audio-recipes` (457 lines/8 its), `audio-toggle` (55/1), `board-layout` (353/2; BUILD M5-T1 pixel acceptance), `card-faces` (685/14), `deckbuilder-layout` (420/12), `emotes-layout` (267/8), `fx-layer` (255/1), `keyword-visuals` (261/5), `landing-and-code-field` (303/8), `mobile-ux` (970/34), `practice-table` (138/1), `radiant-marks` (144/6), `stack-wheel-and-sweeps` (164/3).

### 2.4 Classification

| Group | Class | Why |
|---|---|---|
| static: lint, typecheck, `validate:catalog`, `missing-tests`, `rulings:coverage` | ESSENTIAL | 1m50 total, catches the most failures (8 of 25 failing runs) |
| engine unit tests (effects, combat, prompts, turn, viewFor, hidden info, replay determinism, `rulings-a/b/c`) | ESSENTIAL (port to Rust `cargo test`) | the core rules; run in < 3 min today inside coverage shards |
| per-card tests (309 card files + 6 token files) | ESSENTIAL (port; BUILD's must-pass tables are the oracle) | card regressions; cheap per file |
| cards cross-card behaviour tests (`paused-sequences`, `hidden-information`, `re-entry`, `preview`, …) | ESSENTIAL (port) | rules interactions |
| fuzz smoke (100 seeds both files) | ESSENTIAL | replay/termination smoke; today inside `pnpm test`/coverage |
| fuzz full 1000 seeds × 2 (`pnpm fuzz`) | DAILY | ≈ 8 runner-min; 0 failures in the sampled window; seeds are independent so a daily sweep finds the same bugs |
| ai decision/redaction tests (`packages/ai/test` minus `gate-*`) | ESSENTIAL (port) | hidden-info guarantees (R185) and decision correctness |
| ai quality gates full (`pnpm ai:gate`, 12 shards) | DAILY | ≈ 21 runner-min, statistical, flaky on main; the smoke size (20 games) can stay per-PR or go daily |
| validator, shared | ESSENTIAL (port; shared types become WASM/serde types) | tiny |
| server api + match (memory store) | ESSENTIAL (port) | auth, protocol, persistence contract, 37 s today |
| `sql` invariants (`apps/server/test/sql`) | ESSENTIAL when `apps/server/src/db/migrations/` changes, else DAILY | 23 s; schema/RLS only change with migrations |
| `db` store contract + `test:deploy` | ESSENTIAL when server/db changes, else DAILY | 1m16; Docker |
| web unit: game/, routes/, practice/, tutorial/, net auth, deckbuilder, settings, stats | ESSENTIAL | the only TS code left; user-facing flows |
| web unit: audio/ (22 files, 575 its), fx/ (25, 562), cards visuals, emotes, haptics | DAILY (or ESSENTIAL only when `apps/web/src/{audio,fx}` change) | cosmetic layers; large |
| web tests that read CSS source and regex it (28 files: `game/{countered,animations,Clock,Hand,emotes,overflow,position-switch,PromptE18,Board}.test.*`, `game/deckbuilder/browse.test.tsx`, `test/qa-v020.test.tsx`, `test/ux/{settings,board-ux}.test.tsx`, `fx/{css,sprites}.test.ts`, `cards/{CardFace,CardMarks,glitch,cardState,radiantDiff,foil,keywordVisuals,quest}.test.*`, `settings/wiring.test.tsx`, `routes/landing.test.tsx`, `patches/CardHistory.test.tsx`, `emotes/{ui,PortraitPicker}.test.tsx`) | DAILY or DELETE the CSS-text assertions | source-text pins, not behaviour; jsdom has no layout, component specs measure the real thing |
| web tooling tests: `net/ci-scope.test.ts`, `net/vercel-ignore.test.ts` | KEEP but rewrite with the scripts | they test repo tooling that changes in the CI trim |
| web tooling tests: `net/promote-production.test.ts` (923), `net/cloudflare-config.test.ts`, `net/cloudflare-serve.test.ts`, `net/deploy-routes.test.ts`, `net/env-production.test.ts` | DAILY or leave untouched (Cloudflare out of scope per #306) | not web behaviour |
| asset tests: `audio/gen-voice.test.ts`, `audio/voice-assets.test.ts`, `audio/music-assets.test.ts`, `audio/voiceData.test.ts`, `audio/voice-lines.test.ts`, `cards/art/convention.test.ts` | DAILY (or ESSENTIAL when `apps/web/public/audio` / `card-audio.json5` / catalog change) | asset completeness; break on every new card from Linux (ADDING_CARDS §7) |
| `apps/web/src/audio/spec-rows.test.ts` (104 lines) | DELETE | duplicates `rulings.test.ts`'s completeness test and spawns `rulings-coverage.ts` itself; pins SPEC heading order |
| `packages/engine/test/conditionActive.test.ts:784–825` (R195 prose block) | DELETE | pins SPEC prose and the index's file list (#133 example 2) |
| `packages/engine/test/rulings.test.ts` source-regex pins (23 `sourceOf(...)` uses, e.g. lines 1224–1600 against `apps/server/src/**` and migrations) | DELETE with the TS engine | they regex server source/SQL comments, e.g. `toMatch(/SPEC §11 R111: the launch quantity/)` at line 1529 |
| `apps/web/src/cards/rules.test.ts` SPEC-table parsing (`ruleColumn` l.40–57, `SPEC_ROW_NAME` l.71, `PAIRED_HALF` l.89, `CRY_RULING_BEFORE_R500`, `SHORT_REMINDER_MAX_WORDS`) | DELETE/replace (#133 example 3) | parses SPEC.md prose tables at test time |
| e2e smoke per PR: 01 (hotseat + replay), 06 (networked match), 10 (auth/invite), 13 (practice vs AI), 19 (queue/series) on Chrome | ESSENTIAL | one smoke per user-facing flow: ≈ 167 s of spec time |
| e2e full 01–35 on Chrome | DAILY | ≈ 28 runner-min today |
| e2e on Electron (8 shards) | DELETE (or weekly) | duplicates Chrome: same shards fail together in 8 of 10 failing runs; `ci.yml:401` notes Cypress 16 deprecates Electron |
| component specs (13 files) | DAILY (or ESSENTIAL when `apps/web/src/**/*.css` changes) | 3 min; pixel layout |
| coverage merge (90% floor) | DAILY | measurement, not a regression catcher |
| `packages/engine/test/lint-ban.test.ts` | DELETE with TS engine (Clippy/deny rules replace it) | tests the ESLint purity rule |
| `packages/cards/test/patches*.test.ts`, `versions`, `loc` | KEEP with whichever language owns the patch tooling | catalog history integrity |
| `ladder/tests` (Python) | not run anywhere | decide in plan |

---

## 3. SPEC.md, BUILD.md, REVIEW.md structure

### 3.1 SPEC.md — 1,986 lines, 853,936 bytes (very long table rows)

| Section | Lines | Bytes |
|---|---|---|
| (title) | 1–4 | |
| ## 1. Overview | 5–20 | 2,293 |
| ## 2. Core systems (### 2.1 Setup 25–32, 2.2 Turn loop 33–56, 2.3 Mana 57–64, 2.4 Drawing 65–71, 2.5 Ending 72–86, 2.6 Deckbuilding 87–90) | 21–90 | ≈10,800 |
| ## 3. Zones and board layout (3.1 105–112, 3.2 113–123) | 91–123 | ≈6,800 |
| ## 4. Combat (4.1 128–136, 4.2 137–146, 4.3 147–164, 4.4 165–183, 4.5 184–195) | 124–195 | ≈10,800 |
| ## 5. Card anatomy and card types (5.1 216–229, 5.2 Radiant 230–242, 5.3 Catalog data decisions 243–276) | 196–276 | ≈14,000 |
| ## 6. Keyword glossary (6.1 Unit keywords 283–322, 6.2 Triggers 323–344, 6.3 Actions and verbs 345–394) | 277–394 | ≈36,600 |
| ## 7. Tokens | 395–466 | 15,846 |
| ## 8. Complete card catalog (8.1 #1–20 495–519, 8.2 #21–50 520–554, 8.3 #51–80 555–591, 8.4 #81–95 592–614, 8.5 #96–100 615–624, 8.6 Classic 625–721, 8.7 Classic+ 722–844) | 467–844 | ≈182,500 |
| ## 9. Architecture (9.1 Trust 849, 9.2 Topology 861, 9.3 Engine requirements 876–884, 9.4 Accounts 885, 9.5 Matchmaking 896, 9.6 Build order 907, 9.7 Suggested repository layout 922–937, 9.8 Abuse 938, 9.9 Practice AI 954–981, 9.10 Tutorial 982, 9.11 Card statistics 999, 9.12 Ranked ladder 1010–1024) | 845–1024 | ≈42,000 |
| ## 10. Engine implementation guide (10.1 State model 1029–1105, 10.2 Actions, 10.3 Events, 10.4 Layers, 10.5 Playing a card, 10.6 Prompts, 10.7 Randomness, 10.8 viewFor 1168–1186, 10.9 Card scripts and tests 1187–1192, 10.10 Client rendering 1193–1218, 10.11 Audio 1219–1296) | 1025–1296 | ≈50,500 |
| ## 11. Open rules questions and recommended rulings | 1297–1986 | **481,509 (56% of the file)** |

§8 card table header (lines 497/522/557/594): `| # | Name | Rarity | Cost | Type, tags | Stats | Base effect | Radiant effect | Engine |`.

§11 layout: lines 1299–1422 are 88 non-table provenance paragraphs ("**R91 to R170 were added on 2026-09-18** …", "**R745 is the game log's history …**"); the table header is at line 1424–1425:

```
| # | Topic | Recommended ruling | Cards affected |
| --- | --- | --- | --- |
```

Three example rows (verbatim, row 1 truncated by me after the third cell's end):

```
| R1 | When does Cry fire? (decide) | When the card is played — from hand, or, since patch v0.2.0, from a graveyard while a permission lets its player play from there (§6.3 Play) — or cast by an effect (R70). A summon fires it only when it names the card it summons (C+ #19 League of Losers summoning C+ #19.3 Mid Loser, R411); copies, Recruit, Reborn, Transform, Flicker and every other summon, a token's included, never fire it. "Trigger a Cry" (§6.3) runs one by effect, on the field or in a graveyard, without the card being played. A countered card is never played and fires none (§10.5) | #12 would fill the board for 2 mana otherwise; #3, #22, #61, #69; C #28, C #54, C #90, C+ #19, C+ #19.3; R70, R411 |
| R111 | The launch grant | Becoming `active` grants one copy of every non-token card, written by a trigger on the `pending → active` transition and idempotent, so a repeated redemption cannot double a collection | §9.4, §9.5 |
| R195 | When a card glows yellow (`conditionActive`) | Hearthstone lights a playable card yellow when its special condition is met; ... | §10.8, §10.9, #10, #53, #68, #71, #93, C #22, C #36, C #40 |
```

Counts: **561 R-rows** (`^| R(\d+) |`), unique, ascending, **R1..R762**, 201 unused numbers (blocks reserved by workstreams leave gaps). "decide" rows still open: R1, R4, R5, R14, R26, R39 (R2 decided by R389). The 4th column mixes card refs (`#12`, `C #28`, `C+ #19.3`), section refs (`§9.4`) and other R-ids.

### 3.2 `packages/engine/test/rulings.test.ts` (4,565 lines)

- Imports only `node:fs`, vitest and `../src/config` (by design: stays green while modules change).
- Helpers: `sourceOf(file)` (reads any file relative to the test dir, missing → ""), `rulingTitles(file)` (`/\bit\(\s*"(R\d+[^"]*)"/g`), `sqlHeadings(file)` (`\echo '### … ###'` / `=== … ===` headings only), `sqlProofs`, `proofsFor(file,row)` (sibling engine file: title must lead with `R<n>`; a file with `/` in its path may also name the row mid-title or on a `describe`; `.sql` via headings), `provenIn(row, ...files)` (expects a non-empty proof list per file), `serverConstant(file, name)` (regexes `export const NAME = …;`).
- Path constants for proofs outside the package (≈30: `SERVER_CONFIG`, `SERVER_SQL`, `SERVER_ACTOR`, `SERVER_*_TEST`, `CARDS_*_TEST`, `WEB_*_TEST`, …).
- `describe("SPEC §11 rulings, every row (BUILD M3 gate, REVIEW B4)")` at line 419: **561** `it("R<n> …")` (one per row, ascending, gaps included), 552 `provenIn(` calls; "decide"/numeric rows also `expect(config.X).toBe(...)`.
- `describe("SPEC §11 index completeness")` at line 4527: (a) SQL heading-only crediting (R104–R112 vs R107–R109 disclaimers); (b) line 4555–4561: reads `../../../SPEC.md`, `rows = [...spec.matchAll(/^\| R(\d+) \|/gm)]`, `named = rulingTitles("rulings.test.ts")`, `expect(named).toEqual(rows)`.
- Where proofs live (it-title counts outside the index): engine 1,506 (297 distinct R), cards 2,926 (328), ai 120 (20), shared 70 (7), validator 22 (6), server 530 (88), web 1,670 (143). Total `it("R…")` titles repo-wide 7,405; all 561 rows have at least one. **63 rows are proven only in `apps/web` tests** (they survive a Rust rewrite as TS); 4 rows are proven only by the index's own config asserts.

### 3.3 `packages/engine/scripts/rulings-coverage.ts` (140 lines; `pnpm rulings:coverage`)

1. `specRows()`: `SPEC.md` `^\| R(\d+) \|` → sorted unique ids.
2. `namedRows()`: `/\bit(?:\.\w+)?\(\s*["'`]R(\d+)\b/g` over `packages/engine/test/*.test.ts` and `packages/cards/test/*.test.ts` — **top level only** (`readdirSync`, not recursive: `test/classic/` and `test/classic-plus/` are not scanned).
3. `citedAnywhere()`: `/\bR(\d+)\b/g` over every `.ts/.tsx/.sql` under `packages/` and `apps/` (skips node_modules, dist, coverage, .git, artifacts).
4. Prints MISSING (row with no named test), UNKNOWN (test names a non-row), ONLY IN CODE (cited id with no row); exits 1 if any. Currently clean: 561 distinct R-ids cited in code = exactly the 561 rows; 28,716 R-citations in 1,759 files.

### 3.4 BUILD.md — 920 lines, 222,729 bytes

| Section | Lines | Bytes | Status after a Rust rewrite |
|---|---|---|---|
| ## 0. How to work this file | 5–15 | 1,952 | rewrite (names TS stack, ESLint ban, Cloudflare DO) |
| ## 1. Repository layout | 16–93 | 5,195 | obsolete (TS file tree: `packages/engine/src/reduce.ts`, …) |
| ## 2. Constants (`packages/engine/src/config.ts`) | 94–170 | 8,297 | keep the 63-row table as data; path changes |
| ### M1 Engine core | 173–245 | 9,804 | done (reviews/2026-09-17-m1.md); acceptance cases are a test oracle |
| ### M2 Combat | 246–279 | 4,291 | done (m2 review) |
| ### M3 Effects, triggers, prompts, layers, view | 280–319 | 7,914 | done (m3, m3-m4-gates) |
| ### M4 Catalog and card scripts | 320–463 | 24,900 | done; contains the Core per-card must-pass table (header line 351, 105 rows) — keep as test oracle |
| ### M5 Hotseat client and animations | 464–554 | 15,352 | done; animation table (line 481, 65 event rows) stays relevant (web stays TS) |
| ### M6 Server … | 555–574 | 2,933 | done (m6 review); file names obsolete |
| ### M7 Clock, disconnects, results, matchmaking | 575–588 | 1,951 | done |
| ### M8 End-to-end | 589–624 | 9,805 | done (m8 review); e2e table (line 593, 35 specs) stays relevant |
| ### M9 Patch v0.2.0 (issue #40) | 625–900 | 128,028 | done (v0.2.0 shipped; reviews/2026-10-02-v0.2.0-part-a.md PASS); Classic table (line 684, ≈91 rows) and Classic+ table (line 779, ≈118 rows) are test oracles; 13 tasks M9-T1..T13 |
| ## 4. Test strategy summary | 901–910 | 796 | rewrite (vitest, coverage floor, nightly run of 01 over 20 seeds) |
| ## 5. Definition of done | 911–920 | 1,144 | rewrite (`pnpm lint && pnpm typecheck && pnpm test && pnpm test:e2e`, `missing-tests.ts`, `rulings.test.ts`) |

48 task headers `**M<n>-T<k>`; 118 distinct backticked `*.ts/tsx/json/sql/mjs` paths in BUILD. Every milestone gate has passed (reviews/ has m1, m2, m3, m3-m4-gates, m5, m6, m8, m5-m6-m8-gates, part-b-gate, polish-part-b, v0.2.0 part A). Milestone-ordering language ("Do not start M(n+1)…") is historical.

### 3.5 REVIEW.md — 183 lines, 18,469 bytes

`## 0. Rules for the reviewer` (5–15); `## Part A — Spec audit` (16–64: A1 completeness vs sources, A2 internal consistency, A3 implementability, A4 procedure) — language-independent, survives; `## Part B — Implementation audit` (65–151): B0 Setup (pnpm commands), B1 Architecture invariants (greps over `packages/engine/src`, `packages/cards/src`, `eslint.config.js`, `engine/src/rng.ts`), B2 Rules conformance (maps § to BUILD M-T tests), B3 Card conformance (`missing-tests.ts`, file counts "318 … 111 in packages/cards/test/"), B4 Rulings (grep of `rulings.test.ts`), B5 Client (survives mostly), B6 Server, B7 E2E, B8 Determinism and fuzz — **B0, B1, B3, B4, B8 and most of B2/B6 are TS-path-specific and become obsolete**; `## Report format` (152–174); `## Appendix — paste-in prompt` (175–183).

### 3.6 SPEC parts that are TS-specific

§9.3 (line 878–879: "`Math.random` banned by lint"), §9.7 "Suggested repository layout" (922–937, `scripts/<index>-<slug>.ts`), §10 (engine implementation guide: TS-shaped types such as `` `Script = { … modes?, conditionMet?, preview? }` `` in §10.9), §5.3 (`catalog.json` fields). 20 distinct backticked code paths in SPEC (`config.ts`, `triggers.ts`, `traps.ts`, `subsystems/callToChaos.ts`, `playChoices.ts`, `game/damageFeel.ts`, `gen-voice.mjs`, …).

---

## 4. Prose coupling (#133)

#133 was closed on 2026-10-04 with "moving to version 3"; the only commit referencing it is `bb7edb9` ("queue #133 [skip ci]", bot state). **None of its plan landed**: `splitPairedRule`, `inPlayerWords`, `PLAYER_WORDS`, `RULING_CITATION` still exist in `apps/web/src/cards/glossary.ts` (325 lines; `PLAYER_WORDS` l.126–141, `RULING_CITATION` l.143, `inPlayerWords` l.149–151, `keyword/trigger/verb/status` builders l.153–167, `splitPairedRule` l.175–185), and the R195 prose block is still in `conditionActive.test.ts`. SPEC §6.3 still has one "Degrade / Upgrade" row.

### 4.1 Code that reads Markdown at test/run time

| Site | Reads | What it asserts | Kind |
|---|---|---|---|
| `packages/engine/test/rulings.test.ts:4556` | `SPEC.md` | `^\| R(\d+) \|` ids == index `it` titles | structural (the #133 model) |
| `packages/engine/scripts/rulings-coverage.ts:26` | `SPEC.md` | same ids vs titles vs code citations | structural |
| `packages/engine/test/conditionActive.test.ts:770–825` (`textOf`, `specBetween`) | `SPEC.md`, `rulings.test.ts`, `../../cards/test/condition-active.test.ts` | l.787–797: exactly one R195 row, in numeric position, row contains `"| When a card glows yellow (\`conditionActive\`) |"`, last cell `toBe("§10.8, §10.9, #10, #53, #68, #71, #93, C #22, C #36, C #40")`; l.800–811: §10.8 contains `` `conditionActive: true` ``, `never set on the opponent's cards`, `R195`; §10.9 matches ``/`Script = \{[^`]*\bmodes\?, conditionMet\?, preview\? \}`/``, contains ``"`conditionMet` is R195's read-only predicate"`` and `"a card with \`conditionMet\` also tests both answers of it"`; l.813–824: rulings index R195 entry's file list `toEqual(["conditionActive.test.ts", "../../cards/test/condition-active.test.ts"])` | PROSE + exact file list |
| `apps/web/src/audio/spec-rows.test.ts:21–103` | `SPEC.md`, `packages/engine/test/rulings.test.ts`; spawns `rulings-coverage.ts` | R203/R204 rows exist and ascend; index `provenIn(203, …)` constants resolve to `cues.test.ts` + `director.test.ts`; `hasTestNamed`; `pnpm rulings:coverage` exits 0 (60 s timeout); `### 10.10 ` < `### 10.11 Audio$` < `## 11. ` heading order | ids + heading wording + duplicate of the coverage gate |
| `apps/web/src/cards/rules.test.ts:37–97` | `SPEC.md` | `ruleColumn(section)` splits SPEC §6.1/6.2/6.3 tables on `|`, keyed by first cell; `SPEC_ROW_NAME` shim (13 renames: `Armor`→`Armor X`, …, `Degrade`/`Upgrade`→`Degrade / Upgrade`); `PAIRED_HALF`; `specRule()` | PROSE table parse |
| `apps/web/src/cards/rules.test.ts:369, 391–392, 417, 435, 484` | via `specRule` | glossary `entry.rule` `toBe(inPlayerWords(specRule("6.x", id)))` for every keyword, trigger, verb, status; `RULED_TERMS` must differ; Tribute short reminder shorter | exact wording equality after regex rewriting |
| `apps/web/src/cards/rules.test.ts:444–508` | – | `inPlayerWords` and `splitPairedRule` unit cases (e.g. l.493 `"Weaken / strengthen a card: one change per application (R386)"`) | tests the string-surgery helpers |
| `apps/web/src/cards/rules.test.ts:368, 389, 415, 433, 439, 483, 499–500` | – | `entry.section` `toBe("§6.1" | "§6.2" | "§6.3" | "§5.2")` | section ids (structural-ish) |
| `packages/cards/test/radiant-standard.test.ts:33, 70–76` | `docs/radiant-audit.md` | every catalog entry has a row containing `` `| ${card.index} | ${card.name} |` `` | structural (index + name), ADDING_CARDS step 8 |

### 4.2 Source-text pins (same smell, against code/SQL instead of Markdown)

- `packages/engine/test/rulings.test.ts`: 23 `sourceOf(...)` uses; e.g. l.1224 `SERVER_MATCHES_SQL` contains `^[${alphabet}]{6}$`; l.1234–1240 `0002_collection.sql` / invites SQL regexes; l.1251–1252 `('redemption_failure_threshold', '100')`; l.1497–1500 `actor.ts` `function floodExceeded\(player: PlayerId`; **l.1529 `toMatch(/SPEC §11 R111: the launch quantity/)`** (a SQL comment), l.1530 `L5 card % totals % copies across the loadout, only % owned`; l.1538 `current_match_id is null as cleared`; l.1555–1600 regexes over `codes.ts` (`new ApiError\("invalid_code", REDEMPTION_IDENTICAL_ERROR\)`), `results.ts`, `clock.ts`, `wsServer.ts` (`unauthorized: 4401,` …, `private-use mirrors of the HTTP statuses`), `rooms.ts` (`const CODE_ATTEMPTS = \d+;`, `could not allocate a room code`). `serverConstant()` (l.414) regexes `export const NAME = …;`.
- `packages/engine/test/effects-core.test.ts:230, 249` reads source files line by line.
- `packages/shared/test/events.test.ts:23` reads `src/events.ts` source.
- `packages/ai/test/personas.test.ts:756–776` reads `packages/ai/src/*.ts` and `config.ts` source.
- `apps/server/test/validator-single-source.test.ts` (7 reads), `apps/server/test/db/seed-accounts.test.ts:54`, `apps/server/test/api/e2e.test.ts:489` (reads `e2e/support/config.ts`), `apps/server/test/api/codes.test.ts:775` (migration text).
- `apps/web/src/game/deckbuilder/messages.test.ts:33, 91` reads the validator source; `apps/web/src/wording.test.ts` parses every web source file with the TS compiler to ban "library"/"sacrifice" in player text (keeps working, web stays TS).
- 28 web tests regex CSS files (listed in 2.4).
- `packages/engine/test/handicap.test.ts:979` `/exactly 20 cards \(§2.6 L2\)/` (validator's user-visible message).
- `packages/cards/test/patches-ship.test.ts:288–291` `toContain('CATALOG_VERSION = "v0.2.0"')` in `catalog-data.ts`, `.env.example`, `render.yaml`, `apps/server/src/index.ts` (version sites).
- `packages/cards/test/catalog.test.ts` does **not** read docs at runtime: its fixture tables are literals transcribed from SPEC §8.1–8.5, §7 and `docs/classic-sets.md` B6/B7/B8 (header l.1–30).

### 4.3 Markdown read by tooling/workflows

`deploy-watch.yml` and `patches.test.ts:120` parse `render.yaml`; `.github/workflows/ci-duration.yml` none; `scripts/ci-scope.sh` / `vercel-ignore.sh` list doc paths but do not read them.

---

## 5. Doc graph data

### 5.1 Markdown inventory (`git ls-files '*.md'`: 74 files, 32,383 lines, 3.32 MB)

| Doc | Lines | Bytes | Role |
|---|---|---|---|
| `SPEC.md` | 1,986 | 853,936 | rules authority (§11 = 481 KB) |
| `docs/classic-sets.md` | 3,713 | 229,101 | Classic/Classic+ brief (B0–B9 sections, E1–E40 engine systems, CL1–CL46) |
| `BUILD.md` | 920 | 222,729 | work order M1–M9 |
| `docs/polish/3-ai.md` | 2,267 | 158,947 | AI polish (B-behaviours) |
| `docs/polish/4-edge-cases.md` | 1,773 | 158,043 | edge-case hunt |
| `docs/radiant-audit.md` | 497 | 129,099 | per-card Radiant audit table (read by a test) |
| `docs/polish/5-sign-in.md` | 1,200 | 107,273 | |
| `docs/polish/1-animations.md` | 1,135 | 100,723 | |
| `docs/polish/2-sound.md` | 1,228 | 97,199 | |
| `docs/polish/6-cards.md` | 1,432 | 94,542 | |
| `bot/README.md` | 1,079 | 88,559 | |
| `docs/polish/7-mobile-ux.md` | 1,129 | 72,151 | |
| `docs/architecture.md` | 820 | 67,599 | server deployment, R104–R112 |
| `JackiOh_Classic_Cards.md` | 2,703 | 50,914 | designer source |
| `packages/cards/README.md` | 611 | 47,685 | card-file contract |
| `e2e/README.md` | 298 | 47,537 | e2e contract (A1–A22 assumptions) |
| `apps/web/README.md` | 528 | 42,674 | |
| `apps/server/README.md` | 496 | 37,863 | |
| `CLAUDE.md` | 128 | 32,044 | agent contract |
| `docs/deploy-cloudflare.md` | 381 | 28,049 | |
| `JackiOh_Core_Cards.md` | 1,481 | 26,371 | designer source |
| `docs/polish/reference.md` | 420 | 23,790 | |
| `REVIEW.md` | 183 | 18,469 | |
| `packages/ai/README.md` | 253 | 18,071 | |
| `ARCHITECTURE-CCG.md` | 351 | 17,746 | source design note |
| `bot/machine/README.md` | 210 | 14,459 | |
| `docs/ADDING_CARDS.md` | 172 | 13,031 | |
| `docs/issues-and-patches.md` | 159 | 10,922 | |
| `JackiOh_Mechanics.md` | 233 | 10,763 | designer source |
| `assets/music/LICENSES.md` | 109 | 7,870 | |
| `ladder/README.md` / `ladder/DECISIONS.md` | 132 / 18 | 6,531 / 2,940 | |
| `README.md` | 58 | 2,729 | |
| `apps/web/src/cards/art/ART.md` | 42 | 2,853 | |
| `.harness/README.md`, `.squishy/README.md` | 17, 13 | | |
| `AGENTS.md`, `GEMINI.md`, `.github/copilot-instructions.md`, `JackiOh_Tokens.md` | 43, 5, 5, 59 | | |
| `reviews/*.md` (16 files) | 2,996 total | ≈ 428 KB | past audit reports |
| `.claude/skills/{fullsend,ponytail}/**` (7 files) | 877 | | skills |
| `bot/prompts/*.md` (9), `ladder/prompts/proposer.md` | 637 | | prompts |

### 5.2 Doc → doc references (path or unique basename mentioned; root `README.md` inbound count is inflated because every `*/README.md` path contains it)

| Doc | Out | References |
|---|---|---|
| `CLAUDE.md` | 20 | AGENTS, ARCHITECTURE-CCG, BUILD, GEMINI, JackiOh_{Classic_Cards,Core_Cards,Mechanics,Tokens}, README, REVIEW, SPEC, bot/README, bot/machine/README, docs/{ADDING_CARDS,architecture,classic-sets,deploy-cloudflare,issues-and-patches,polish/reference,radiant-audit} |
| `SPEC.md` | 12 | CLAUDE, JackiOh_Classic_Cards, JackiOh_Mechanics, README, assets/music/LICENSES, docs/{architecture,classic-sets,issues-and-patches,polish/3-ai,polish/4-edge-cases,radiant-audit}, e2e/README |
| `AGENTS.md` | 11 | ponytail SKILL, copilot-instructions, BUILD, CLAUDE, GEMINI, README, REVIEW, SPEC, bot/README, docs/ADDING_CARDS, docs/architecture |
| `docs/ADDING_CARDS.md` | 9 | BUILD, CLAUDE, README, REVIEW, SPEC, docs/architecture, docs/issues-and-patches, docs/radiant-audit, packages/cards/README |
| `docs/classic-sets.md` | 9 | BUILD, CLAUDE, JackiOh_{Classic,Core,Mechanics}, REVIEW, SPEC, docs/issues-and-patches, docs/radiant-audit |
| `packages/cards/README.md` | 9 | BUILD, CLAUDE, README, SPEC, apps/server/README, apps/web/README, art/ART.md, docs/classic-sets, docs/radiant-audit |
| `BUILD.md` | 8 | ARCHITECTURE-CCG, CLAUDE, JackiOh_Core_Cards, JackiOh_Mechanics, REVIEW, SPEC, docs/classic-sets, docs/polish/1-animations |
| `docs/polish/3-ai.md` | 8 | BUILD, CLAUDE, README, SPEC, apps/web/README, polish/reference, e2e/README, packages/ai/README |
| `REVIEW.md` | 7 | ARCHITECTURE-CCG, BUILD, JackiOh_{Classic,Core,Mechanics}, SPEC, docs/classic-sets |
| `bot/README.md` | 7 | fullsend SKILL, BUILD, CLAUDE, README, SPEC, bot/prompts/plan, docs/issues-and-patches |
| `docs/polish/7-mobile-ux.md` | 7 | CLAUDE, README, SPEC, apps/web/README, polish/reference, e2e/README, packages/cards/README |
| `docs/polish/{1,2,5}-*.md` | 6 each | CLAUDE, README, SPEC, polish/reference + apps/web/README / e2e/README / architecture |
| `docs/issues-and-patches.md` | 6 | BUILD, README, SPEC, bot/README, deploy-cloudflare, packages/cards/README |
| `docs/polish/{4-edge-cases,6-cards}.md` | 5 each | |
| `README.md` | 5 | BUILD, CLAUDE, REVIEW, SPEC, bot/README |
| `apps/server/README.md`, `apps/web/README.md`, `e2e/README.md`, `packages/ai/README.md`, `.harness/README.md` | 4 each | |
| `docs/architecture.md`, `docs/polish/reference.md`, `JackiOh_Classic_Cards.md`, `.squishy/README.md` | 2 each | |
| `ARCHITECTURE-CCG.md`, `JackiOh_{Core_Cards,Mechanics,Tokens}.md`, `docs/radiant-audit.md`, `bot/machine/README.md`, `assets/music/LICENSES.md`, `ladder/DECISIONS.md` | 0 | leaves |

Inbound (approx.): SPEC 46, CLAUDE 38, BUILD 26, REVIEW 17, ARCHITECTURE-CCG 10, JackiOh_Mechanics 10, JackiOh_Core_Cards 9, docs/polish/reference 9, bot/README 8, docs/classic-sets 7, docs/radiant-audit 7, apps/web/README 7, docs/architecture 6, docs/issues-and-patches 6, e2e/README 6.

### 5.3 Code → doc references (files under `packages/ apps/ e2e/` with `.ts .tsx .mjs .sql .css` mentioning the doc)

| Doc string | Files | Mentions |
|---|---|---|
| `SPEC` | 1,320 | 2,991 |
| `BUILD` | 633 | 1,146 |
| `CLAUDE.md` | 261 | 286 (rule 7 ×150, rule 5 ×34, rule 9 ×29, rule 4 ×22, rule 3 ×5, rule 6 ×3) |
| `docs/classic-sets` | 153 | 160 |
| `REVIEW` | 32 | 172 |
| `docs/polish/6-cards` 47, `1-animations` 39, `3-ai` 36, `7-mobile-ux` 34, `4-edge-cases` 32, `5-sign-in` 27, `2-sound` 24 | | |
| `e2e/README` | 17 | |
| `docs/architecture` | 12 | 19 |
| `ARCHITECTURE-CCG` | 6 | 21 |
| `docs/radiant-audit` | 4 | 7 |
| `packages/cards/README` 2, `apps/server/README` 2, `docs/issues-and-patches` 2, `apps/web/README` 1, `docs/ADDING_CARDS` 0, `packages/ai/README` 0 | | |

### 5.4 Id families (cross-cutting keys a graph would index)

Counted over `packages/ apps/ e2e/` (`.ts .tsx .sql .mjs .css`):

| Id family | Defined in | Distinct in code | Total citations | Notes |
|---|---|---|---|---|
| `R<n>` rulings | SPEC §11 (561 rows, R1–R762) | 561 (= all rows; enforced by `rulings:coverage`) | 28,716 in 1,759 files | per package: engine 9,927 (561 distinct), cards 10,123 (370), web 4,742 (231), server 2,988 (134), e2e 864 (152), ai 397 (64), shared 351 (113), validator 59 (7) |
| `§n(.n)` section refs | SPEC headings (≈ 67 ids) | 67 | 11,154 | engine 3,511, cards 4,788, server 1,549, web 657, e2e 410 |
| card ids `core-NNN`, `classic-NNN`, `classicplus-NNN`, `-k` tokens, `*-t-*` | `packages/cards/catalog.json` (268 cards + 50 tokens) | 333 | 11,372 | plus 6,389 prose card refs `#NN`, `C #NN`, `C+ #NN(.k)` |
| `M<n>-T<k>` BUILD tasks | BUILD.md (48 task headers) | 40 | 838 | |
| `B<n>` | **overloaded**: REVIEW.md B0–B8 (+B1.1…), each `docs/polish/*.md` B1–B75 (behaviour tables, e.g. 1-animations 75 rows, 3-ai 61, 6-cards 60), `docs/classic-sets.md` B0–B9 sections (B2.7, B6…) | 79 | 4,731 | same id means different things per doc; a graph needs doc-qualified ids |
| `E<n>` engine systems | `docs/classic-sets.md` (40 distinct, table rows like `\| E18 \| **New prompt kinds** \|`) | 41 | 1,512 | E5 107, E35 103, E18 100, E39 88, E21 73 citations |
| `CL<n>` | `docs/classic-sets.md` (46) | 0 in code | – | |
| `L1–L6`, `D1–D4`, `T1–T3` deck/trio/queue rules | SPEC §2.6/§9.4, validator | – | L1 102, L2 97, L3 96, L4 64, L5 90, L6 123, D1 80, D2 27, D3 18, D4 38, T1 233, T2 108, T3 113 (T-counts include other uses) | |
| `I1–I4` fuzz invariants | `packages/cards/test/_invariants.ts` | 4 | 29 | |
| `A1–A22` e2e assumptions | `e2e/README.md` table (l.206–227) | | | |
| `#<n>` GitHub issues | GitHub | 136 | 644 (`issue #n` / `(#n)` forms) | |
| `CLAUDE.md rule <n>` | CLAUDE.md rules 1–10 | 6 | 243 | rule numbers are stable by policy ("never renumber") |

---

## 6. Agent docs: statements that become wrong after a Rust rewrite

### 6.1 `CLAUDE.md` (128 lines, 32,044 bytes)

| Line | Statement | Why it breaks |
|---|---|---|
| 10 | "`BUILD.md` — … milestones M1–M8" | BUILD already has M9; and milestones become history |
| 17 (rule 1) | "Never implement a rule from memory of Hearthstone…" | survives |
| 18 (rule 2) | "Work BUILD.md in order … Do not open the next milestone until the current gate passes." | BUILD's milestones are done/TS-specific |
| 19 (rule 3) | "add the row to the index `packages/engine/test/rulings.test.ts` … `provenIn(n, file)` … `pnpm rulings:coverage` fails on … an `R<n>` cited anywhere in `packages/` or `apps/`" | index file, `provenIn`, and script path/command change |
| 20 (rule 4) | "`packages/engine`, `packages/cards` and `packages/ai` are pure: no `Math.random`, no `Date`, no I/O, no promises inside `reduce` … ESLint enforces this (it also bans timers, `fetch`, `process`, `crypto`, `node:*` imports and async functions…); tooling that needs `fs` goes in a package's `scripts/`" | enforcement becomes Rust (crate deps / clippy `disallowed-methods`, no `std::time`, no `rand::thread_rng`); package paths change |
| 21 (rule 5) | "Card scripts return `Effect[]` from `packages/engine/src/effects`." | Rust type/path |
| 22 (rule 6) | "one script file and one test file … per the BUILD M4-T4 table" | file layout changes |
| 23 (rule 7) | "The client sends intent and renders `viewFor`" | survives (WASM `viewFor`) |
| 24 (rule 8) | "run the paste-in prompt at the end of REVIEW.md" | REVIEW Part B is TS-specific |
| 25 (rule 9) | "rules numbers in `packages/engine/src/config.ts` (BUILD §2…), server numbers … in `apps/server/src/config.ts`" | paths change |
| 26 (rule 10) | "read `docs/ADDING_CARDS.md`" | survives; the doc's content changes |
| 30–53 | Commands: `Node ≥ 22.13 (.nvmrc: 24.19.0), pnpm 11`; `pnpm lint` ("Math.random / Date ban in engine, cards and ai"), `pnpm typecheck` ("regenerates the cards registry first, then tsc for every project"), `pnpm test` ("vitest projects: shared, engine, cards, ai, validator, server, web (fuzz at seeds 1–100, AI gates at 20 games)"), `pnpm test:coverage` ("90% line floor, engine + cards"), `pnpm fuzz`, `pnpm ai:gate/sweep/stats`, `pnpm validate:catalog` ("268 cards and 50 tokens"), `pnpm rulings:coverage`, `pnpm --filter @jackioh/cards missing-tests`, `pnpm --filter @jackioh/cards run patches …`, `pnpm test:sql`, `test:db` ("src/db/store.ts"), `test:deploy`, `gen:voice`, `gen:music` | all engine/cards/ai/server commands become cargo; web ones stay |
| 55 | "Replay one failing fuzz seed: `JACKIOH_FUZZ_FROM=<seed> JACKIOH_FUZZ_SEEDS=1 pnpm fuzz`" | |
| 57 | server bring-up: `pnpm --filter @jackioh/server db:migrate`, `db:seed-catalog`, `codes:mint`, `db:seed-accounts`, `dev`, `stats:cards`, `stats:import` | Rust server binary |
| 59–65 | "Running a subset: `pnpm vitest run --project engine` / `packages/cards/test/002-bigot.test.ts` / `-t "R58"`" | |
| 67–75 | E2E: `E2E=1 pnpm --dir apps/server dev`; networked spec list "05, 06, 07, 09, 10, 18, 19, 20, 26, 27, 35" (07 is wrong today) | server start command changes |
| 77–83 | "CI … reports five required checks" + per-check description (checks/ai-gate/sql/db/e2e) | will change with the CI trim; already miscounted (7 names) |
| 85–89 | `ci-scope.sh` skip list, `vercel.json`/`scripts/vercel-ignore.sh` ("a merge of tests, tooling, docs, the server or the bot never deploys") | lists change |
| 93 | "Workspace packages, from pure to impure" | |
| 95 | `packages/shared` (`Action`, `GameEvent`, `PlayerId`, `events.ts`, `stats.ts`) | becomes a Rust crate + generated TS types for web |
| 96–99 | `packages/engine` (`reduce(state, action, rng)`, `legalActions`, `viewFor`, `combat.ts`, `playSteps.ts`, `turn.ts`, `traps.ts`, `layers.ts`, `subsystems/*`, `reduce.ts`'s header, `triggers.settle`, `stateCheck.ts`, `work.ts`, `Resume` records "survive `JSON.parse(JSON.stringify(...))`", `replay.ts`) | Rust modules |
| 100 | "The engine doesn't depend on `packages/cards`… test-only script in `packages/engine/test/fixtures/`" | |
| 101 | `packages/cards` (`catalog.json`, `src/scripts/NNN-slug.ts` `{ def, base, radiant }`, `cardDef("core-NNN")`, `@jackioh/engine/effects`, read helpers, `src/scripts/_generated.ts` via `pnpm --filter @jackioh/cards gen`, `test/_harness.ts`'s `scenario()`, `query.ts`, `patches/`, `scripts/patches.ts`) | |
| 102 | `packages/validator` ("Client and server run the same module") | one Rust crate compiled to WASM for client |
| 103 | `packages/ai` (`decide`, `aiToAct`, `redact`, `determinize`, `shouldStop`, `src/` never imports `@jackioh/cards`, `registerAll()`, `buildAiDeck`, `shadowBan.ts`) | |
| 104 | `apps/server` ("one Node process", `src/api`, `src/match`, `src/api/ports.ts`, `src/match/contracts.ts`, `src/db/**`, "Vitest runs against an in-memory fake store", `test/db/contract.ts`, `src/match/engine.ts` dynamic import of `engine.real.ts`, `series-rules.ts`, `series.ts`, `results.ts`, `src/match/protocol.ts`, `src/env.ts`) | |
| 105 | `apps/web`: "The engine … is reached through two entry points … `src/game/engine.ts` (hotseat; … `engine.real.ts` is loaded lazily) and `src/practice/core.ts` … which alone imports `@jackioh/ai`" | entry points stay, implementation becomes WASM |
| 107–112 | client layers (`animations.ts`, `fx/`, `audio/`, `cards/`, `drag/`, `settings/`) | survive |
| 114 | Deployment: "`apps/server` runs on Render (`render.yaml`, whose start command takes `CATALOG_VERSION` from the newest entry of `patches.json`…)", "a match survives the restart by folding `(seed, log)`" | `render.yaml` `runtime: node`, `buildCommand: pnpm install …`, `startCommand: … node scripts/catalog-version.mjs … pnpm --filter @jackioh/server release && … start` all change |
| 118–120 | night bot runners "run only the light checks, leaving lint and the unit tests to the pull request's CI" | bot-forbidden; gates in `.harness/config.json` hard-code pnpm |
| 128 | Parallel work: "Don't use it for shared surfaces (SPEC §11 numbering, `packages/shared/src/events.ts`, `script.ts`, `state.ts`, the effects barrel)" | paths change |

### 6.2 `AGENTS.md` (43 lines)

- "Read first" table: `BUILD.md` "Work order, milestones, acceptance criteria, definition of done", `REVIEW.md` (as above).
- "Rules every agent follows": "work `BUILD.md` in order, keep `packages/engine`, `packages/cards` and `packages/ai` pure (no `Math.random`, `Date`, I/O, promises inside `reduce`), return `Effect[]` from card scripts" — paths and TS vocabulary.
- "Package contracts live in the READMEs of `packages/cards`, `packages/ai`, `apps/server`, `apps/web` and `e2e`" — package set changes.

### 6.3 `GEMINI.md` (5 lines), `.github/copilot-instructions.md` (5 lines)

Pointer files only ("read `AGENTS.md` first, then `CLAUDE.md`… `SPEC.md` wins"); nothing TS-specific. `README.md` (58 lines) also says "Node 22.13 or newer and pnpm 11", `pnpm dev/test/lint/typecheck`, "all 100 Core cards and 11 tokens" (stale: 268 + 50) and a night-bot description that is stale ("21:00 to 07:00 Central … Claude Opus").

### 6.4 `docs/ADDING_CARDS.md` (172 lines)

| Line(s) | Statement | After rewrite |
|---|---|---|
| 12–13 | "Core cards live at the top of `src/scripts/` and `test/`, Classic in `classic/`… `NNN-slug`" | Rust layout |
| 17–29 (table) | steps 1 `SPEC.md` §8.x row; 2 `packages/cards/catalog.json`; 3 `packages/cards/src/scripts/classic-plus/NNN-slug.ts` (`registry.test.ts`, `missing-tests`); 4 `packages/cards/test/classic-plus/NNN-slug.test.ts`; 5 `packages/cards/src/scripts/_generated.ts` ("run `pnpm typecheck`"); 6 `packages/cards/test/catalog.test.ts` (`RARITY_COUNTS`, `SET_SIZES`); 7 `packages/cards/scripts/validate-catalog.ts` (`EXPECTED_TAG_COUNTS`); 8 `docs/radiant-audit.md` (`radiant-standard.test.ts`); 9 `BUILD.md` must-pass row; 10 `apps/web/src/audio/card-audio.json5` (stays); 11 `packages/cards/flavour.json`; 13 `packages/cards/patches/pending/<version>.json` | steps 3–7, 13 change paths/commands; 1, 2, 8, 10, 11 survive if data stays JSON/MD |
| 31–42 | count assertions: `test/query.test.ts`, `test/registry.test.ts` (`CATALOG_SIZE`), `test/059-unbiased-immigration.test.ts`, `apps/server/test/api/catalog.test.ts`, `apps/server/test/db/seed-catalog.test.ts`/`.spec.ts`, `apps/web/src/game/deckbuilder/filters.test.ts`, `e2e/cypress/component/deckbuilder-layout.cy.tsx` (`DECKABLE_COUNT`) | Rust test paths |
| 49–107 | templates: catalog JSON (stays), TS script `import type { Script } from "@jackioh/engine"; … cardDef("classicplus-006")`, `destroyAll` from `@jackioh/engine/effects`, vitest test with `scenario` from `../_harness` | rewrite as Rust |
| 116 | `pnpm --filter @jackioh/cards run patches <version> …` | tooling command |
| 125 | "`typecheck` runs `gen`, whose `gen-loc` may update the card's `loc`" | |
| 132–135 | gates: `pnpm typecheck`, `pnpm lint`, `pnpm validate:catalog`, `pnpm --filter @jackioh/cards missing-tests`, `pnpm rulings:coverage`, `pnpm test` / `pnpm vitest run --project cards` ("about ten minutes … the two 1000-game fuzz suites"), `JACKIOH_FUZZ_FROM=<seed> JACKIOH_FUZZ_SEEDS=1 pnpm fuzz` | |
| 139–147 | seed-pinned tests (`play-choices`, `resolving-face`, `fused-hooks`, `tributes`, `pools-and-randomness`, `098-heroic-power`, `ai/test/shadowBan.test.ts`) | RNG must be bit-identical in Rust or every seed re-pins |
| 153–157 | "Do not read": `packages/cards/src/index.ts`, `_generated.ts`, `catalog-data.ts`, `packages/engine/**`, `apps/web/src/cards/**`, `apps/server/src/**`, `packages/ai/src/**` | paths |
| 162–164 | "A verb is an effect in `packages/engine/src/effects/`… board fact in `packages/engine/src/query.ts`, a new keyword also needs its row in SPEC §6.1 (the web glossary reads that table)" | the glossary claim is the #133 coupling |
| 168–172 | voice assets: `apps/web/src/audio/voice-assets.test.ts`, `gen:voice` (macOS `say`) | survives (web) |

---

## 7. Root tooling

### 7.1 `package.json` (root, 39 lines of JSON) scripts

| Script | Command | Survives if only web+e2e stay TS? |
|---|---|---|
| `build` | `pnpm --filter @jackioh/web build` | yes (+ a wasm-pack step first) |
| `build:e2e` | `pnpm --filter @jackioh/web build:e2e` (= `vite build --mode development`) | yes |
| `ai:gate` | `JACKIOH_AI_GATE=full vitest run --project ai gate-` | no (cargo) |
| `ai:gate:merge` | `tsx packages/ai/scripts/gate-merge.ts` | no |
| `ai:stats` / `ai:sweep` | `pnpm --filter @jackioh/ai stats|sweep` | no |
| `dev` | `pnpm --filter @jackioh/web dev` | yes |
| `fuzz` | `vitest run --project cards fuzz` | no |
| `lint` | `eslint .` | yes, scoped to `apps/web`, `e2e` |
| `rulings:coverage` | `tsx packages/engine/scripts/rulings-coverage.ts` | rewrite (could stay a TS/Node or become a Rust xtask) |
| `test` | `vitest run` | yes (web only) |
| `test:coverage` | `vitest run --coverage --project engine --project cards` | no (cargo-llvm-cov / tarpaulin) |
| `test:db` / `test:deploy` / `test:sql` | `sh apps/server/test/db/run.sh` / `sh apps/server/test/deploy/rehearse.sh` / `sh apps/server/test/sql/run.sh` | `test:sql` is pure SQL+Docker (keeps); the other two run Node/vitest and `render.yaml` |
| `test:e2e` | `pnpm --filter @jackioh/e2e test:e2e` | yes |
| `typecheck` | `pnpm --filter @jackioh/cards run gen && tsc -p tsconfig.json && tsc -p packages/{engine,cards,ai,validator}/tsconfig.json && tsc -p apps/server/tsconfig.json && tsc -p apps/web/tsconfig.json && tsc -p e2e/tsconfig.json && tsc -p e2e/cypress/component/tsconfig.json` | only the web + e2e + component `tsc -p` calls remain |
| `validate:catalog` | `tsx packages/cards/scripts/validate-catalog.ts && tsx packages/cards/scripts/patches.ts check` | depends on where patch tooling goes |

`packageManager: pnpm@11.3.0`, `engines.node >=22.13`; devDeps: `@eslint/js 10.0.1`, `@types/node`, `@vitest/coverage-v8 5.0.1`, `eslint 10.10.0`, `fast-check 4.10.1`, `tsx 4.23.13`, `typescript 6.0.3`, `typescript-eslint 8.70.0`, `vitest 5.0.1`.

Per-package scripts: `packages/cards` `gen` (`tsx scripts/gen-registry.ts && tsx scripts/gen-loc.ts`), `missing-tests`, `patches`; `packages/ai` `arena-bridge` (used by `ladder/`), `stats`, `sweep`; `apps/server` `dev`, `start`, `db:migrate`, `db:seed-catalog`, `release`, `codes:mint`, `db:seed-accounts`, `db:season-start`, `stats:cards`, `stats:import`, `typecheck`, `test` (all `tsx --env-file-if-exists=.env src/...`); `apps/web` `dev`, `build`, `preview`, `build:e2e`, `typecheck`, `test`, `gen:voice` (`node scripts/gen-voice.mjs`), `gen:music`; `e2e` `typecheck`, `typecheck:component`, `check:fixtures`, `test:e2e[:chrome|:electron]`, `test:component[:chrome]`, `open`, `open:component`, `verify`. Tooling dirs: `packages/cards/scripts/` (gen-loc, gen-registry, missing-tests, naming, patch, patches-io, patches, validate-catalog, versions; 2,420 lines), `packages/ai/scripts/` (arena-bridge, bench, duel, gate-merge, oracle, stats, sweep, trace; 1,038), `packages/engine/scripts/rulings-coverage.ts` (140), `apps/web/scripts/` (check-production-bundle.mjs, font-fallbacks.py, gen-music.mjs, gen-voice.mjs, lesson-deal.ts, music/; 2,946), root `scripts/` (catalog-version.mjs, ci-scope.sh, promote-production.sh, vercel-ignore.sh, worktree.sh).

### 7.2 `eslint.config.js` (97 lines)

- Ignores `**/node_modules/**`, `**/dist/**`, `**/coverage/**`, `**/test/fixtures/lint/**`, `.claude/**`; extends `js.configs.recommended`, `tseslint.configs.recommended`; `@typescript-eslint/no-unused-vars` with `^_` ignore patterns.
- `PURE_PACKAGES = ["packages/engine/**/*.ts", "packages/cards/**/*.ts", "packages/ai/**/*.ts"]` (src AND tests): `no-restricted-properties` bans `Math.random`, `Date.now`; `no-restricted-syntax` bans `new Date(...)` and `Date(...)`.
- `PURE_SOURCES = ["packages/engine/src/**/*.ts", "packages/cards/src/**/*.ts", "packages/ai/src/**/*.ts"]`: `no-restricted-globals` bans `setTimeout setInterval setImmediate queueMicrotask fetch performance process crypto`; `no-restricted-imports` bans `node:*`, `fs`, `fs/*`, `path`, `os`, `child_process`, `http`, `https`, `net`; `no-restricted-syntax` bans `Date`, `:function[async=true]`, `new Promise`.
- `**/*.mjs`: Node globals allowed.
- Tested by `packages/engine/test/lint-ban.test.ts` against `packages/engine/test/fixtures/lint/`.
- After the rewrite, only the base config + unused-vars rule apply (to `apps/web`, `e2e`); all purity blocks move to Rust.

### 7.3 tsconfig layout

`tsconfig.base.json` (ES2023, ESNext/Bundler, strict, `noUncheckedIndexedAccess`, `noImplicitOverride`, `noFallthroughCasesInSwitch`, `isolatedModules`, `allowImportingTsExtensions`, `verbatimModuleSyntax`, `resolveJsonModule`, `noEmit`, `types: []`); `tsconfig.json` (root: `types: ["node"]`, includes `packages/*/src`, `packages/*/test`, `packages/*/scripts`); `packages/{engine,cards,ai,validator}/tsconfig.json` (extend base, `include: ["src"]`); `apps/server/tsconfig.json` (`types: ["node"]`, includes `src`, `test`, `vitest.config.ts`); `apps/web/tsconfig.json` (DOM libs, `jsx: react-jsx`, `types: ["vite/client"]`, includes `src`, `vite.config.ts`, `vitest.config.ts`); `e2e/tsconfig.json` (`types: ["cypress","node"]`, includes `cypress.config.ts`, `support/**/*.ts`, `cypress/**/*.ts`; excludes `support/tasks/replay-runner.ts` and `support/tasks/lessons-runner.ts`); `e2e/cypress/component/tsconfig.json`. `packages/shared` has no tsconfig of its own (covered by root).

### 7.4 `pnpm-workspace.yaml` (7 lines)

`packages: [packages/*, apps/*, e2e]`; `allowBuilds: { cypress: true, esbuild: true }`. `e2e/` also has its own `pnpm-workspace.yaml` and `pnpm-lock.yaml`.

### 7.5 What remains TypeScript if only `apps/web` and `e2e` stay

- Workspace: `apps/web`, `e2e` (+ whatever package hosts the generated WASM bindings and TS types for `Action`, `GameEvent`, `PlayerView`, `CardDef`, today from `@jackioh/shared` in 143 web src files, and `@jackioh/engine/config` constants in 21).
- Web's engine boundary today: `apps/web/src/game/engine.ts` → `engine.real.ts` (hotseat), `apps/web/src/practice/core.ts` (+ `practice.worker.ts`, `host.ts`), `apps/web/src/tutorial/harness.ts`; validator used directly in 20 deckbuilder files; `@jackioh/cards` (`CATALOG`, `registerAll`, `CATALOG_VERSION`, `CardFlavour`) in 4 src files and 52 test files; `@jackioh/ai` in 3 src files.
- e2e coupling to engine source: `e2e/support/tasks/replay-runner.ts` dynamically imports `packages/cards/src/index.ts` and `packages/engine/src/replay.ts` under the repo's `tsx` (spec 01 replay hash; spec 13 too); `e2e/support/tasks/lessons-runner.ts` imports `apps/web/src/tutorial/lessons.ts` and `packages/engine/src/config.ts` (spec 23). Both need a WASM/CLI replacement. Component specs import `apps/web/src/game/Game.tsx` and `apps/web/src/test/fixtures.ts` directly (stay).
- Root tooling kept: `eslint.config.js` (base rules only), `tsconfig.base.json`, web/e2e tsconfigs, `vitest.config.ts` (projects shrink to `apps/web`; the coverage block for engine/cards goes), `package.json` scripts `build`, `build:e2e`, `dev`, `lint`, `test`, `test:e2e`, a reduced `typecheck`; `scripts/*.sh` (bash, language-neutral); `apps/web/scripts/gen-voice.mjs`, `gen-music.mjs`, `check-production-bundle.mjs`.
- Tooling whose language is a plan decision (TS today): `packages/cards/scripts/*` (catalog validation, registry/loc generation, patch history ship/check — `patches-ship.yml` runs it), `packages/engine/scripts/rulings-coverage.ts`, `packages/ai/scripts/*` (`arena-bridge.ts` is the stdio bridge `ladder/arena/bridge.py` talks to).
- Gone with the TS engine: `packages/{shared,engine,cards,ai,validator}/{src,test}` (≈ 61.5k src lines, ≈ 160.8k test lines), `apps/server/{src,test}` (≈ 19.7k src, ≈ 20k test lines + SQL evidence which stays), `@vitest/coverage-v8`, `fast-check`, `tsx` (unless tooling stays TS), the purity ESLint blocks, `packages/engine/test/fixtures/lint/`.
- Bot-owned config that hard-codes the TS toolchain and can only be changed by a person: `.harness/config.json` / `.squishy/config.json` (`install`, `gates`, `required_checks`, `review_paths` incl. `pnpm-lock.yaml`, `vitest.config.*`, `eslint.config.*`, `tsconfig*.json`), `bot/harness/easy.py`, `bot/harness/work.py`, `bot-night.yml`, `squishy-run.yml`, `bot-selftest.yml` actionlint list.

---

## 8. Discrepancies found along the way (useful for the plan)

1. `ci.yml:41` `CATALOG_VERSION: v0.2.0` while `render.yaml`/patches are at v0.2.11.
2. `ci.yml:87` step title "268 cards, 49 tokens"; CLAUDE.md and BUILD say 50 tokens (`e2e/README.md` A7 also says 49).
3. CLAUDE.md "five required checks" vs 7 names in `required_checks`.
4. CLAUDE.md lists e2e spec 07 as networked; the spec header says no server.
5. CLAUDE.md line 10 says BUILD has M1–M8; BUILD has M9.
6. `rulings-coverage.ts` scans only the top level of `packages/cards/test/` for `it("R…")` titles (classic/ and classic-plus/ are not scanned); `rulings.test.ts`'s `provenIn` covers them instead.
7. `README.md` says "all 100 Core cards and 11 tokens" and describes the night bot as Opus-only 21:00–07:00.
8. `ladder/tests/` (Python) is run by no workflow.
9. `apps/web/src/audio/spec-rows.test.ts` re-runs `rulings-coverage.ts` inside the web unit suite (60 s timeout), duplicating the `static` job.
10. The AI gate failed on a main push on 2026-10-06 (`ai gates (3/12)`), so the full gate is not a stable per-PR signal.
