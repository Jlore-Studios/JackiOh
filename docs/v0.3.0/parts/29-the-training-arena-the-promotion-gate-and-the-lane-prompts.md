# v0.3.0 (part 29 of 40): the training arena, the promotion gate and the lane prompts

Part 29 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | nothing (the bot works from `main`) |
| Branch | the night bot's own branch and pull request to `main` (new files only; the orchestrator merges `main` into `staging`) |
| Builder | the night bot, `difficulty:hard` (Opus builds and reviews) |
| Notes | `.fullsend/notes/part-29.md` and `.assumptions` (SURFACE §16) |

Queued for the night bot by #306's orchestrator (`bot:build`, `difficulty:hard`). The Plan below is between the night bot's plan markers: build from it without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Build the repository side of the AI training lanes (README §8, SURFACE §14): `cargo jackioh arena`, `agent` and `promote`, the standing prompts Devin runs from, the loop script the training box's services run, and generation 0's record.

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
| `crates/tools/src/arena.rs` | new | SURFACE §14.2; referee holding the true state; agents `self`, `random` (§10.7's policy via jackioh_ai::random_action), `bin:<path>` (spawns `<path> agent`, JSON lines); one GameRecord per game to --out; port of `packages/ai/scripts/duel.ts` (129 lines). |
| `crates/tools/src/agent.rs` | new | SURFACE §14.1 |
| `crates/tools/src/promote.rs` | new | SURFACE §14.2: the gate table, seeds from `git rev-parse HEAD:crates/ai/src`, generation.json + history line, --dry-run, --verify |
| `crates/ai/generation.json` | new | {"generation": 0, "lane": "port", "parent": null, "vsRandom": null, "vsParent": null, "shadowBan": 11, "source": "packages/ai at 91cc43c (94/100, 35/50, 47/50 in TS)"} |
| `training/README.md` | new | how the lanes run, what they may change (crates/ai/** and training/history/** only), the gate table, where stats go, how a person stops a lane (`systemctl stop jackioh-train@<lane>`), how to read history |
| `training/improve.md` | new | the improve lane's standing Devin prompt (below) |
| `training/unban.md` | new | the unban lane's standing Devin prompt (below) |
| `training/loop.sh` | new | README §8's loop, POSIX sh, `set -u`, logs to ~/logs/<lane>.log |
| `training/history/improve.jsonl, unban.jsonl` | new | empty |

Do **not** touch: any `Cargo.toml`, `lib.rs`, `mod.rs` or `main.rs` (part 1's), other parts' files, `packages/` and `apps/server/` (read only; part 37 deletes them).

### 3. Steps

1. Read README §8 and SURFACE §14 fully, then `packages/ai/src/{match,gate,devRun}.ts` and `ladder/` (for its seat alternation and logging, which you replace).
2. Write `arena.rs`, `agent.rs`, `promote.rs`. Run every game of a promotion on rayon threads (`RAYON_NUM_THREADS` respected); results do not depend on thread count.
3. Write the two standing prompts. Each tells Devin, in this order: you are improving JackiOh's Rust AI in `crates/ai/` (improve: to beat its predecessor 85/100 and random 90/100; unban: to play well with fewer shadow-banned cards, removing entries from `crates/ai/src/shadow_ban.rs` while beating its predecessor 75/100 and random 90/100); you may change only `crates/ai/**`; never change the engine, cards, tests outside crates/ai, or thresholds; measure with `cargo jackioh promote --lane <lane> --parent-bin ~/parent-jackioh --dry-run`; read `training/history/<lane>.jsonl` and `~/training-out/<lane>/` for what was tried; keep each attempt small and note it in `~/training-out/<lane>/attempts.md`; commit only after a non-dry-run `promote` exits 0, with the message `AI gen <N> (<lane>): <what changed>`; stop the session after a promotion or after 4 hours.
4. Write `loop.sh` exactly as README §8 steps 1–4 (fetch, reset `ai/<lane>` to `origin/main`, build the parent binary, run Devin with `--prompt-file training/<lane>.md`, on a promotion commit rebase, re-check the parent, push and `gh pr create --base main --head ai/<lane> --title … --body-file <promote output>` then `gh pr merge --auto --squash`; sleep 60; repeat).
5. Commit on your branch and open your pull request.

### 4. Tests

- `promote.rs` unit tests (V26): the gate table for both lanes with synthetic counts, including the strictly-fewer ban rule and draws counting as losses; the seed derivation is stable across a generation.json-only commit.
- `arena.rs`: two `self` agents over 4 games are deterministic (same seeds → same GameRecords).

### 5. Done when

- [ ] `cargo jackioh arena|agent|promote` exist with SURFACE §14's flags; prompts and loop committed; `crates/ai/generation.json` is generation 0.
- [ ] `.fullsend/notes/part-29.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- An agent binary from an older commit may not parse a newer state: `loop.sh` always rebuilds the parent from current `main`, and CI rejects an `ai/*` branch that touches anything outside `crates/ai/` and `training/history/` (part 30).

<!-- /jackioh-bot:plan -->
