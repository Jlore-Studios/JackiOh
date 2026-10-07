# v0.3.0 (part 40 of 40): generation 0, the sweep of record, and the lanes switched on

Part 40 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 5 (training lanes) |
| Starts after | part 38, part 39 |
| Branch | a pull request to `main` |
| Builder | a Claude Code cloud session, Sonnet or stronger |
| Notes | a comment on this issue when done |

Unlabelled on purpose: #306 holds labels until a person says the run is ready. The Plan below is between the night bot's plan markers, so if a person later queues this issue for a bot, it builds from this plan without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Measure generation 0 (the ported AI) with the Rust gates, run the shadow-ban sweep of record over all 268 cards that #65 left undone, write generation 0's record and the quality-gate numbers into the spec, and switch both training lanes on. The 11 banned cards stay banned (#306 allows it); the sweep's findings become the unban lane's starting notes.

**How you work.** Normal engineering: build and test as you go; a person does the steps that need dashboard access.

### 2. Files to touch

| File | new/change/delete | What |
|---|---|---|
| `crates/ai/generation.json` | change | generation 0's measured vsRandom (`gate --full`'s ai-vs-random) and the date |
| `spec/09-architecture.md (§9.9 Quality gates)` | change | the Rust counts of the three matchups |
| `training/history/sweep-<date>.md` | new | the sweep of record: flags per card and tier, the pass-2 seeds, which cards would be banned or watched |
| `crates/ai/src/shadow_ban.rs` | change | `SHADOW_WATCH` gets the cleared at-risk cards; `SHADOW_BAN` unchanged |


### 3. Steps

1. `cargo jackioh gate --full` (record all three counts) and `cargo jackioh sweep` (both passes, all 268 cards; note the seeds).
2. Write the files above in one pull request to `main`.
3. Ask the person who owns the training box to confirm both services run; watch the first promotion attempts' `promote --dry-run` numbers for a day.
4. Report on #306: generation 0's measured win rates against random and against itself, and what the 85/100 and 75/100 thresholds mean for the lanes (README §8's feasibility note) so a person can keep or change them.

### 4. Tests

- `gate --full` meets `gateNeeded` (V10).

### 5. Done when

- [ ] generation.json and §9.9 hold measured Rust numbers; the sweep of record is committed; both lanes have run one full iteration.

### 6. Risks

- The improve lane's 85/100 threshold may never be met by small changes; that is #306's rule, reported, not changed here.
- The lanes run from part 39 on, so one may promote generation 1 before this pull request merges. Then keep `main`'s `generation.json` as it is and put generation 0's measured numbers in the sweep file instead.

<!-- /jackioh-bot:plan -->
