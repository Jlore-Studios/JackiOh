# v0.3.0 (part 3 of 40): engine 2: combat, damage and the resolution loop

Part 3 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | part 1 |
| Branch | `staging` (push straight to it; no pull request) |
| Builder | a Claude Code cloud session, Opus |
| Notes | `.fullsend/notes/part-03.md` and `.assumptions` (SURFACE §16) |

Unlabelled on purpose: #306 holds labels until a person says the run is ready. The Plan below is between the night bot's plan markers, so if a person later queues this issue for a bot, it builds from this plan without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port combat, damage, replacements, targeting and the resolution loop (work, resolve, prompts, triggers, traps, the state check, modifiers) from TypeScript to Rust, file for file (the table below), by SURFACE.md §4's rules, so that every function keeps its name (snake_cased), its module, its behaviour and its RNG draws. These modules are called by every other engine part, the cards and the AI by the names §4.2 gives them.

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
| `crates/engine/src/combat.rs` | new | port of `packages/engine/src/combat.ts` (1043 lines). |
| `crates/engine/src/damage.rs` | new | port of `packages/engine/src/damage.ts` (427 lines). |
| `crates/engine/src/modifiers.rs` | new | port of `packages/engine/src/modifiers.ts` (240 lines). |
| `crates/engine/src/prompts.rs` | new | port of `packages/engine/src/prompts.ts` (974 lines). answerer registry → one `match` |
| `crates/engine/src/replacements.rs` | new | port of `packages/engine/src/replacements.ts` (584 lines). `converting` moves to EngineSink |
| `crates/engine/src/resolve.rs` | new | port of `packages/engine/src/resolve.ts` (206 lines). |
| `crates/engine/src/state_check.rs` | new | port of `packages/engine/src/stateCheck.ts` (662 lines). |
| `crates/engine/src/targeting.rs` | new | port of `packages/engine/src/targeting.ts` (166 lines). |
| `crates/engine/src/targeting_point.rs` | new | port of `packages/engine/src/targetingPoint.ts` (172 lines). |
| `crates/engine/src/traps.rs` | new | port of `packages/engine/src/traps.ts` (766 lines). |
| `crates/engine/src/triggers.rs` | new | port of `packages/engine/src/triggers.ts` (853 lines). |
| `crates/engine/src/work.rs` | new | port of `packages/engine/src/work.ts` (593 lines). handler registry → one `match` on resume.hook (SURFACE §6.6) |

Do **not** touch: any `Cargo.toml`, `lib.rs`, `mod.rs` or `main.rs` (part 1's), other parts' files, `packages/` and `apps/server/` (read only; part 37 deletes them).

### 3. Steps

1. Read SURFACE.md §3–§6 fully, then each TS file in your table (and, read-only, any TS file it imports, to know what it calls).
2. `work.rs`: TS's handler registry becomes one dispatcher: `match resume.hook.as_str()` over the hooks of research A §2.7 (`"@setup"` → `crate::setup::…`, `"@deaths"` → `crate::state_check::…`, `"@trapWindow"`/`"@trapFiring"` → `crate::traps::…`, `"play"` → `crate::play_steps::…`, `"@drawChain"`/`"@drawCount"` → `crate::draw::…`, `"@attackWindow"`, `"@forcedRun"`, `"@forcedRandom"`, `"@afterAttack"` → `crate::combat::…`, `"@startOfTurn"`/`"@endOfTurn"` → `crate::turn::…`, `"@activate"` → `crate::subsystems::activate::…`, `"@aiTurn"` → `crate::subsystems::ai_policy::…`, `"@delayedDestroy"`/`"@delayedDiscardHand"` → `crate::effects::delay::…`, default → the card's own step via `script_step_for`). For each arm, grep the TS for the `registerWorkHandler("<hook>", fn)` call and call that `fn` snake_cased in its module.
3. `prompts.rs`: the answerer map (`prompts.ts:427`) becomes a `match` on the answerer key: `"play"` → `crate::play_steps::<registered fn>`, `"plague:placement"` → `crate::effects::plague::…`, `"fuse:onto"` → `crate::effects::fuse::…`, `"@triggerCry"` → `crate::cry_trigger::…`; default re-enters a card script step. `promptAnswers` enumerates in index order, capped at `MAX_PROMPT_ANSWERS`; `pickAnswers` as TS.
4. `resolve.rs`: `EngineSink` is SURFACE §6.5's struct (part 1 declared it in `state.rs` or `script.rs`; use it). The cast driver (`registerCastDriver`) is a direct call to `crate::play_steps::<registered fn>`; targeting hooks (`registerTargetingHooks`) direct calls into `crate::targeting_point::…`; the declaration check (`registerDeclarationCheck`) a direct call into `crate::traps::…`.
5. `replacements.rs`: TS's module-level `let converting` becomes `sink.converting` (§6.5).
6. `triggers.rs`: `settle` in R68 order; library holders sorted by `creation_number` (`/^c(\d+)$/`, else `i64::MAX`) then id, stably.
7. For each row: write the Rust file. Port every function, exported or not, in TS order; keep the TS header comment as `//!` lines and every inline comment that states a rule or cites a ruling (`R113`, `§10.3`) — `spec check` reads the R-ids (§15).
8. Constants that TS declared at the top of your files now live in `crate::config` (part 1 moved them, SURFACE §6.4); use them from there.
9. A function another module provides is called by its TS name snake_cased at its TS module's path (`crate::zones::place_on_field`), taking `&GameState`, `&mut GameState` or `&mut EngineSink` by §6.5. Don't look at whether it exists yet.
10. Push after every 3–5 files (README §3.2).

### 4. Tests

- Unit tests are not yours: parts 24–27 port the engine's TS tests. Add none.
- Part 32 compiles and tests these files in Wave 3.

### 5. Done when

- [ ] Every destination in the table holds the full port of its TS file (no function missing that TS exported).
- [ ] No `todo!`, `unimplemented!`, placeholder bodies or `// TODO` in your files.
- [ ] `.fullsend/notes/part-03.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- The resolution loop is the heart of trace parity: preserve the exact order in which work items, triggers and dispatch entries are pushed and popped (R113 cursor, R68 order).
- SURFACE §4.4's semantics (stable sorts, insertion-ordered maps, presence of optional fields, RNG order) are where a port goes quietly wrong; the golden traces catch it in Wave 3, at the cost of a debugging session.

<!-- /jackioh-bot:plan -->
