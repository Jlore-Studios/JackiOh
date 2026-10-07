# v0.3.0 (part 37 of 40): cull: delete the TypeScript, rewrite the docs

Part 37 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 4 (cull) |
| Starts after | part 33, part 34, part 35, part 36 |
| Branch | `staging` (push straight to it; no pull request) |
| Builder | a Claude Code cloud session, Opus |
| Notes | a comment on this issue when done |

Unlabelled on purpose: #306 holds labels until a person says the run is ready. The Plan below is between the night bot's plan markers, so if a person later queues this issue for a bot, it builds from this plan without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Fullsend Phase 6 and the end of the TypeScript: delete the whole TypeScript tree (`packages/`, `apps/server/`, `ladder/`, `scripts/catalog-version.mjs`: every file in it is ported, copied, merged or marked `delete` in PORT-MAP.md), cull what nobody needs from the Rust (unreferenced exports, single-implementation abstractions, dead options, scratch), and rewrite the docs that describe the old layout so that CLAUDE.md, AGENTS.md, README.md, BUILD.md, REVIEW.md, ADDING_CARDS.md, architecture.md and the crate READMEs are true.

**How you work (fullsend Phase 6; full text in `.claude/skills/fullsend/agents/culler.md`).** Delete, run the suite, restore on red. Never delete a test.

### 2. Files to touch

| File | new/change/delete | What |
|---|---|---|
| `packages/, apps/server/, ladder/, scripts/catalog-version.mjs, .fullsend/` | delete | the whole TypeScript tree (every file in it is ported, copied, moved, merged or marked delete in PORT-MAP.md), and `.fullsend/` after the cull report |
| `package.json, pnpm-workspace.yaml, tsconfig.json, vitest.config.ts, eslint.config.js` | change | web and e2e only; the purity blocks go |
| `CLAUDE.md` | change | rules 3, 4, 5, 6, 9 restated for Rust (numbers unchanged, never renumber), Commands (cargo + the web's pnpm), Architecture (crates), Deployment (Docker on Render), the corrected e2e networked list (05, 06, 09, 10, 18, 19, 20, 26, 27, 35) |
| `AGENTS.md, README.md` | change | the Rust layout and commands; README's card counts (268 + 50) |
| `BUILD.md` | change | §1 layout, §4 test strategy and §5 definition of done for Rust; keep the milestone history and the per-card must-pass tables |
| `REVIEW.md` | change | Part B's B0, B1, B3, B4, B8 rewritten for crates and cargo; Part A unchanged |
| `docs/ADDING_CARDS.md` | change | the Rust card file, `ID`, tests in the file, `catalog loc`, the cargo gates |
| `docs/architecture.md` | change | the Rust server, Docker, the env contract minus Node |
| `crates/{engine,cards,ai,server,tools}/README.md` | new | the contracts the package READMEs held, for Rust |
| `apps/web/README.md, e2e/README.md` | change | the WASM seams and the Rust server |
| `apps/server/README.md` | delete | README content moves to crates/server/README.md (part 37) |
| `apps/server/package.json` | delete | PORT-MAP |
| `apps/server/test/validator-single-source.test.ts` | delete | one crate is one source |
| `apps/server/tsconfig.json` | delete | PORT-MAP |
| `apps/server/vitest.config.ts` | delete | PORT-MAP |
| `ladder/DECISIONS.md` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/README.md` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/agents/__init__.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/agents/random/__init__.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/agents/random/agent.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/arena/__init__.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/arena/bridge.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/arena/runner.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/arena/types.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/audit/__init__.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/audit/gates.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/audit/shadowban.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/champions/1/.gitkeep` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/champions/2/.gitkeep` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/common.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/config.yaml` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/hall_of_fame/.gitkeep` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/history/.gitkeep` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/llm/__init__.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/llm/client.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/llm/providers/__init__.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/llm/providers/anthropic.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/llm/providers/base.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/llm/providers/google.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/llm/providers/mock.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/llm/providers/openai.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/prompts/proposer.md` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/proposals/.gitkeep` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/proposer/__init__.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/proposer/propose.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/pyproject.toml` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/schemas/spec.schema.json` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/tests/__init__.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/tests/test_arena.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/tests/test_gates.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/tests/test_llm.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/tests/test_proposer.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `ladder/tests/test_shadowban.py` | delete | the Rust arena and training lanes replace it (part 29) |
| `packages/ai/README.md` | delete | PORT-MAP |
| `packages/ai/package.json` | delete | PORT-MAP |
| `packages/ai/scripts/arena-bridge.ts` | delete | the Rust arena replaces the Node bridge |
| `packages/ai/scripts/bench.ts` | delete | superseded by `arena` |
| `packages/ai/scripts/oracle.ts` | delete | dev tool, not ported |
| `packages/ai/test/_shard.ts` | delete | PORT-MAP |
| `packages/ai/test/arena-bridge.test.ts` | delete | PORT-MAP |
| `packages/ai/test/setup.ts` | delete | PORT-MAP |
| `packages/ai/tsconfig.json` | delete | PORT-MAP |
| `packages/ai/vitest.config.ts` | delete | PORT-MAP |
| `packages/cards/README.md` | delete | README content moves to crates/cards/README.md (part 37) |
| `packages/cards/package.json` | delete | PORT-MAP |
| `packages/cards/scripts/gen-loc.ts` | delete | loc frozen; `cargo jackioh catalog loc` prints a new card's count |
| `packages/cards/scripts/gen-registry.ts` | delete | build.rs (SURFACE §7.4) |
| `packages/cards/scripts/missing-tests.ts` | delete | build.rs (SURFACE §7.4) |
| `packages/cards/src/scripts/_generated.ts` | delete | build.rs generates the registry (SURFACE §7.4) |
| `packages/cards/test/globalSetup.ts` | delete | registry generation moved to build.rs |
| `packages/cards/test/loc.test.ts` | delete | loc is frozen data (SURFACE §7.5) |
| `packages/cards/tsconfig.json` | delete | PORT-MAP |
| `packages/cards/vitest.config.ts` | delete | PORT-MAP |
| `packages/engine/package.json` | delete | PORT-MAP |
| `packages/engine/scripts/rulings-coverage.ts` | delete | replaced by `cargo jackioh spec check` (part 28) |
| `packages/engine/test/fixtures/lint/date-now.ts` | delete | ESLint probe; clippy.toml replaces the lint (SURFACE §3) |
| `packages/engine/test/fixtures/lint/math-random.ts` | delete | ESLint probe; clippy.toml replaces the lint (SURFACE §3) |
| `packages/engine/test/fixtures/lint/new-date.ts` | delete | ESLint probe; clippy.toml replaces the lint (SURFACE §3) |
| `packages/engine/test/lint-ban.test.ts` | delete | tests the ESLint purity rule |
| `packages/engine/tsconfig.json` | delete | PORT-MAP |
| `packages/engine/vitest.config.ts` | delete | PORT-MAP |
| `packages/shared/package.json` | delete | PORT-MAP |
| `packages/shared/test/events.test.ts` | delete | reads its own source; the generated TS types replace it |
| `packages/shared/vitest.config.ts` | delete | PORT-MAP |
| `packages/validator/package.json` | delete | PORT-MAP |
| `packages/validator/src/config.ts` | delete | re-exported DECK_SIZE/MAX_COPIES; use crate::config |
| `packages/validator/tsconfig.json` | delete | PORT-MAP |
| `packages/validator/vitest.config.ts` | delete | PORT-MAP |
| `scripts/catalog-version.mjs` | delete | the server compiles the version in; `cargo jackioh catalog-version` for scripts |


### 3. Steps

1. Check that every destination exists before deleting any source: `awk -F'\t' 'NR>1 && $2!="delete" {print $4}' docs/v0.3.0/port-map.tsv | sort -u | while read d; do [ -e "$d" ] || echo "MISSING $d"; done` must print nothing. Then delete the whole TypeScript tree, since every file in it is ported, copied, moved, merged or marked delete: `git rm -r --quiet packages apps/server ladder scripts/catalog-version.mjs` (keep `.fullsend/` until step 3). Then make the root `package.json`, `pnpm-workspace.yaml`, `tsconfig.json`, `vitest.config.ts` and `eslint.config.js` changes of your table and run `pnpm install` so the lockfile drops the deleted workspaces.
2. Run the full suite (`cargo test --workspace`, web vitest, e2e smoke) — green before the cull.
3. Cull the Rust in fullsend's order (`.claude/skills/fullsend/agents/culler.md` verbatim): delete, run the suite, restore with `git checkout --` on red. Write `.fullsend/notes/cull-report.md` before deleting `.fullsend/` (paste it on #306).
4. List every V of README §6 that no test cites (fullsend coverage check) and add the missing tests.
5. Rewrite the docs; check every command in them by running it.

### 4. Tests

- The whole suite green after the cull (V28: no TS under packages/ or apps/server/; ladder/ gone).

### 5. Done when

- [ ] `git ls-files packages apps/server ladder` is empty; docs true; cull report on #306.

### 6. Risks

- A doc command that does not run is a bug: run each one.

<!-- /jackioh-bot:plan -->
