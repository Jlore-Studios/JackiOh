# v0.3.0 (part 12 of 40): cards lane 4: Core tokens Rush to Sheep, Classic #1–#32

Part 12 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | nothing (the bot works from `main`) |
| Branch | the night bot's own branch and pull request to `main` (new files only; the orchestrator merges `main` into `staging`) |
| Builder | the night bot, `difficulty:hard` (Opus builds and reviews) |
| Notes | `.fullsend/notes/part-12.md` and `.assumptions` (SURFACE §16) |

Queued for the night bot by #306's orchestrator (`bot:build`, `difficulty:hard`). The Plan below is between the night bot's plan markers: build from it without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port every card in cards lane 4 (the table below, in order): each TypeScript script and its test file become one Rust file holding the script and, at the bottom, its tests (SURFACE.md §7 and §8). The behaviour must match the TypeScript exactly; the golden traces (part 23) and these tests decide it in Wave 3.

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
| `crates/cards/src/scripts/classic/c001_curse_of_the_forgotten_classic.rs` | new | script and tests from `packages/cards/src/scripts/classic/001-curse-of-the-forgotten-classic.ts` (90) + `packages/cards/test/classic/001-curse-of-the-forgotten-classic.test.ts` (249) |
| `crates/cards/src/scripts/classic/c002_the_trickster.rs` | new | script and tests from `packages/cards/src/scripts/classic/002-the-trickster.ts` (37) + `packages/cards/test/classic/002-the-trickster.test.ts` (206) |
| `crates/cards/src/scripts/classic/c003_book_of_heal.rs` | new | script and tests from `packages/cards/src/scripts/classic/003-book-of-heal.ts` (37) + `packages/cards/test/classic/003-book-of-heal.test.ts` (127) |
| `crates/cards/src/scripts/classic/c004_palantir.rs` | new | script and tests from `packages/cards/src/scripts/classic/004-palantir.ts` (86) + `packages/cards/test/classic/004-palantir.test.ts` (416) |
| `crates/cards/src/scripts/classic/c005_tesla.rs` | new | script and tests from `packages/cards/src/scripts/classic/005-tesla.ts` (65) + `packages/cards/test/classic/005-tesla.test.ts` (299) |
| `crates/cards/src/scripts/classic/c006_cloaked_toe_cracker.rs` | new | script and tests from `packages/cards/src/scripts/classic/006-cloaked-toe-cracker.ts` (50) + `packages/cards/test/classic/006-cloaked-toe-cracker.test.ts` (229) |
| `crates/cards/src/scripts/classic/c007_infiniscepter.rs` | new | script and tests from `packages/cards/src/scripts/classic/007-infiniscepter.ts` (76) + `packages/cards/test/classic/007-infiniscepter.test.ts` (314) |
| `crates/cards/src/scripts/classic/c008_pickle.rs` | new | script and tests from `packages/cards/src/scripts/classic/008-pickle.ts` (144) + `packages/cards/test/classic/008-pickle.test.ts` (330) |
| `crates/cards/src/scripts/classic/c009_income_tax.rs` | new | script and tests from `packages/cards/src/scripts/classic/009-income-tax.ts` (86) + `packages/cards/test/classic/009-income-tax.test.ts` (388) |
| `crates/cards/src/scripts/classic/c010_exile.rs` | new | script and tests from `packages/cards/src/scripts/classic/010-exile.ts` (89) + `packages/cards/test/classic/010-exile.test.ts` (324) |
| `crates/cards/src/scripts/classic/c011_mind_melt.rs` | new | script and tests from `packages/cards/src/scripts/classic/011-mind-melt.ts` (52) + `packages/cards/test/classic/011-mind-melt.test.ts` (243) |
| `crates/cards/src/scripts/classic/c012_book_of_blood.rs` | new | script and tests from `packages/cards/src/scripts/classic/012-book-of-blood.ts` (33) + `packages/cards/test/classic/012-book-of-blood.test.ts` (168) |
| `crates/cards/src/scripts/classic/c013_boots_on_the_ground.rs` | new | script and tests from `packages/cards/src/scripts/classic/013-boots-on-the-ground.ts` (39) + `packages/cards/test/classic/013-boots-on-the-ground.test.ts` (247) |
| `crates/cards/src/scripts/classic/c014_shadowstep.rs` | new | script and tests from `packages/cards/src/scripts/classic/014-shadowstep.ts` (97) + `packages/cards/test/classic/014-shadowstep.test.ts` (299) |
| `crates/cards/src/scripts/classic/c015_nose_hunter.rs` | new | script and tests from `packages/cards/src/scripts/classic/015-nose-hunter.ts` (47) + `packages/cards/test/classic/015-nose-hunter.test.ts` (270) |
| `crates/cards/src/scripts/classic/c016_book_of_flame.rs` | new | script and tests from `packages/cards/src/scripts/classic/016-book-of-flame.ts` (32) + `packages/cards/test/classic/016-book-of-flame.test.ts` (124) |
| `crates/cards/src/scripts/classic/c017_counterspell.rs` | new | script and tests from `packages/cards/src/scripts/classic/017-counterspell.ts` (65) + `packages/cards/test/classic/017-counterspell.test.ts` (271) |
| `crates/cards/src/scripts/classic/c018_glitch_in_the_system.rs` | new | script and tests from `packages/cards/src/scripts/classic/018-glitch-in-the-system.ts` (81) + `packages/cards/test/classic/018-glitch-in-the-system.test.ts` (278) |
| `crates/cards/src/scripts/classic/c019_lizards_breath.rs` | new | script and tests from `packages/cards/src/scripts/classic/019-lizards-breath.ts` (79) + `packages/cards/test/classic/019-lizards-breath.test.ts` (229) |
| `crates/cards/src/scripts/classic/c020_the_power_to_punish.rs` | new | script and tests from `packages/cards/src/scripts/classic/020-the-power-to-punish.ts` (75) + `packages/cards/test/classic/020-the-power-to-punish.test.ts` (387) |
| `crates/cards/src/scripts/classic/c021_turtinator.rs` | new | script and tests from `packages/cards/src/scripts/classic/021-turtinator.ts` (43) + `packages/cards/test/classic/021-turtinator.test.ts` (275) |
| `crates/cards/src/scripts/classic/c022_mid_runner.rs` | new | script and tests from `packages/cards/src/scripts/classic/022-mid-runner.ts` (88) + `packages/cards/test/classic/022-mid-runner.test.ts` (230) |
| `crates/cards/src/scripts/classic/c023_devils_pact.rs` | new | script and tests from `packages/cards/src/scripts/classic/023-devils-pact.ts` (61) + `packages/cards/test/classic/023-devils-pact.test.ts` (337) |
| `crates/cards/src/scripts/classic/c024_book_of_knowledge.rs` | new | script and tests from `packages/cards/src/scripts/classic/024-book-of-knowledge.ts` (24) + `packages/cards/test/classic/024-book-of-knowledge.test.ts` (157) |
| `crates/cards/src/scripts/classic/c025_lag_in_the_system.rs` | new | script and tests from `packages/cards/src/scripts/classic/025-lag-in-the-system.ts` (63) + `packages/cards/test/classic/025-lag-in-the-system.test.ts` (276) |
| `crates/cards/src/scripts/classic/c026_rapid_draw.rs` | new | script and tests from `packages/cards/src/scripts/classic/026-rapid-draw.ts` (44) + `packages/cards/test/classic/026-rapid-draw.test.ts` (184) |
| `crates/cards/src/scripts/classic/c027_pestilent_slime.rs` | new | script and tests from `packages/cards/src/scripts/classic/027-pestilent-slime.ts` (28) + `packages/cards/test/classic/027-pestilent-slime.test.ts` (162) |
| `crates/cards/src/scripts/classic/c028_second_wind.rs` | new | script and tests from `packages/cards/src/scripts/classic/028-second-wind.ts` (67) + `packages/cards/test/classic/028-second-wind.test.ts` (293) |
| `crates/cards/src/scripts/classic/c029_book_of_vital_kill.rs` | new | script and tests from `packages/cards/src/scripts/classic/029-book-of-vital-kill.ts` (41) + `packages/cards/test/classic/029-book-of-vital-kill.test.ts` (189) |
| `crates/cards/src/scripts/classic/c030_recycle.rs` | new | script and tests from `packages/cards/src/scripts/classic/030-recycle.ts` (57) + `packages/cards/test/classic/030-recycle.test.ts` (197) |
| `crates/cards/src/scripts/classic/c031_cookie_guild.rs` | new | script and tests from `packages/cards/src/scripts/classic/031-cookie-guild.ts` (35) + `packages/cards/test/classic/031-cookie-guild.test.ts` (209) |
| `crates/cards/src/scripts/classic/c032_felinor_feelings.rs` | new | script and tests from `packages/cards/src/scripts/classic/032-felinor-feelings.ts` (74) + `packages/cards/test/classic/032-felinor-feelings.test.ts` (292) |
| `crates/cards/src/scripts/core/t_rush.rs` | new | script and tests from `packages/cards/src/scripts/t-rush.ts` (46) + `packages/cards/test/t-rush.test.ts` (277) |
| `crates/cards/src/scripts/core/t_sheep.rs` | new | script and tests from `packages/cards/src/scripts/t-sheep.ts` (34) + `packages/cards/test/t-sheep.test.ts` (224) |

Do **not** touch: any `Cargo.toml`, `lib.rs`, `mod.rs` or `main.rs` (part 1's), other parts' files, `packages/` and `apps/server/` (read only; part 37 deletes them).

### 3. Steps

1. Read SURFACE.md §4 (translation), §6.6 (effects and hooks), §7 (card files) and §8 (testkit) once, fully.
2. Work down the table in order. For each card: (a) read the TS script and its TS test; (b) write the Rust file at the destination: the TS header comment as `//!` lines, `use jackioh_engine::prelude::*;`, `pub const ID: &str = "<the id in cardDef(...)>";`, `pub fn script() -> CardScripts { … }` with each TS `Script` field snake_cased (`cry`, `end_of_turn`, `static_flags`, `targets`, …), each hook as `hook(|ctx| vec![ … ])`, each effect via `jackioh_engine::effects::<name>` with its argument built as a struct literal or `json_as(json!({ …the TS object literal… }))`; (c) below it, `#[cfg(test)] mod tests { use jackioh_engine::testkit::*; … }` with one `#[test]` per TS `it(...)`, translated with §8's table, keeping the TS test file's header comment above the module.
3. TS `export const radiant: Script = base;` → build `base` once and use `base.clone()` for `radiant`. TS `param(args, "cap")` → `param(&args, "cap")`. TS `resume: { step: fn }` → `resume: IndexMap::from([("step", hook(step_fn))])`. TS constants at the top of a script stay `const` items in the file.
4. A TS test that builds state with engine internals (`placeOnField`, `newInstance`, `registerScripts`, `makeContext`) uses the same names snake_cased from `jackioh_engine::testkit::*` (it re-exports the whole engine). `registerScripts` is `testkit::register_scripts` (the thread-local override, §8).
5. Commit after every 5 cards on your branch.

### 4. Tests

- Every TS `it` of every card in the table exists as a `#[test]` in that card's file, with the same assertions.
- Nothing is run in Wave 1; part 33 runs `cargo test -p jackioh-cards` in Wave 3.

### 5. Done when

- [ ] Every destination in the table exists, holds `pub const ID`, `pub fn script()` and a `mod tests` with one test per TS `it`.
- [ ] No `todo!`, `unimplemented!`, empty hook or `// TODO` in your files (`grep -nE 'todo!|unimplemented!|TODO' <your files>` is empty).
- [ ] `.fullsend/notes/part-12.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- A card that calls an engine helper the prelude does not re-export: name it by its module path (`jackioh_engine::zones::…`) and list it under GAPS.
- Object literals with a function inside (a filter callback) cannot go through `json!`: build that argument in Rust and set the function field directly.
- Seed-pinned tests (`createRng("…")` with expected picks) must keep their seeds; the Rust RNG is bit-identical (§6.3).

<!-- /jackioh-bot:plan -->
