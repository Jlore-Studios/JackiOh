# v0.3.0 (part 33 of 40): green 2: the cards

Part 33 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

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

Make `jackioh-cards` (318 scripts and their tests, the cross-card suites) green, against the green engine of part 32.

**How you work (fullsend Phase 5; full text in `.claude/skills/fullsend/SKILL.md`).** Compile first, then tests. The ported tests and the golden traces are the spec.

### 2. Files to touch

| File | new/change/delete | What |
|---|---|---|
| `crates/cards/**` | change | anything needed |


### 3. Steps

1. `cargo check -p jackioh-cards --all-targets`, then clippy -D warnings, then `cargo test -p jackioh-cards`. `build.rs`'s missing-file check becomes an error now (318 files).
2. Fullsend Phase 5 rules, non-negotiable: compile first, then tests; **a failing ported test means the implementation is wrong** unless you can show the test mis-ported its TypeScript original (then fix the port to match the TS test, and log the TS file and line in `.fullsend/notes/test-corrections.md`); never weaken, skip or `#[ignore]` a test; never edit a golden trace (only `golden bless` after an intended rules change, which there is none of in v0.3.0).
3. A golden divergence names seed, step and hash: run `pnpm exec tsx scripts/golden/record.ts --seed <k> --dump-step <n>` for TS's side, diff it with `target/golden-diff/`, fix the Rust.
4. You may change any file under the crates your part names; a fix in another crate goes in its own commit titled `<crate> fix: …` after `cargo test -p <that crate>` passes. Pull and push often (README §3.2); others are fixing in parallel.
5. Iteration cap: if the same failure survives three rounds, stop, write it to `.fullsend/notes/spec-gaps.md`, and comment on #306 (fullsend: "a fourth iteration means the spec has a hole").

### 4. Tests

- `cargo test -p jackioh-cards` green (V5).

### 5. Done when

- [ ] Cards green; comment on #306.

<!-- /jackioh-bot:plan -->
