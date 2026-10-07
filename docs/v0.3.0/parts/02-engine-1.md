# v0.3.0 (part 2 of 40): engine 1: the model and the board

Part 2 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | part 1 |
| Branch | `staging` (push straight to it; no pull request) |
| Builder | a Claude Code cloud session, Sonnet or stronger |
| Notes | `.fullsend/notes/part-02.md` and `.assumptions` (SURFACE §16) |

Unlabelled on purpose: #306 holds labels until a person says the run is ready. The Plan below is between the night bot's plan markers, so if a person later queues this issue for a bot, it builds from this plan without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port the model readers and the board (catalog and script registries, zones, layers, query, numbers, field statuses, preview) from TypeScript to Rust, file for file (the table below), by SURFACE.md §4's rules, so that every function keeps its name (snake_cased), its module, its behaviour and its RNG draws. These modules are called by every other engine part, the cards and the AI by the names §4.2 gives them.

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
| `crates/engine/src/animated.rs` | new | port of `packages/engine/src/animated.ts` (225 lines). |
| `crates/engine/src/announce.rs` | new | port of `packages/engine/src/announce.ts` (81 lines). |
| `crates/engine/src/book_swap.rs` | new | port of `packages/engine/src/bookSwap.ts` (35 lines). |
| `crates/engine/src/brittle.rs` | new | port of `packages/engine/src/brittle.ts` (92 lines). |
| `crates/engine/src/brittle_count.rs` | new | port of `packages/engine/src/brittleCount.ts` (98 lines). |
| `crates/engine/src/carriers.rs` | new | port of `packages/engine/src/carriers.ts` (73 lines). |
| `crates/engine/src/cast_on_draw_now.rs` | new | port of `packages/engine/src/castOnDrawNow.ts` (13 lines). |
| `crates/engine/src/catalog.rs` | new | port of `packages/engine/src/catalog.ts` (344 lines). registry is a OnceLock; transient defs read from state; no digest table |
| `crates/engine/src/draw_complete.rs` | new | port of `packages/engine/src/drawComplete.ts` (42 lines). |
| `crates/engine/src/enchantments.rs` | new | port of `packages/engine/src/enchantments.ts` (58 lines). |
| `crates/engine/src/faces.rs` | new | port of `packages/engine/src/faces.ts` (31 lines). |
| `crates/engine/src/kill_credit.rs` | new | port of `packages/engine/src/killCredit.ts` (23 lines). |
| `crates/engine/src/layers.rs` | new | port of `packages/engine/src/layers.ts` (324 lines). |
| `crates/engine/src/marks.rs` | new | port of `packages/engine/src/marks.ts` (92 lines). |
| `crates/engine/src/numbers.rs` | new | port of `packages/engine/src/numbers.ts` (193 lines). |
| `crates/engine/src/own_library.rs` | new | port of `packages/engine/src/ownLibrary.ts` (89 lines). |
| `crates/engine/src/ownership.rs` | new | port of `packages/engine/src/ownership.ts` (112 lines). |
| `crates/engine/src/params.rs` | new | port of `packages/engine/src/params.ts` (193 lines). |
| `crates/engine/src/plague.rs` | new | port of `packages/engine/src/plague.ts` (107 lines). |
| `crates/engine/src/preview.rs` | new | port of `packages/engine/src/preview.ts` (100 lines). |
| `crates/engine/src/query.rs` | new | port of `packages/engine/src/query.ts` (348 lines). |
| `crates/engine/src/restrictions.rs` | new | port of `packages/engine/src/restrictions.ts` (152 lines). registerAttackBar not ported |
| `crates/engine/src/scripts.rs` | new | port of `packages/engine/src/scripts.ts` (130 lines). |
| `crates/engine/src/stays.rs` | new | port of `packages/engine/src/stays.ts` (285 lines). |
| `crates/engine/src/temporary.rs` | new | port of `packages/engine/src/temporary.ts` (33 lines). |
| `crates/engine/src/times_played.rs` | new | port of `packages/engine/src/timesPlayed.ts` (24 lines). |
| `crates/engine/src/tuning.rs` | new | port of `packages/engine/src/tuning.ts` (200 lines). |
| `crates/engine/src/zones.rs` | new | port of `packages/engine/src/zones.ts` (874 lines). |

Do **not** touch: any `Cargo.toml`, `lib.rs`, `mod.rs` or `main.rs` (part 1's), other parts' files, `packages/` and `apps/server/` (read only; part 37 deletes them).

### 3. Steps

1. Read SURFACE.md §3–§6 fully, then each TS file in your table (and, read-only, any TS file it imports, to know what it calls).
2. `catalog.rs`: the catalog lives in a `OnceLock` set by `register_catalog(defs: CardDefs, version: &str)`; `registered_catalog()` and `def_of(state: Option<&GameState>, id)` read transient defs from `state.transient_defs` first (TS's "transient defs win"). The process-global digest table (`catalog.ts:181`) is not ported: a digest id's ingredients are read from the fused def in `state.transient_defs`. Under `#[cfg(feature = "testkit")]`, consult the testkit's thread-local override first (SURFACE §8).
3. `scripts.rs`: `register_scripts(map: IndexMap<String, CardScripts>)` into a `OnceLock`; `script_of(state: &GameState, def_id: &str) -> CardScripts` returns the registry's entry, or for a fused/crafted id calls `crate::subsystems::fuse::compose_fused_scripts(state, def)` (part 8); `flags_of` likewise. No `syncFusedScripts`.
4. `zones.rs`: the only zone movers (`place_on_field`, `move_to_zone`, `remove_from_any_zone`) and `slots_of`/`active_units_of` exactly as TS (lane order, rotation rings, locks, Stack piles). TS's `registerGraveyardRedirect` hook becomes a direct call to `crate::replacements::<the registered function's name, snake_cased>`.
5. `restrictions.rs`: `registerAttackBar` and its registry are not ported (nothing registers one).
6. `layers.rs`: computed on read, never cached (SPEC §10.4); same order of aura sources and stat mods as TS.
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
- [ ] `.fullsend/notes/part-02.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- SURFACE §4.4's semantics (stable sorts, insertion-ordered maps, presence of optional fields, RNG order) are where a port goes quietly wrong; the golden traces catch it in Wave 3, at the cost of a debugging session.

<!-- /jackioh-bot:plan -->
