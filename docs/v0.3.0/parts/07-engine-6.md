# v0.3.0 (part 7 of 40): engine 6: effect verbs, second half

Part 7 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | part 1 |
| Branch | `staging` (push straight to it; no pull request) |
| Builder | a Claude Code cloud session, Sonnet or stronger |
| Notes | `.fullsend/notes/part-07.md` and `.assumptions` (SURFACE §16) |

Unlabelled on purpose: #306 holds labels until a person says the run is ready. The Plan below is between the night bot's plan markers, so if a person later queues this issue for a bot, it builds from this plan without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port the second half of the effect verbs (`effects/*`, alphabetical, as the table lists) from TypeScript to Rust, file for file (the table below), by SURFACE.md §4's rules, so that every function keeps its name (snake_cased), its module, its behaviour and its RNG draws. These modules are called by every other engine part, the cards and the AI by the names §4.2 gives them.

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
| `crates/engine/src/effects/last_board.rs` | new | port of `packages/engine/src/effects/lastBoard.ts` (104 lines). |
| `crates/engine/src/effects/library.rs` | new | port of `packages/engine/src/effects/library.ts` (94 lines). |
| `crates/engine/src/effects/library_copies.rs` | new | port of `packages/engine/src/effects/libraryCopies.ts` (41 lines). |
| `crates/engine/src/effects/locks.rs` | new | port of `packages/engine/src/effects/locks.ts` (145 lines). |
| `crates/engine/src/effects/lose_health.rs` | new | port of `packages/engine/src/effects/loseHealth.ts` (14 lines). |
| `crates/engine/src/effects/mana.rs` | new | port of `packages/engine/src/effects/mana.ts` (63 lines). |
| `crates/engine/src/effects/memory.rs` | new | port of `packages/engine/src/effects/memory.ts` (38 lines). |
| `crates/engine/src/effects/move_.rs` | new | port of `packages/engine/src/effects/move.ts` (398 lines). |
| `crates/engine/src/effects/perks.rs` | new | port of `packages/engine/src/effects/perks.ts` (43 lines). |
| `crates/engine/src/effects/plague.rs` | new | port of `packages/engine/src/effects/plague.ts` (232 lines). |
| `crates/engine/src/effects/player_mods.rs` | new | port of `packages/engine/src/effects/playerMods.ts` (43 lines). |
| `crates/engine/src/effects/position.rs` | new | port of `packages/engine/src/effects/position.ts` (35 lines). |
| `crates/engine/src/effects/radiant.rs` | new | port of `packages/engine/src/effects/radiant.ts` (257 lines). |
| `crates/engine/src/effects/random_picks.rs` | new | port of `packages/engine/src/effects/randomPicks.ts` (49 lines). |
| `crates/engine/src/effects/reveal.rs` | new | port of `packages/engine/src/effects/reveal.ts` (27 lines). |
| `crates/engine/src/effects/rotate.rs` | new | port of `packages/engine/src/effects/rotate.ts` (53 lines). |
| `crates/engine/src/effects/rounds.rs` | new | port of `packages/engine/src/effects/rounds.ts` (69 lines). |
| `crates/engine/src/effects/shuffle_card.rs` | new | port of `packages/engine/src/effects/shuffleCard.ts` (48 lines). |
| `crates/engine/src/effects/shuffle_into.rs` | new | port of `packages/engine/src/effects/shuffleInto.ts` (69 lines). |
| `crates/engine/src/effects/shuffle_random.rs` | new | port of `packages/engine/src/effects/shuffleRandom.ts` (38 lines). |
| `crates/engine/src/effects/split.rs` | new | port of `packages/engine/src/effects/split.ts` (47 lines). |
| `crates/engine/src/effects/statuses.rs` | new | port of `packages/engine/src/effects/statuses.ts` (48 lines). |
| `crates/engine/src/effects/steal.rs` | new | port of `packages/engine/src/effects/steal.ts` (118 lines). |
| `crates/engine/src/effects/summon.rs` | new | port of `packages/engine/src/effects/summon.ts` (553 lines). |
| `crates/engine/src/effects/summon_this.rs` | new | port of `packages/engine/src/effects/summonThis.ts` (21 lines). |
| `crates/engine/src/effects/swap.rs` | new | port of `packages/engine/src/effects/swap.ts` (270 lines). |
| `crates/engine/src/effects/targets.rs` | new | port of `packages/engine/src/effects/targets.ts` (250 lines). |
| `crates/engine/src/effects/transform.rs` | new | port of `packages/engine/src/effects/transform.ts` (319 lines). |
| `crates/engine/src/effects/tune.rs` | new | port of `packages/engine/src/effects/tune.ts` (569 lines). |
| `crates/engine/src/effects/turn_end.rs` | new | port of `packages/engine/src/effects/turnEnd.ts` (44 lines). |

Do **not** touch: any `Cargo.toml`, `lib.rs`, `mod.rs` or `main.rs` (part 1's), other parts' files, `packages/` and `apps/server/` (read only; part 37 deletes them).

### 3. Steps

1. Read SURFACE.md §3–§6 fully, then each TS file in your table (and, read-only, any TS file it imports, to know what it calls).
2. Each TS constructor `name(args) => ({ kind: "…", apply(ctx) { … }, expand?(ctx, memo) { … } })` becomes `pub fn name(args: Args) -> Effect` returning `Effect::new("…", move |ctx| { … })` (plus `.with_expand(…)` where TS has `expand`), SURFACE §6.6.
3. Every argument type (`TargetSpec`, `BoardScope`, `PlayerSpec`, `DamageEffectArgs`, …) derives `Serialize, Deserialize, Clone, Debug, PartialEq` with camelCase serde, and `Default` when all fields are optional, so cards can build them with `json_as(json!(…))` (SURFACE §6.6). Variant names of TS string unions by §4.3.
4. Exported constants (`EXILE_ZONE_ORDER`, `DELAYED_HOOK`, `SWAP_ROWS`, …) stay `pub const` in the same file. Prompt answerers registered by an effect file (plague placement, fuse onto) are plain `pub fn`s that `prompts.rs` calls by name.
5. For each row: write the Rust file. Port every function, exported or not, in TS order; keep the TS header comment as `//!` lines and every inline comment that states a rule or cites a ruling (`R113`, `§10.3`) — `spec check` reads the R-ids (§15).
6. Constants that TS declared at the top of your files now live in `crate::config` (part 1 moved them, SURFACE §6.4); use them from there.
7. A function another module provides is called by its TS name snake_cased at its TS module's path (`crate::zones::place_on_field`), taking `&GameState`, `&mut GameState` or `&mut EngineSink` by §6.5. Don't look at whether it exists yet.
8. Push after every 3–5 files (README §3.2).

### 4. Tests

- Unit tests are not yours: parts 24–27 port the engine's TS tests. Add none.
- Part 32 compiles and tests these files in Wave 3.

### 5. Done when

- [ ] Every destination in the table holds the full port of its TS file (no function missing that TS exported).
- [ ] No `todo!`, `unimplemented!`, placeholder bodies or `// TODO` in your files.
- [ ] `.fullsend/notes/part-07.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- SURFACE §4.4's semantics (stable sorts, insertion-ordered maps, presence of optional fields, RNG order) are where a port goes quietly wrong; the golden traces catch it in Wave 3, at the cost of a debugging session.

<!-- /jackioh-bot:plan -->
