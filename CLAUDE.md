# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

# JackiOh — project instructions for Claude Code

JackiOh is a 1v1 card game: Hearthstone-style mana, combat and keywords on Yu-Gi-Oh-style lanes with a hidden trap backrow. Three documents drive all work:

- `SPEC.md` — the master game specification. The only source of rules, cards and engine design. Section references (§) everywhere point here.
- `BUILD.md` — the work order: repo layout, constants, milestones M1–M8 with tasks, files and acceptance criteria, the per-card must-pass table, animations, e2e specs, definition of done.
- `REVIEW.md` — the audit procedure: Part A checks SPEC.md against the source design notes; Part B checks the code against SPEC.md and BUILD.md.

Supporting docs: `docs/architecture.md` (server deployment: Supabase, match actor, R104–R112), the READMEs in `packages/cards`, `packages/ai`, `apps/server`, `apps/web` and `e2e`, which set the contracts inside each package, and `docs/polish/` (the polish pass's brief, `reference.md`, and one design note per task: animations, sound, AI, edge cases, sign-in, cards, mobile UX). `JackiOh_Mechanics.md`, `JackiOh_Core_Cards.md`, `JackiOh_Classic_Cards.md` (the designer's Classic and Classic+ list, read card by card in the design brief `docs/classic-sets.md`) and `ARCHITECTURE-CCG.md` (the generic trust model and match-actor design that SPEC §9 and `docs/architecture.md` build on) are the source design notes that REVIEW Part A checks SPEC against. `JackiOh_Tokens.md` holds the source text for the tokens (SPEC §7). Past audit reports are in `reviews/`. If a doc disagrees with SPEC.md, SPEC wins and the doc is the bug.

## Rules of engagement

1. Read SPEC.md fully before the first task and re-read the relevant section before each task. Never implement a rule from memory of Hearthstone or Yu-Gi-Oh when SPEC.md states it.
2. Work BUILD.md in order. A task is done when its acceptance items are green tests. Do not open the next milestone until the current gate passes.
3. Rulings live in SPEC §11. If you need a decision the spec does not make, follow Hearthstone semantics, append a new R-row to SPEC §11 in the same PR, and name the proving test after it (`it("R58 …")`). The proof can live in any package. Then add the row to the index `packages/engine/test/rulings.test.ts`, which has one `it("R<n> …")` per §11 row, in order, pointing at the proof with `provenIn(n, file)`; its completeness test fails without that entry. Use the next free number. `pnpm rulings:coverage` fails on three things: a row with no test, a test naming a row that doesn't exist, or an `R<n>` cited anywhere in `packages/` or `apps/` (comments included) that §11 doesn't have.
4. `packages/engine`, `packages/cards` and `packages/ai` are pure: no `Math.random`, no `Date`, no I/O, no promises inside `reduce`. Every random draw goes through `rng` in state; every player choice is a `PendingChoice` in state. ESLint enforces this (it also bans timers, `fetch`, `process`, `crypto`, `node:*` imports and async functions in those packages); tooling that needs `fs` goes in a package's `scripts/`, never in `src/`.
5. Card scripts return `Effect[]` from `packages/engine/src/effects`. Never mutate state in a card file.
6. Every card has one script file and one test file covering base and radiant behaviour per the BUILD M4-T4 table.
7. The client sends intent and renders `viewFor`; it never enforces rules and never sees hidden information.
8. Before claiming a milestone is done, run the paste-in prompt at the end of REVIEW.md as a separate session and attach the report.
9. Every number is a named constant: rules numbers in `packages/engine/src/config.ts` (BUILD §2; the "decide" rulings R1, R4, R5, R14, R26, R39 included, and R2's turn cap), server numbers such as clocks, Elo and rate limits in `apps/server/src/config.ts`. Nothing else states a number.

Code comments cite these rules by number ("CLAUDE.md rule 7"), so add new rules at the end and never renumber.

## Commands

Node ≥ 22.13 (`.nvmrc`: 24.19.0), pnpm 11.

```
pnpm install
pnpm lint              # includes the Math.random / Date ban in engine, cards and ai
pnpm typecheck         # regenerates the cards registry first, then tsc for every project
pnpm test              # vitest projects: shared, engine, cards, ai, validator, server, web (fuzz at seeds 1–100, AI gates at 20 games)
pnpm test:coverage     # 90% line floor, engine + cards
pnpm fuzz              # the CI gate: seeds 1–1000 of random-policy games with replay hashing, one seat handicapped in fuzz-handicap, ~100 s
pnpm ai:gate           # the AI's quality gates at full size (random 100, greedy 50, Hard vs Easy 50); minutes, CI's ai-gate job
pnpm ai:sweep          # the shadow-ban sweep (R186); prints SHADOW_BAN rows to copy into packages/ai/src/shadowBan.ts by hand
pnpm ai:stats          # an AI development run for the card statistics (R378): AI-vs-AI All Random games filed under a patch, --out writes them
pnpm validate:catalog  # catalog.json data checks (268 cards and 49 tokens across Core, Classic, Classic+; rarity counts per set)
pnpm rulings:coverage  # SPEC §11 rows vs named tests vs R-ids cited in code (rule 3)
pnpm --filter @jackioh/cards missing-tests   # catalog ids with no test file, and the path each one expects
pnpm --filter @jackioh/cards patch <version> "<title>" --date <YYYY-MM-DD>   # the card patch history, packages/cards/patches/ (R388): snapshots the catalog and bumps CATALOG_VERSION everywhere
pnpm test:sql          # schema, RLS and trigger invariants: Docker only, starts a throwaway postgres:16
pnpm test:db           # src/db/store.ts against a throwaway postgres:16 (Docker only; KEEP_DB=1 keeps it)
pnpm test:deploy       # Render's deploy rehearsed: render.yaml's start command, migrating as a role that is not a superuser (as on Supabase), then a production boot (Docker only)
pnpm --filter @jackioh/web gen:voice   # re-render changed voice lines with macOS `say`/`afconvert` (idempotent); `--check` runs anywhere
```

Replay one failing fuzz seed: `JACKIOH_FUZZ_FROM=<seed> JACKIOH_FUZZ_SEEDS=1 pnpm fuzz`.

Server against a real Supabase project (copy `apps/server/.env.example` to `.env` first; the client's half is `apps/web/.env.example`): `pnpm --filter @jackioh/server db:migrate`, then `db:seed-catalog`, then `codes:mint` for an invite code (or `db:seed-accounts` for active test accounts that skip the invite gate; it refuses under `NODE_ENV=production`), then `dev`. Card win rates off the game records every finished match leaves: `stats:cards` (live games unless `--source=dev` or `all`; `--mode`, `--patch`, `--pilot`, `--card`, `--json`), and `stats:import <file>` loads an `ai:stats` run (SPEC §9.11).

Running a subset:

```
pnpm vitest run --project engine                       # one project
pnpm vitest run packages/cards/test/002-bigot.test.ts  # one file
pnpm vitest run --project cards -t "R58"               # by test name
```

E2E (Cypress). The root `pnpm install` covers `e2e/`, as CI relies on. `e2e/` also carries its own `pnpm-workspace.yaml` and lockfile, so `cd e2e && pnpm install` works on its own too.

```
E2E=1 pnpm --dir apps/web dev       # http://localhost:5173, serves /dev/hotseat
E2E=1 pnpm --dir apps/server dev    # :8787 + /ws/match; needed for networked specs 05, 06, 07, 09, 10, 18, 19, 20, 26, 27
cd e2e && pnpm exec cypress run --spec cypress/e2e/01-hotseat-full-game.cy.ts
pnpm --dir e2e test:component       # component/pixel specs, no server needed; E2E_COMPONENT_PORT (default 5273) moves its dev server
```

The server reseeds fixture accounts and invite codes at boot (R144). Spec 10 uses them up, so restart the server before re-running it. `vite dev` reloads the page mid-run whenever `apps/web/src` changes, which restarts a hotseat game at turn 0. If you're editing while specs run, serve a built client the way CI does: `pnpm build:e2e`, then `pnpm --dir apps/web exec vite preview --port 5173 --strictPort` (a plain `build` strips `window.__jackioh`, and every spec fails). `99-online-smoke.cy.ts` drives the deployed stack with real accounts. It is skipped unless enabled, and its header gives the command.

CI (`.github/workflows/ci.yml`) reports five required checks. Each long one is a summary job over short jobs that run side by side, so no job takes more than five minutes (#73):
- `checks`: lint, typecheck, validate:catalog, missing-tests and rulings:coverage in one job; `pnpm test` by project; the fuzz gate by seed range; `test:coverage` by shard, with the 90% floor held on the merged report.
- `ai-gate`: `pnpm ai:gate`, every k-th game per shard (`JACKIOH_AI_GATE_SHARD=k/K`). `pnpm ai:gate:merge` holds the shards' wins together against `gateNeeded`.
- `sql`: `test:sql`.
- `db`: `test:db`, then `test:deploy`.
- `e2e`: the twenty-eight specs (`01`–`28`) on Chrome and on Electron, each browser split by `e2e/scripts/shard-specs.mjs` over jobs that boot their own server, plus the component specs on Chrome.

`deploy-watch.yml` polls the live server after every push to main and opens an issue if it never serves render.yaml's catalog version, which is what a failed Render deploy looks like from outside. `ci-duration.yml` reads every CI run's job times and opens an issue (or comments on the open one) when a job went over five minutes; split that job further, usually by lengthening its matrix list.

`bot-selftest.yml` adds a sixth required check, `bot selftest`: the night bot's own suite (`cd bot && python3 -m unittest discover -s tests -t .`) and actionlint over its workflows. Branch protection on `main` requires all of these checks, which is what lets the night bot's pull requests auto-merge safely.

## Architecture

Workspace packages, from pure to impure:

- `packages/shared` holds the types every layer shares: `Action`, `GameEvent`, `PlayerId`, and the event list in `events.ts`. `stats.ts` is the card statistics' game record and the win rates read off a set of them (SPEC §9.11, R376–R378), which the server's scripts and the AI's development run share.
- `packages/engine` is the rules. Its entry points are `reduce(state, action, rng)`, `legalActions` and `viewFor(state, playerId)`. `reduce` clones state, applies the action and returns new state plus events. An illegal action comes back as an error with the state unchanged. Each rule has one owning module (`combat.ts`, `playSteps.ts`, `turn.ts`, `traps.ts`, `layers.ts`, `subsystems/*`, …), and `legalActions` and the reducer's refusals call the same function, so the two can't disagree. `reduce.ts`'s header maps each action to its owning module.
- The resolution loop (`triggers.settle`, SPEC §10.3) runs after every action. It dispatches events, drains `state.work`, runs the state check (`stateCheck.ts`) and pops the trigger queue until everything is empty or a prompt stops it.
- Prompts end the action (`state.pending`). The one exception is the mulligan: both seats' mulligan prompts are open at once in `state.mulligan`, each answer sealed until both are in, and they resolve together in seat order (R265–R268); code that plays both seats asks `seatToAct(state)` who acts next. Any sequence that can pause mid-way parks its remainder on `state.work` (`work.ts`) as plain-data `Resume` records, never closures. That lets a paused state survive `JSON.parse(JSON.stringify(...))` and replay exactly. Resume order follows R113 (a cursor, not a queue or a stack). Read `work.ts`'s header before touching anything that can open a prompt.
- `replay.ts` folds an action log back into state. The fuzz suite and e2e check replay hashes against it.
- The engine doesn't depend on `packages/cards`. An engine test that needs a card's behaviour uses a test-only script in `packages/engine/test/fixtures/`, and the real card's test covers the same case again.
- `packages/cards` holds `catalog.json` (the card data, proved against SPEC §8; each card's `radiant.text` is its Radiant face written out in full, R277, and `refs` lists the cards its text names, R279, proved by `test/references.test.ts`; every Radiant face meets R275's standard, recorded in `docs/radiant-audit.md`) and `src/scripts/NNN-slug.ts`, one per card: `{ def, base, radiant }`. `def` always comes from `cardDef("core-NNN")` and is never retyped. A script writes through `@jackioh/engine/effects` and reads state only through the read helpers `@jackioh/engine` exports (`heroOf`, `zoneCards`, `activeUnitsOf`, …, listed in the cards README), never `state.players[…]`. If a verb or fact is missing, add it to the engine (new board facts go in `packages/engine/src/query.ts`) and test it there. `src/scripts/_generated.ts` is generated by `pnpm --filter @jackioh/cards gen`, which typecheck and the test globalSetup also run, so never edit it by hand. Card tests live at `test/NNN-slug.test.ts` and must build games through `test/_harness.ts`'s `scenario()`. Every random pool goes through `query.ts`. `patches/` is the card patch history (R388): `patches.json`, a `<version>.json` snapshot of the catalog per patch and `index.json`, written by `scripts/patch.ts`; the newest snapshot always equals `catalog.json` and its version is `CATALOG_VERSION`, so a change to the catalog is a new patch. The package README is the card-file contract.
- `packages/validator` checks decks and trios (SPEC §9.4): the structure a save checks (D1–D4 for a deck, T1–T3 for a trio; a saved deck is a draft, R250) and the rules a queued deck or trio must pass (L2, L3, L5, L6 for a Best-of-1 deck; L1–L6 for a Conquest trio, whose three decks share no card, R253), and the room a trio import needs under the caps (R340). Client and server run the same module.
- `packages/ai` is the practice opponent (SPEC §9.9), pure and seeded like the engine and linted as such. `decide(state, seat, options)` reads the true state only through `aiToAct` and `redact` (R185) and simulates only on `determinize`d copies, so hidden cards cannot change a decision. Budgets count `reduce` calls; the browser's wall-clock cap arrives as `shouldStop`. The difficulty tiers are engine handicaps (`AI_DIFFICULTY` in `config.ts`, R180–R184), never AI behaviour. `src/` never imports `@jackioh/cards`; callers `registerAll()` first. Decks come from `buildAiDeck` minus `shadowBan.ts` (R186), except a tutorial lesson's fixed list (R291), and its README is the contract.
- `apps/server` is one Node process that runs both the HTTP API (`src/api`) and one in-memory match actor per match (`src/match`), which pushes `viewFor` to each player over a WebSocket. Handlers and the actor depend only on ports (`src/api/ports.ts`, `src/match/contracts.ts`). `src/db/**` implements the `Store` port against Supabase Postgres. Vitest runs against an in-memory fake store, so RLS and triggers are only checked by `test:sql`/`test:db`. `test/db/contract.ts` is one suite run against both stores, the fake in `pnpm test` and Postgres in `test:db`, so a change to the `Store` port goes there. The engine is reached only through `src/match/engine.ts`, whose dynamic import of `engine.real.ts` is deliberate (its header says why); All Random's decks (R258) come through the same port. A Conquest series (the trio mode, still `bo3` on the wire: a win with each deck takes it, a deck that wins is locked, and both players seal a pick before each game, R330–R338) is a database row written by compare-and-set: `src/api/series-rules.ts` holds its pure transitions, `src/api/series.ts` its routes and the sweeper that runs the pick clock and restarts a game a restart interrupted, and `results.ts` advances it in the same transaction as a game's result. A trio import (`POST /api/trios/import`, R341) writes its decks and trio in one transaction. The wire protocol is fixed in `src/match/protocol.ts`. Env is parsed in `src/env.ts` (`E2E` must never be set together with `NODE_ENV=production`).
- `apps/web` is a Vite/React client. Components take a `PlayerView` and nothing else. The engine (its `config` constants aside) is reached through two entry points and nowhere else: `src/game/engine.ts` (hotseat; `EngineState` is opaque, and `engine.real.ts` is loaded lazily) and `src/practice/core.ts`, which only the practice Web Worker (`practice.worker.ts`; `host.ts` runs it in-thread under jsdom) loads, and which alone imports `@jackioh/ai`. The worker stands where the server's match actor does: `/practice` holds only `viewFor(state, human)`, the human's `legalActions` and whether the AI owes a move (R187). The tutorial (`src/tutorial/`, SPEC §9.10) sits at the top of `/practice`: each lesson is a practice game the worker plays with the lesson's fixed decks and the tutorial handicap (`AI_TUTORIAL`, R290, R291), its coach reads that same snapshot and nothing else (R292), and its progress stays on the device, with a copy on an active account merged as a union (R294, R320, R321). Legality comes from `legalActions`, never a client-side check. `/dev/hotseat` runs the engine locally and exposes `window.__jackioh` only in non-production builds, which is why `build:e2e` exists. Online play uses Supabase Auth directly and sends the token to the server.
- `Game.tsx` is the one board for hotseat, online and practice, inside the route's `CatalogContext`, and mounts every client layer. None of them is a rule (rule 7) and each has one owning module:
  - `src/game/animations.ts` — the `ANIMATIONS` total map over `GameEventType` and the queue runner: the view swaps only when an event's entry ends, and `data-animating` marks what is in flight (`cy.settled()` and practice pacing both wait on it).
  - `src/fx/` — the effects layer: a pooled canvas overlay and DOM flourishes that decorate the runner's entries and never pace them (R200–R202).
  - `src/audio/` — the WebAudio engine, procedural SFX and pre-rendered voice lines; `SOUND_CUES` is a total map over `GameEventType`, cues play when the runner starts their entry, and nothing sounds for a card behind the sentinel (R203, R204). The engine's `speaking()` becomes the board's `data-speaking`, which practice holds the AI on.
  - `src/cards/` — `CardFace`, the board's `MinionFace`, deterministic procedural art (real art is listed in `art/manifest.ts`), hover and long-press inspect. `RulesText` draws every face's text with its marks: a Radiant face's changes in gold (`radiantDiff.ts`, R277), the cards its `refs` name as references with a tooltip (`refs.ts`, `CardRef.tsx`, R279), and in play the numbers the view's `preview` carries, "{n}" (R280).
  - `src/game/drag/` plus `glow.ts` — drag to play (click-click always works too) and the green (`legalActions`) and yellow (`conditionActive`, R195) glows.
  - `src/settings/` — the store and panel. Tasks' own stores (`fx/settings.ts`, `audio/settings.ts`, `cards/settings.ts`) mount through `SETTINGS_SLOTS`; "Hover previews" and "Reduce motion" are the panel's own switches and gate the card preview and the effects too. Every store sits in `localStorage` inside try/catch and applies live.

Deployment: `apps/web` is a static bundle on Vercel (`vercel.json`), and `apps/server` runs on Render (`render.yaml`). Render's free tier sleeps when idle, and a match survives the restart by folding `(seed, log)`. Postgres and Auth are on Supabase. `docs/architecture.md` has the bring-up checklist and the env-var contract.

## The night bot

`@jgoetzmann-bot` builds issues on whichever of the owner's subscriptions is free (`bot/README.md` covers it in full): up to four Claude accounts running Opus (21:00 to 07:00 Central), and at any hour ChatGPT through Codex, Google through Antigravity (`agy`) and Meta through Muse Code, listed with their models, logins, runners, hours and limits in `.harness/providers.json`. Up to three items run at once, one per subscription. The model sessions run on the bot's own AWS machine, each subscription as its own Linux user with its own runner (`night-vm-<id>`), which powers off when idle and is started by a five-minute check (`bot/machine/README.md`). It picks up an issue labelled `bot:build`, assigned to it, or named in a `/harness build` or `@jgoetzmann-bot …` comment; an issue labelled `difficult` is Opus's alone. An independent adversarial reviewer of the same model then reads the change, the two go round until the reviewer approves, and the pull request merges itself once CI passes; a change a model other than Opus built must first be approved by a second model too (`bot:cross-review`). The shared Claude account (`claude-1`) is used only when it is quiet: nobody else's usage rose over ten minutes, bright-bots-harness excepted. A run that stops half-way leaves its branch and a handoff (its notes and the end of its session) for the next run, on any subscription. The bot code is Python in `bot/`, its switches are in `.harness/` (`config.json`, `providers.json`, `trust.txt`, an optional `HALT`), and its workflows are `bot-night.yml`, `bot-commands.yml` and `bot-selftest.yml`.

When you are the bot's builder or reviewer, this file binds you like anyone else. You must not change `.github/`, `.harness/` or `bot/`: the harness puts such a change back, and the deliver job refuses to push one.

## Parallel work

`scripts/worktree.sh <name> [base-ref]` creates a git worktree at `../jackioh-wt/<name>` on branch `wt/<name>` with dependencies linked, so each worktree tests only its own changes. It suits isolated work like card scripts. Don't use it for shared surfaces (SPEC §11 numbering, `packages/shared/src/events.ts`, `script.ts`, `state.ts`, the effects barrel): parallel edits to those conflict on merge.
