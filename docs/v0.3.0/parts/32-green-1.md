# v0.3.0 (part 32 of 40): green 1: the engine compiles and replays the golden traces

Part 32 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 3a (green) |
| Starts after | part 31 |
| Branch | `staging` (push straight to it; no pull request) |
| Builder | a Claude Code cloud session, Opus |
| Notes | a comment on this issue when done |

Unlabelled on purpose: #306 holds labels until a person says the run is ready. The Plan below is between the night bot's plan markers, so if a person later queues this issue for a bot, it builds from this plan without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Make `jackioh-engine` compile with zero warnings under clippy, pass every ported engine test, replay every golden trace identically, and fold the hotseat fixture to "a798906b". Every other green part waits on this one.

**How you work (fullsend Phase 5; full text in `.claude/skills/fullsend/SKILL.md`).** Compile first, then tests. The ported tests and the golden traces are the spec.

### 2. Files to touch

| File | new/change/delete | What |
|---|---|---|
| `crates/engine/**` | change | anything needed |


### 3. Steps

1. `cargo check -p jackioh-engine --all-targets --features testkit` to zero errors, fixing by module in the port order of research A §1.0 (L0 up).
2. `cargo clippy -p jackioh-engine --all-targets --features testkit -- -D warnings` to zero.
3. `cargo test -p jackioh-engine --features testkit`: the unit tests (rng's ten draws, V3), `tests/rules` (parts 24–27), `tests/golden.rs` (V4).
4. Fullsend Phase 5 rules, non-negotiable: compile first, then tests; **a failing ported test means the implementation is wrong** unless you can show the test mis-ported its TypeScript original (then fix the port to match the TS test, and log the TS file and line in `.fullsend/notes/test-corrections.md`); never weaken, skip or `#[ignore]` a test; never edit a golden trace (only `golden bless` after an intended rules change, which there is none of in v0.3.0).
5. A golden divergence names seed, step and hash: run `pnpm exec tsx scripts/golden/record.ts --seed <k> --dump-step <n>` for TS's side, diff it with `target/golden-diff/`, fix the Rust.
6. You may change any file under the crates your part names; a fix in another crate goes in its own commit titled `<crate> fix: …` after `cargo test -p <that crate>` passes. Pull and push often (README §3.2); others are fixing in parallel.
7. Iteration cap: if the same failure survives three rounds, stop, write it to `.fullsend/notes/spec-gaps.md`, and comment on #306 (fullsend: "a fourth iteration means the spec has a hole").

### 4. Tests

- `cargo test -p jackioh-engine --features testkit` green; `cargo test -p jackioh-engine --features ts export_bindings` writes the TS types (committed under apps/web/src/wire/generated/).

### 5. Done when

- [ ] Engine green as above; tag `v0.3.0-wave-3a`; comment on #306 so parts 33–36 start.

### 6. Risks

- Golden divergences cluster: fix the earliest seed and step first; one root cause often explains dozens.

<!-- /jackioh-bot:plan -->
