# v0.3.0 (part 8 of 40): engine 7: subsystems

Part 8 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | part 1 |
| Branch | `staging` (push straight to it; no pull request) |
| Builder | a Claude Code cloud session, Opus |
| Notes | `.fullsend/notes/part-08.md` and `.assumptions` (SURFACE §16) |

Unlabelled on purpose: #306 holds labels until a person says the run is ready. The Plan below is between the night bot's plan markers, so if a person later queues this issue for a bot, it builds from this plan without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port the subsystems (`subsystems/*`: Fuse and Craft, Heroic Power, Activate, quests, the Zephyrs scorer, Call to Chaos, the random policy, rotation, board history, lethal, KY's machinery, Glitch, last boards, audits) from TypeScript to Rust, file for file (the table below), by SURFACE.md §4's rules, so that every function keeps its name (snake_cased), its module, its behaviour and its RNG draws. These modules are called by every other engine part, the cards and the AI by the names §4.2 gives them.

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
| `crates/engine/src/subsystems/activate.rs` | new | port of `packages/engine/src/subsystems/activate.ts` (568 lines). |
| `crates/engine/src/subsystems/ai_policy.rs` | new | port of `packages/engine/src/subsystems/aiPolicy.ts` (205 lines). |
| `crates/engine/src/subsystems/audit.rs` | new | port of `packages/engine/src/subsystems/audit.ts` (40 lines). |
| `crates/engine/src/subsystems/board_history.rs` | new | port of `packages/engine/src/subsystems/boardHistory.ts` (194 lines). |
| `crates/engine/src/subsystems/call_to_chaos.rs` | new | port of `packages/engine/src/subsystems/callToChaos.ts` (394 lines). |
| `crates/engine/src/subsystems/call_to_chaos_plus.rs` | new | port of `packages/engine/src/subsystems/callToChaosPlus.ts` (139 lines). |
| `crates/engine/src/subsystems/combo_index.rs` | new | port of `packages/engine/src/subsystems/comboIndex.ts` (266 lines). |
| `crates/engine/src/subsystems/copied_text.rs` | new | port of `packages/engine/src/subsystems/copiedText.ts` (136 lines). |
| `crates/engine/src/subsystems/fuse.rs` | new | port of `packages/engine/src/subsystems/fuse.ts` (1171 lines). no syncFusedScripts, no global registry: fused scripts built on lookup (SURFACE §6.6) |
| `crates/engine/src/subsystems/glitch.rs` | new | port of `packages/engine/src/subsystems/glitch.ts` (132 lines). |
| `crates/engine/src/subsystems/hero_power.rs` | new | port of `packages/engine/src/subsystems/heroPower.ts` (590 lines). |
| `crates/engine/src/subsystems/ky_test.rs` | new | port of `packages/engine/src/subsystems/kyTest.ts` (110 lines). |
| `crates/engine/src/subsystems/last_boards.rs` | new | port of `packages/engine/src/subsystems/lastBoards.ts` (106 lines). |
| `crates/engine/src/subsystems/lethal.rs` | new | port of `packages/engine/src/subsystems/lethal.ts` (172 lines). |
| `crates/engine/src/subsystems/papaya.rs` | new | port of `packages/engine/src/subsystems/papaya.ts` (141 lines). |
| `crates/engine/src/subsystems/perfect_hand.rs` | new | port of `packages/engine/src/subsystems/perfectHand.ts` (49 lines). |
| `crates/engine/src/subsystems/quests.rs` | new | port of `packages/engine/src/subsystems/quests.ts` (544 lines). |
| `crates/engine/src/subsystems/rotation.rs` | new | port of `packages/engine/src/subsystems/rotation.ts` (197 lines). |
| `crates/engine/src/subsystems/scorer.rs` | new | port of `packages/engine/src/subsystems/scorer.ts` (503 lines). `dryRunning` moves to EngineSink (SURFACE §6.5) |
| `crates/engine/src/subsystems/twice_forward.rs` | new | port of `packages/engine/src/subsystems/twiceForward.ts` (120 lines). |

Do **not** touch: any `Cargo.toml`, `lib.rs`, `mod.rs` or `main.rs` (part 1's), other parts' files, `packages/` and `apps/server/` (read only; part 37 deletes them).

### 3. Steps

1. Read SURFACE.md §3–§6 fully, then each TS file in your table (and, read-only, any TS file it imports, to know what it calls).
2. `fuse.rs`: port the fused-card machinery, but **no `syncFusedScripts` and no registry writes**: write `pub fn compose_fused_scripts(state: &GameState, def: &CardDef) -> CardScripts`, which builds the fused card's scripts from its ingredients' scripts (`crate::scripts::script_of`) with TS's `combineObjects` rules (fuse.ts:663: key union in insertion order, how each key combines), building composed hooks as closures (`hook(move |ctx| …)`). `fusedDigest` (fuse.ts:360-370, two 32-bit lanes over UTF-16 code units, `wrapping_mul`) bit for bit.
3. `scorer.rs`: the dry runs clone the state, run `crate::play_steps::run_play_steps` and `crate::triggers::settle` with `sink.dry_running = true` (no module `let`), the side rng `Rng::new("zephyrs-dry-run", 0)`, and sort with the stable comparators of scorer.ts:260, :483-497.
4. `ai_policy.rs`: `choose_action(state, player, rng)` (§10.7's random policy: `AI_SKIPPED_ACTIONS`, `AI_END_TURN_PROBABILITY`) draw for draw as TS; the fuzz, the golden recorder and the training lanes' random opponent all use it.
5. `hero_power.rs`: the 13 powers as Activate abilities (R752–R761); `power_ability_of` used by `reduce`'s `activatePower` alias.
6. `audit.rs`: reads the catalog's frozen `loc` field (SURFACE §7.5).
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
- [ ] `.fullsend/notes/part-08.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- Fuse is the one place the Rust design departs from TS (no global re-sync). Composition order must equal what `combineObjects` produced, or fused cards diverge in the golden traces.
- SURFACE §4.4's semantics (stable sorts, insertion-ordered maps, presence of optional fields, RNG order) are where a port goes quietly wrong; the golden traces catch it in Wave 3, at the cost of a debugging session.

<!-- /jackioh-bot:plan -->
