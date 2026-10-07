# v0.3.0 (part 31 of 40): contact and reconcile

Part 31 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 2 (contact and reconcile) |
| Starts after | parts 2–29 |
| Branch | `staging` (push straight to it; no pull request) |
| Builder | a Claude Code cloud session, Opus |
| Notes | a comment on this issue when done |

Unlabelled on purpose: #306 holds labels until a person says the run is ready. The Plan below is between the night bot's plan markers, so if a person later queues this issue for a bot, it builds from this plan without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Fullsend Phases 3 and 4 over `staging`: build once, classify the damage without fixing it, then resolve every collision by picking one winner and deleting the losers, settle every semantic conflict against SURFACE.md, and add what the parts listed under GAPS (missing fields, modules, dependencies). Not green: Wave 3 does that.

**How you work (fullsend Phases 3–4; full text in `.claude/skills/fullsend/SKILL.md` and `agents/reconciler.md`).** Classify before fixing anything. Pick one winner, delete every loser, update callers; never merge two designs or add a compatibility shim. Do not chase green.

### 2. Files to touch

| File | new/change/delete | What |
|---|---|---|
| `.fullsend/damage/raw.txt, .fullsend/damage.md` | new | the first build's output and the four lists |
| `.fullsend/notes/reconcile-decisions.md` | new | one entry per cluster (fullsend reconciler template) |
| `any file under crates/` | change | only to delete a loser, update its callers, or add a GAPS item |


### 3. Steps

1. Tag `v0.3.0-wave-1` on staging's head. `grep -h BUILDS-RUN .fullsend/notes/*.md`: every value must be 0; a part with more is noted (its code is still used).
2. `cargo build --workspace --all-targets 2>&1 | tee .fullsend/damage/raw.txt`. Hundreds of errors are expected.
3. Duplicate symbols: run the fullsend grep (`.claude/skills/fullsend/SKILL.md`, Phase 3) over `crates/`. Semantic conflicts: `cat .fullsend/notes/*.assumptions | sort -u | cut -d: -f1 | uniq -c | awk '$1>1'`.
4. Write `.fullsend/damage.md`: Collisions (concept, every location), Seams (calls to names that do not exist, or exist with another signature), Drift (deps, features), Semantic conflicts (key, values, parts).
5. Reconcile in waves of disjoint file sets (run subagents per cluster if you have them, each with `.claude/skills/fullsend/agents/reconciler.md` verbatim): score by the fullsend formula, keep one, delete the rest, update callers, never merge or shim. Settle conflicts by SURFACE.md; where it is silent, the TS decides.
6. Fix seams by renaming callers to the existing name when SURFACE §4.2 and the TS agree; add the GAPS items (fields to state types, `pub mod` lines, workspace deps from SURFACE §2 only).
7. Read the test porters' gaps (`.fullsend/notes/spec-gaps-part-*.md`, parts 24–27) beside the builders' GAPS lists. Patch SURFACE.md only for a real hole, listed in `.fullsend/notes/spec-gaps.md`, and say so on #306.
8. Commit per wave of clusters; tag `v0.3.0-wave-2` at the end.

### 4. Tests

- None run green yet. `git diff --shortstat v0.3.0-wave-1..HEAD -- crates` shows more deletions than insertions (fullsend's merge tell).

### 5. Done when

- [ ] damage.md and reconcile-decisions.md committed; no concept implemented twice in `crates/`; every GAPS item added or answered.
- [ ] Tag `v0.3.0-wave-2` pushed.

### 6. Risks

- Error count may rise as deletions expose seams. That is correct; do not chase green here.

<!-- /jackioh-bot:plan -->
