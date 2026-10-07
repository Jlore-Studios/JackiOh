# v0.3.0 (part 36 of 40): green 5: the web client and e2e

Part 36 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 3b (green) |
| Starts after | part 32 |
| Branch | `staging` (push straight to it; no pull request) |
| Builder | a Claude Code cloud session, Sonnet or stronger |
| Notes | a comment on this issue when done |

Unlabelled on purpose: #306 holds labels until a person says the run is ready. The Plan below is between the night bot's plan markers, so if a person later queues this issue for a bot, it builds from this plan without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Make the WASM build, the web client and all e2e specs green, against the green engine of part 32.

**How you work (fullsend Phase 5; full text in `.claude/skills/fullsend/SKILL.md`).** Compile first, then tests. The ported tests and the golden traces are the spec.

### 2. Files to touch

| File | new/change/delete | What |
|---|---|---|
| `crates/wasm/**` | change | anything needed |
| `apps/web/**` | change | anything needed |
| `e2e/**` | change | anything needed |
| `scripts/build-wasm.sh` | change | anything needed |
| `vercel.json` | change | anything needed |


### 3. Steps

1. `sh scripts/build-wasm.sh`; `pnpm --dir apps/web typecheck`; `pnpm exec eslint apps/web e2e`; `pnpm vitest run --project web`; regenerate `apps/web/src/wire/generated/`, `engineConfig.ts` and `serverConfig.ts` and commit them (V20).
2. `pnpm build:e2e`, serve it, boot the Rust server with `E2E=1`, and run all 35 e2e specs on Chrome plus the component specs (V19, V21). Record the gzipped size of the `.wasm` on #306.
3. Fullsend Phase 5 rules, non-negotiable: compile first, then tests; **a failing ported test means the implementation is wrong** unless you can show the test mis-ported its TypeScript original (then fix the port to match the TS test, and log the TS file and line in `.fullsend/notes/test-corrections.md`); never weaken, skip or `#[ignore]` a test; never edit a golden trace (only `golden bless` after an intended rules change, which there is none of in v0.3.0).
4. A golden divergence names seed, step and hash: run `pnpm exec tsx scripts/golden/record.ts --seed <k> --dump-step <n>` for TS's side, diff it with `target/golden-diff/`, fix the Rust.
5. You may change any file under the crates your part names; a fix in another crate goes in its own commit titled `<crate> fix: …` after `cargo test -p <that crate>` passes. Pull and push often (README §3.2); others are fixing in parallel.
6. Iteration cap: if the same failure survives three rounds, stop, write it to `.fullsend/notes/spec-gaps.md`, and comment on #306 (fullsend: "a fourth iteration means the spec has a hole").

### 4. Tests

- Web suite green; all e2e specs green on Chrome.
- `apps/web/src/cards/rules.test.ts` fails when a term row is added to a `spec/06-keywords.md` table without a glossary entry, and passes when a sentence of a row is reworded (try both, revert): part 28's promise (#133).

### 5. Done when

- [ ] Web and e2e green; tag `v0.3.0-wave-3` once 33–36 are all green.

<!-- /jackioh-bot:plan -->
