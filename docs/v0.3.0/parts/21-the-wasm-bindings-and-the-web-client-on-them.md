# v0.3.0 (part 21 of 40): the WASM bindings and the web client on them

Part 21 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | part 1 |
| Branch | `staging` (push straight to it; no pull request) |
| Builder | a Claude Code cloud session, Opus |
| Notes | `.fullsend/notes/part-21.md` and `.assumptions` (SURFACE §16) |

Unlabelled on purpose: #306 holds labels until a person says the run is ready. The Plan below is between the night bot's plan markers, so if a person later queues this issue for a bot, it builds from this plan without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Put the web client on the Rust engine and AI through WebAssembly without changing anything a player sees: the `jackioh-wasm` bindings, the build script, the TS wrapper, the hotseat engine port and the practice worker on it, the aliases that keep ~150 web imports unchanged, the client's wire layer (generated types and constants plus hand-kept helpers), the emote personas moved into the web, the e2e harness's replay on the `jackioh` CLI, and the CSP and Vercel changes WASM needs.

**How you work (fullsend builder rules, in priority order; full text in `.claude/skills/fullsend/agents/builder.md`).**
1. Never run `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or any test runner. Nothing on `staging` compiles until Wave 3; every error you would see is someone else's unwritten file. Count any you ran as `BUILDS-RUN`.
2. Write only the files in your Files to touch table and your two notes files. Read anything in `packages/`, `apps/`, `docs/`, `SPEC.md` (the TypeScript is your spec); do not read other parts' Rust under `crates/`: it is half-written.
3. Match SURFACE.md exactly at every boundary: paths by §4.1, names by §4.2, types by §4.3, the shapes of §5–§14.
4. No stubs: no `todo!()`, `unimplemented!()`, placeholder bodies or `// TODO`. Port the real body; if you truly cannot, leave the function out and list it under GAPS.
5. Duplicate on purpose: a small helper another part probably writes, write your own private copy.
6. Call what you wish existed: another module's function by its TS name snake_cased at its TS module's Rust path.
7. Don't ask questions: decide, and record the decision in your notes.
8. Stop when Done when holds.

**Pushing.** README §3.2: `git fetch origin staging && git checkout -B work origin/staging`, write, commit only your files, `git pull --rebase origin staging && git push origin HEAD:staging` (retry with back-off). Commit often. A session that is cut off leaves its work on `staging`; the next session for this part starts at the first file in the table that is missing or empty.

### 2. Files to touch

| File | new/change/delete | What |
|---|---|---|
| `crates/wasm/src/lib.rs` | new | SURFACE §10.1, every binding; `init` calls `console_error_panic_hook::set_once()` and `jackioh_cards::register_all()` |
| `scripts/build-wasm.sh` | new | SURFACE §10.2 |
| `apps/web/src/wasm/index.ts` | new | SURFACE §10.3 |
| `apps/web/src/wire/index.ts, catalog.ts, codes.ts, emotes.ts, aim.ts, stats.ts, validator.ts, cards.ts, ai.ts` | new | SURFACE §10.4; the helpers copied unchanged from packages/shared/src; validator.ts and ai.ts wrap WASM with the TS signatures |
| `apps/web/src/wire/generated/.gitkeep` | new | the ts-rs output lands here in Wave 3 |
| `apps/web/src/wire/engine.ts` | new | SURFACE §10.4's `@jackioh/engine` row: the `../wasm` functions under their TS names, `type GameState`, `createRng`/`type Rng`, and `subsystems` (`CHAOS_EFFECTS`, `CHAOS_PLUS_EFFECTS`, `HERO_POWER_NAMES`, `HERO_POWERS`, `chooseAction`) |
| `apps/web/src/wire/rng.ts` | new | `cp` of `packages/engine/src/rng.ts` (86 lines), its returned object also carrying `readonly seed` (the tutorial harness's lesson policy and `subsystems.chooseAction` use it) |
| `crates/engine/tests/export_config.rs` | new | writes apps/web/src/wire/engineConfig.ts (SURFACE §5.1: the 12 constants and 2 types the web imports), sorted, `export const NAME = <JSON>;` and the two `export type`s |
| `apps/web/src/game/engine.ts` | change | `EnginePort` over `../wasm` (synchronous after `loadWasm()`); REQUIRED_ENGINE_EXPORTS, setEnginePort/injectedEnginePort kept for tests, the lazy import goes |
| `apps/web/src/game/engine.real.ts` | delete | the WASM wrapper replaces it |
| `apps/web/src/practice/core.ts` | change | engine and AI calls through `../wasm`: `createRng(seed, cursor)` + `decide` → `wasm.decide(state, seat, {rngSeed, rngCursor, budget}, deadlineMs)` returning the new cursor; `buildAiDeck` likewise; `registerAll` goes |
| `apps/web/src/practice/practice.worker.ts, host.ts` | change | await `loadWasm()` before the first message; the in-thread host (jsdom) uses `loadWasmSync` |
| `apps/web/src/tutorial/lessons.test.ts` | change | delete `registerAll()` and its import: the WASM `init` registers the catalog (`emotes.ts` and `harness.ts` need no edit: their `@jackioh/ai` and `@jackioh/engine` imports resolve through the aliases) |
| `apps/web/src/cards/flavour.ts` | change | the `CardFlavour` type, `FLAVOUR_MAX_CHARS` and `ARTIST_MAX_CHARS` move here from `packages/cards/src/flavour.ts` (24 lines), which part 37 deletes with `packages/`; import them from here, not from `@jackioh/cards` |
| `apps/web/src/main.tsx` | change | `await loadWasm()` before the first render |
| `apps/web/src/game/net.ts` | change | delete the handler for a `legal` frame the server never sends (l.222, 260, 514) |
| `apps/web/src/test/setup.ts` | change | `loadWasmSync(readFileSync(new URL('../wasm/pkg/jackioh_wasm_bg.wasm', import.meta.url)))` |
| `apps/web/vite.config.ts, vitest.config.ts, tsconfig.json, package.json` | change | aliases of SURFACE §10.4; `server.fs.allow` the repo root; `optimizeDeps.exclude` the pkg; scripts `predev`/`prebuild`/`prebuild:e2e`/`pretest` run `sh ../../scripts/build-wasm.sh`; drop the `@jackioh/{ai,cards,engine,shared,validator}` deps |
| `the 23 web and 10 e2e files importing apps/server/src/config(.ts)` | change | import from `@jackioh/server-config` (alias → apps/web/src/wire/serverConfig.ts); list them with `grep -rlE 'server/src/config' apps/web/src e2e` |
| `e2e/support/tasks/replay-runner.ts` | change | spawn `target/release/jackioh replay` with the JSON on stdin (SURFACE §12) instead of importing TS engine source |
| `e2e/support/tasks/lessons-runner.ts` | change | engine config from `apps/web/src/wire/engineConfig.ts` |
| `e2e/cypress/component/*.cy.tsx (those importing packages/cards or packages/shared)` | change | `crates/cards/catalog.json` and the `@jackioh/*` aliases |
| `e2e/cypress/e2e/99-online-smoke.cy.ts` | change | `{ mode: "bo1", deckId }` instead of `{ deckIndex: 1 }` (l.163, 179, 264, 279) |
| `e2e/tsconfig.json, e2e/cypress/component/tsconfig.json` | change | the same aliases |
| `vercel.json, apps/web/public/_headers` | change | `'wasm-unsafe-eval'` in `script-src` (both, held equal by cloudflare-config.test.ts); vercel.json `installCommand` installs rustup minimal + the wasm target before `pnpm install` |
| `apps/web/src/practice/personas.ts` | copy | `cp` from `packages/ai/src/personas.ts` (362 lines), then add `EMOTE_TRIGGERS`, `EMOTE_REPLY_KEYS`, `AI_EMOTE`, `AI_PERSONAS` and their types from `packages/ai/src/config.ts` (part 17 leaves them out of the Rust AI) and point its imports at `../wasm` and `../wire`; cosmetic emote personas stay TypeScript; the original stays until part 37 deletes it |
| `apps/web/src/practice/personas.test.ts` | copy | `cp` from `packages/ai/test/personas.test.ts` (779 lines); drop l.756–776 (they read packages/ai source) and import from `./personas`; the original stays until part 37 deletes it |

Do **not** touch: any `Cargo.toml`, `mod.rs`, `main.rs` or `lib.rs` other than `crates/wasm/src/lib.rs` (part 1's), other parts' files, `packages/` and `apps/server/` (read only; part 37 deletes them).

### 3. Steps

1. Read SURFACE.md §5, §9, §10 fully, `docs/v0.3.0/research/C-server.md` §7 and the web files you change.
2. Write `crates/wasm/src/lib.rs`: each binding parses its JSON arguments with serde, calls the engine/AI/validator by SURFACE §6.1/§9 names, and returns `serde_json::to_string` of the result. `ai_decide` passes `should_stop = || js_sys::Date::now() >= deadline_ms` when `deadline_ms > 0`.
3. Write `scripts/build-wasm.sh` (idempotent; `set -eu`; skips the download when `.cache/bin/wasm-bindgen --version` already prints 0.2.129).
4. Write the TS wrapper and the wire layer; copy the helper modules from `packages/shared/src` byte for byte (they are the client's own now); add a web test that runs `codes.ts` against `crates/engine/tests/fixtures/code-input-cases.json` (the same fixture as the Rust test).
5. Change the engine seams (engine.ts, core.ts, worker, host, main.tsx), the aliases and the config imports. Keep every exported TS name the rest of the web uses.
6. Do not run `pnpm` or `cargo` in Wave 1; part 36 makes it green.
7. Push after every 5 files (README §3.2).

### 4. Tests

- The web suite (`vitest --project web`) is the test: unchanged except the deleted `legal`-frame case and the personas test copied in; part 36 runs it.
- The new codes parity test against the shared fixture.

### 5. Done when

- [ ] No file under `apps/web/src` or `e2e/` imports `packages/*` or `apps/server/src` by path (`grep -rnE "packages/|apps/server/src" apps/web/src e2e --include=*.ts --include=*.tsx` is empty).
- [ ] `.fullsend/notes/part-21.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- The WASM file adds about 1 MB (gzip) to the first load; measure it in part 36 and report it on #306.
- Practice saves on devices fold through the Rust engine; they resume only if `hash_state` is TS's (SURFACE §5.2), which the golden traces prove.

<!-- /jackioh-bot:plan -->
