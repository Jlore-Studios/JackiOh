# v0.3.0 (part 9 of 40): cards lane 1: Core #1–#48

Part 9 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | nothing (the bot works from `main`) |
| Branch | the night bot's own branch and pull request to `main` (new files only; the orchestrator merges `main` into `staging`) |
| Builder | the night bot, `difficulty:hard` (Opus builds and reviews) |
| Notes | `.fullsend/notes/part-09.md` and `.assumptions` (SURFACE §16) |

Queued for the night bot by #306's orchestrator (`bot:build`, `difficulty:hard`). The Plan below is between the night bot's plan markers: build from it without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port every card in cards lane 1 (the table below, in order): each TypeScript script and its test file become one Rust file holding the script and, at the bottom, its tests (SURFACE.md §7 and §8). The behaviour must match the TypeScript exactly; the golden traces (part 23) and these tests decide it in Wave 3.

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
| `crates/cards/src/query.rs` | new | port of `packages/cards/src/query.ts` (110 lines). used by 083 Transmogulate and 067 Zoomerbin Oomen |
| `crates/cards/src/scripts/core/c001_big_d_fender.rs` | new | script and tests from `packages/cards/src/scripts/001-big-d-fender.ts` (36) + `packages/cards/test/001-big-d-fender.test.ts` (65) |
| `crates/cards/src/scripts/core/c002_bigot.rs` | new | script and tests from `packages/cards/src/scripts/002-bigot.ts` (30) + `packages/cards/test/002-bigot.test.ts` (62) |
| `crates/cards/src/scripts/core/c003_right_house_defender.rs` | new | script and tests from `packages/cards/src/scripts/003-right-house-defender.ts` (27) + `packages/cards/test/003-right-house-defender.test.ts` (146) |
| `crates/cards/src/scripts/core/c004_gary_the_gambler.rs` | new | script and tests from `packages/cards/src/scripts/004-gary-the-gambler.ts` (39) + `packages/cards/test/004-gary-the-gambler.test.ts` (130) |
| `crates/cards/src/scripts/core/c005_stockpile.rs` | new | script and tests from `packages/cards/src/scripts/005-stockpile.ts` (24) + `packages/cards/test/005-stockpile.test.ts` (74) |
| `crates/cards/src/scripts/core/c006_mana_well.rs` | new | script and tests from `packages/cards/src/scripts/006-mana-well.ts` (34) + `packages/cards/test/006-mana-well.test.ts` (95) |
| `crates/cards/src/scripts/core/c007_jewelosco_scarab.rs` | new | script and tests from `packages/cards/src/scripts/007-jewelosco-scarab.ts` (65) + `packages/cards/test/007-jewelosco-scarab.test.ts` (147) |
| `crates/cards/src/scripts/core/c008_mr_vanilla.rs` | new | script and tests from `packages/cards/src/scripts/008-mr-vanilla.ts` (16) + `packages/cards/test/008-mr-vanilla.test.ts` (103) |
| `crates/cards/src/scripts/core/c009_moths_to_the_flame.rs` | new | script and tests from `packages/cards/src/scripts/009-moths-to-the-flame.ts` (37) + `packages/cards/test/009-moths-to-the-flame.test.ts` (125) |
| `crates/cards/src/scripts/core/c010_rapid_replenish.rs` | new | script and tests from `packages/cards/src/scripts/010-rapid-replenish.ts` (54) + `packages/cards/test/010-rapid-replenish.test.ts` (153) |
| `crates/cards/src/scripts/core/c011_tempo_timmy.rs` | new | script and tests from `packages/cards/src/scripts/011-tempo-timmy.ts` (18) + `packages/cards/test/011-tempo-timmy.test.ts` (103) |
| `crates/cards/src/scripts/core/c012_duplicating_felinors.rs` | new | script and tests from `packages/cards/src/scripts/012-duplicating-felinors.ts` (40) + `packages/cards/test/012-duplicating-felinors.test.ts` (153) |
| `crates/cards/src/scripts/core/c013_jlockeed_shredder_10.rs` | new | script and tests from `packages/cards/src/scripts/013-jlockeed-shredder-10.ts` (40) + `packages/cards/test/013-jlockeed-shredder-10.test.ts` (108) |
| `crates/cards/src/scripts/core/c014_jlockeeds_weapons.rs` | new | script and tests from `packages/cards/src/scripts/014-jlockeeds-weapons.ts` (39) + `packages/cards/test/014-jlockeeds-weapons.test.ts` (143) |
| `crates/cards/src/scripts/core/c015_me_and_mr_token.rs` | new | script and tests from `packages/cards/src/scripts/015-me-and-mr-token.ts` (33) + `packages/cards/test/015-me-and-mr-token.test.ts` (89) |
| `crates/cards/src/scripts/core/c016_hit_job.rs` | new | script and tests from `packages/cards/src/scripts/016-hit-job.ts` (45) + `packages/cards/test/016-hit-job.test.ts` (183) |
| `crates/cards/src/scripts/core/c017_flood.rs` | new | script and tests from `packages/cards/src/scripts/017-flood.ts` (70) + `packages/cards/test/017-flood.test.ts` (206) |
| `crates/cards/src/scripts/core/c018_bread_and_butter.rs` | new | script and tests from `packages/cards/src/scripts/018-bread-and-butter.ts` (126) + `packages/cards/test/018-bread-and-butter.test.ts` (205) |
| `crates/cards/src/scripts/core/c019_midrange_menace.rs` | new | script and tests from `packages/cards/src/scripts/019-midrange-menace.ts` (37) + `packages/cards/test/019-midrange-menace.test.ts` (131) |
| `crates/cards/src/scripts/core/c020_pointmaster.rs` | new | script and tests from `packages/cards/src/scripts/020-pointmaster.ts` (22) + `packages/cards/test/020-pointmaster.test.ts` (105) |
| `crates/cards/src/scripts/core/c021_hinder.rs` | new | script and tests from `packages/cards/src/scripts/021-hinder.ts` (42) + `packages/cards/test/021-hinder.test.ts` (256) |
| `crates/cards/src/scripts/core/c022_carnivorous_cube.rs` | new | script and tests from `packages/cards/src/scripts/022-carnivorous-cube.ts` (159) + `packages/cards/test/022-carnivorous-cube.test.ts` (246) |
| `crates/cards/src/scripts/core/c023_reoccurring_dream.rs` | new | script and tests from `packages/cards/src/scripts/023-reoccurring-dream.ts` (81) + `packages/cards/test/023-reoccurring-dream.test.ts` (160) |
| `crates/cards/src/scripts/core/c024_efficiency_dividend.rs` | new | script and tests from `packages/cards/src/scripts/024-efficiency-dividend.ts` (106) + `packages/cards/test/024-efficiency-dividend.test.ts` (233) |
| `crates/cards/src/scripts/core/c025_4_mana_7_7.rs` | new | script and tests from `packages/cards/src/scripts/025-4-mana-7-7.ts` (22) + `packages/cards/test/025-4-mana-7-7.test.ts` (138) |
| `crates/cards/src/scripts/core/c026_glowy_jelly_bean.rs` | new | script and tests from `packages/cards/src/scripts/026-glowy-jelly-bean.ts` (40) + `packages/cards/test/026-glowy-jelly-bean.test.ts` (111) |
| `crates/cards/src/scripts/core/c027_blood_ridden_glowy_jelly_bean.rs` | new | script and tests from `packages/cards/src/scripts/027-blood-ridden-glowy-jelly-bean.ts` (44) + `packages/cards/test/027-blood-ridden-glowy-jelly-bean.test.ts` (123) |
| `crates/cards/src/scripts/core/c028_knockoff_temu_glowy_jelly_bean.rs` | new | script and tests from `packages/cards/src/scripts/028-knockoff-temu-glowy-jelly-bean.ts` (44) + `packages/cards/test/028-knockoff-temu-glowy-jelly-bean.test.ts` (152) |
| `crates/cards/src/scripts/core/c029_giga_glowy_jelly_bean.rs` | new | script and tests from `packages/cards/src/scripts/029-giga-glowy-jelly-bean.ts` (60) + `packages/cards/test/029-giga-glowy-jelly-bean.test.ts` (129) |
| `crates/cards/src/scripts/core/c030_archivist.rs` | new | script and tests from `packages/cards/src/scripts/030-archivist.ts` (97) + `packages/cards/test/030-archivist.test.ts` (140) |
| `crates/cards/src/scripts/core/c031_kys_math_equation.rs` | new | script and tests from `packages/cards/src/scripts/031-kys-math-equation.ts` (134) + `packages/cards/test/031-kys-math-equation.test.ts` (229) |
| `crates/cards/src/scripts/core/c032_prem_panther.rs` | new | script and tests from `packages/cards/src/scripts/032-prem-panther.ts` (32) + `packages/cards/test/032-prem-panther.test.ts` (315) |
| `crates/cards/src/scripts/core/c033_unstable_clone_machine.rs` | new | script and tests from `packages/cards/src/scripts/033-unstable-clone-machine.ts` (88) + `packages/cards/test/033-unstable-clone-machine.test.ts` (313) |
| `crates/cards/src/scripts/core/c034_collateral_damage.rs` | new | script and tests from `packages/cards/src/scripts/034-collateral-damage.ts` (67) + `packages/cards/test/034-collateral-damage.test.ts` (175) |
| `crates/cards/src/scripts/core/c035_lunar_eclipse.rs` | new | script and tests from `packages/cards/src/scripts/035-lunar-eclipse.ts` (81) + `packages/cards/test/035-lunar-eclipse.test.ts` (152) |
| `crates/cards/src/scripts/core/c036_magic_jammed.rs` | new | script and tests from `packages/cards/src/scripts/036-magic-jammed.ts` (50) + `packages/cards/test/036-magic-jammed.test.ts` (135) |
| `crates/cards/src/scripts/core/c037_gravedigger.rs` | new | script and tests from `packages/cards/src/scripts/037-gravedigger.ts` (71) + `packages/cards/test/037-gravedigger.test.ts` (180) |
| `crates/cards/src/scripts/core/c038_quickstriker.rs` | new | script and tests from `packages/cards/src/scripts/038-quickstriker.ts` (57) + `packages/cards/test/038-quickstriker.test.ts` (292) |
| `crates/cards/src/scripts/core/c039_recycling_initiative.rs` | new | script and tests from `packages/cards/src/scripts/039-recycling-initiative.ts` (141) + `packages/cards/test/039-recycling-initiative.test.ts` (330) |
| `crates/cards/src/scripts/core/c040_echoes_of_the_forgotten.rs` | new | script and tests from `packages/cards/src/scripts/040-echoes-of-the-forgotten.ts` (72) + `packages/cards/test/040-echoes-of-the-forgotten.test.ts` (243) |
| `crates/cards/src/scripts/core/c041_sheepish.rs` | new | script and tests from `packages/cards/src/scripts/041-sheepish.ts` (90) + `packages/cards/test/041-sheepish.test.ts` (263) |
| `crates/cards/src/scripts/core/c042_eugenics.rs` | new | script and tests from `packages/cards/src/scripts/042-eugenics.ts` (55) + `packages/cards/test/042-eugenics.test.ts` (169) |
| `crates/cards/src/scripts/core/c043_big_felinor.rs` | new | script and tests from `packages/cards/src/scripts/043-big-felinor.ts` (47) + `packages/cards/test/043-big-felinor.test.ts` (134) |
| `crates/cards/src/scripts/core/c044_true_strike.rs` | new | script and tests from `packages/cards/src/scripts/044-true-strike.ts` (51) + `packages/cards/test/044-true-strike.test.ts` (162) |
| `crates/cards/src/scripts/core/c045_deft_duelist.rs` | new | script and tests from `packages/cards/src/scripts/045-deft-duelist.ts` (21) + `packages/cards/test/045-deft-duelist.test.ts` (160) |
| `crates/cards/src/scripts/core/c046_suppressive_aura.rs` | new | script and tests from `packages/cards/src/scripts/046-suppressive-aura.ts` (66) + `packages/cards/test/046-suppressive-aura.test.ts` (210) |
| `crates/cards/src/scripts/core/c047_fig_of_life.rs` | new | script and tests from `packages/cards/src/scripts/047-fig-of-life.ts` (37) + `packages/cards/test/047-fig-of-life.test.ts` (128) |
| `crates/cards/src/scripts/core/c048_5pek_controller.rs` | new | script and tests from `packages/cards/src/scripts/048-5pek-controller.ts` (51) + `packages/cards/test/048-5pek-controller.test.ts` (157) |

Do **not** touch: any `Cargo.toml`, `lib.rs`, `mod.rs` or `main.rs` (part 1's), other parts' files, `packages/` and `apps/server/` (read only; part 37 deletes them).

### 3. Steps

1. First, port `packages/cards/src/query.ts` to `crates/cards/src/query.rs` (the typed wrapper `catalog.query`, `pool`, `cost`, `TRAP_TYPES`, … over `jackioh_engine::catalog::query`/`query_cost`); cards 067 and 083 (lane 2) import it as `crate::query::*`.
2. Read SURFACE.md §4 (translation), §6.6 (effects and hooks), §7 (card files) and §8 (testkit) once, fully.
3. Work down the table in order. For each card: (a) read the TS script and its TS test; (b) write the Rust file at the destination: the TS header comment as `//!` lines, `use jackioh_engine::prelude::*;`, `pub const ID: &str = "<the id in cardDef(...)>";`, `pub fn script() -> CardScripts { … }` with each TS `Script` field snake_cased (`cry`, `end_of_turn`, `static_flags`, `targets`, …), each hook as `hook(|ctx| vec![ … ])`, each effect via `jackioh_engine::effects::<name>` with its argument built as a struct literal or `json_as(json!({ …the TS object literal… }))`; (c) below it, `#[cfg(test)] mod tests { use jackioh_engine::testkit::*; … }` with one `#[test]` per TS `it(...)`, translated with §8's table, keeping the TS test file's header comment above the module.
4. TS `export const radiant: Script = base;` → build `base` once and use `base.clone()` for `radiant`. TS `param(args, "cap")` → `param(&args, "cap")`. TS `resume: { step: fn }` → `resume: IndexMap::from([("step", hook(step_fn))])`. TS constants at the top of a script stay `const` items in the file.
5. A TS test that builds state with engine internals (`placeOnField`, `newInstance`, `registerScripts`, `makeContext`) uses the same names snake_cased from `jackioh_engine::testkit::*` (it re-exports the whole engine). `registerScripts` is `testkit::register_scripts` (the thread-local override, §8).
6. Commit after every 5 cards on your branch.

### 4. Tests

- Every TS `it` of every card in the table exists as a `#[test]` in that card's file, with the same assertions.
- Nothing is run in Wave 1; part 33 runs `cargo test -p jackioh-cards` in Wave 3.

### 5. Done when

- [ ] Every destination in the table exists, holds `pub const ID`, `pub fn script()` and a `mod tests` with one test per TS `it`.
- [ ] No `todo!`, `unimplemented!`, empty hook or `// TODO` in your files (`grep -nE 'todo!|unimplemented!|TODO' <your files>` is empty).
- [ ] `.fullsend/notes/part-09.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- A card that calls an engine helper the prelude does not re-export: name it by its module path (`jackioh_engine::zones::…`) and list it under GAPS.
- Object literals with a function inside (a filter callback) cannot go through `json!`: build that argument in Rust and set the function field directly.
- Seed-pinned tests (`createRng("…")` with expected picks) must keep their seeds; the Rust RNG is bit-identical (§6.3).

<!-- /jackioh-bot:plan -->
