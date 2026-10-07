# v0.3.0 (part 15 of 40): cards lane 7: Classic+ #19–#49

Part 15 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | nothing (the bot works from `main`) |
| Branch | the night bot's own branch and pull request to `main` (new files only; the orchestrator merges `main` into `staging`) |
| Builder | the night bot, `difficulty:hard` (Opus builds and reviews) |
| Notes | `.fullsend/notes/part-15.md` and `.assumptions` (SURFACE §16) |

Queued for the night bot by #306's orchestrator (`bot:build`, `difficulty:hard`). The Plan below is between the night bot's plan markers: build from it without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port every card in cards lane 7 (the table below, in order): each TypeScript script and its test file become one Rust file holding the script and, at the bottom, its tests (SURFACE.md §7 and §8). The behaviour must match the TypeScript exactly; the golden traces (part 23) and these tests decide it in Wave 3.

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
| `crates/cards/src/ky_test_bank.rs` | new | port of `packages/cards/src/kyTestBank.ts` (108 lines). with C+ #42 KY's Test |
| `crates/cards/src/scripts/classic_plus/c019_1_top_loser.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/019-1-top-loser.ts` (14) + `packages/cards/test/classic-plus/019-1-top-loser.test.ts` (219) |
| `crates/cards/src/scripts/classic_plus/c019_2_jungle_loser.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/019-2-jungle-loser.ts` (56) + `packages/cards/test/classic-plus/019-2-jungle-loser.test.ts` (221) |
| `crates/cards/src/scripts/classic_plus/c019_3_mid_loser.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/019-3-mid-loser.ts` (33) + `packages/cards/test/classic-plus/019-3-mid-loser.test.ts` (191) |
| `crates/cards/src/scripts/classic_plus/c019_4_support_loser.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/019-4-support-loser.ts` (24) + `packages/cards/test/classic-plus/019-4-support-loser.test.ts` (146) |
| `crates/cards/src/scripts/classic_plus/c019_5_bot_loser.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/019-5-bot-loser.ts` (38) + `packages/cards/test/classic-plus/019-5-bot-loser.test.ts` (206) |
| `crates/cards/src/scripts/classic_plus/c019_league_of_losers.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/019-league-of-losers.ts` (49) + `packages/cards/test/classic-plus/019-league-of-losers.test.ts` (178) |
| `crates/cards/src/scripts/classic_plus/c020_mushroom_power.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/020-mushroom-power.ts` (22) + `packages/cards/test/classic-plus/020-mushroom-power.test.ts` (122) |
| `crates/cards/src/scripts/classic_plus/c021_whirlwind.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/021-whirlwind.ts` (17) + `packages/cards/test/classic-plus/021-whirlwind.test.ts` (176) |
| `crates/cards/src/scripts/classic_plus/c022_blood_moon.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/022-blood-moon.ts` (16) + `packages/cards/test/classic-plus/022-blood-moon.test.ts` (251) |
| `crates/cards/src/scripts/classic_plus/c023_dropshipping.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/023-dropshipping.ts` (38) + `packages/cards/test/classic-plus/023-dropshipping.test.ts` (221) |
| `crates/cards/src/scripts/classic_plus/c024_crushing_walls.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/024-crushing-walls.ts` (28) + `packages/cards/test/classic-plus/024-crushing-walls.test.ts` (163) |
| `crates/cards/src/scripts/classic_plus/c025_soul_shot.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/025-soul-shot.ts` (52) + `packages/cards/test/classic-plus/025-soul-shot.test.ts` (167) |
| `crates/cards/src/scripts/classic_plus/c026_tommy_tempo.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/026-tommy-tempo.ts` (22) + `packages/cards/test/classic-plus/026-tommy-tempo.test.ts` (248) |
| `crates/cards/src/scripts/classic_plus/c027_zephrys_zealotism.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/027-zephrys-zealotism.ts` (38) + `packages/cards/test/classic-plus/027-zephrys-zealotism.test.ts` (194) |
| `crates/cards/src/scripts/classic_plus/c028_nuestro_hogar_nuestras_tumbas.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/028-nuestro-hogar-nuestras-tumbas.ts` (24) + `packages/cards/test/classic-plus/028-nuestro-hogar-nuestras-tumbas.test.ts` (138) |
| `crates/cards/src/scripts/classic_plus/c029_portal_to_the_past.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/029-portal-to-the-past.ts` (23) + `packages/cards/test/classic-plus/029-portal-to-the-past.test.ts` (282) |
| `crates/cards/src/scripts/classic_plus/c030_felinor_fuser.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/030-felinor-fuser.ts` (71) + `packages/cards/test/classic-plus/030-felinor-fuser.test.ts` (186) |
| `crates/cards/src/scripts/classic_plus/c031_fusion_lab.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/031-fusion-lab.ts` (54) + `packages/cards/test/classic-plus/031-fusion-lab.test.ts` (206) |
| `crates/cards/src/scripts/classic_plus/c032_1_execute.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/032-1-execute.ts` (38) + `packages/cards/test/classic-plus/032-1-execute.test.ts` (101) |
| `crates/cards/src/scripts/classic_plus/c032_2_brawl.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/032-2-brawl.ts` (56) + `packages/cards/test/classic-plus/032-2-brawl.test.ts` (153) |
| `crates/cards/src/scripts/classic_plus/c032_3_blade_storm.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/032-3-blade-storm.ts` (34) + `packages/cards/test/classic-plus/032-3-blade-storm.test.ts` (179) |
| `crates/cards/src/scripts/classic_plus/c032_otherworldly_removal.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/032-otherworldly-removal.ts` (27) + `packages/cards/test/classic-plus/032-otherworldly-removal.test.ts` (77) |
| `crates/cards/src/scripts/classic_plus/c033_ivory_tower.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/033-ivory-tower.ts` (66) + `packages/cards/test/classic-plus/033-ivory-tower.test.ts` (186) |
| `crates/cards/src/scripts/classic_plus/c034_memory_leak.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/034-memory-leak.ts` (52) + `packages/cards/test/classic-plus/034-memory-leak.test.ts` (209) |
| `crates/cards/src/scripts/classic_plus/c035_rollback.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/035-rollback.ts` (29) + `packages/cards/test/classic-plus/035-rollback.test.ts` (715) |
| `crates/cards/src/scripts/classic_plus/c036_1_bone_storm.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/036-1-bone-storm.ts` (19) + `packages/cards/test/classic-plus/036-1-bone-storm.test.ts` (133) |
| `crates/cards/src/scripts/classic_plus/c036_conjure_bones.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/036-conjure-bones.ts` (30) + `packages/cards/test/classic-plus/036-conjure-bones.test.ts` (125) |
| `crates/cards/src/scripts/classic_plus/c037_wardrum.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/037-wardrum.ts` (98) + `packages/cards/test/classic-plus/037-wardrum.test.ts` (388) |
| `crates/cards/src/scripts/classic_plus/c038_1_solarius_prime.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/038-1-solarius-prime.ts` (26) + `packages/cards/test/classic-plus/038-1-solarius-prime.test.ts` (254) |
| `crates/cards/src/scripts/classic_plus/c038_solarius.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/038-solarius.ts` (23) + `packages/cards/test/classic-plus/038-solarius.test.ts` (220) |
| `crates/cards/src/scripts/classic_plus/c039_book_worm.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/039-book-worm.ts` (41) + `packages/cards/test/classic-plus/039-book-worm.test.ts` (189) |
| `crates/cards/src/scripts/classic_plus/c040_appropriations.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/040-appropriations.ts` (65) + `packages/cards/test/classic-plus/040-appropriations.test.ts` (284) |
| `crates/cards/src/scripts/classic_plus/c041_kys_constant.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/041-kys-constant.ts` (46) + `packages/cards/test/classic-plus/041-kys-constant.test.ts` (200) |
| `crates/cards/src/scripts/classic_plus/c042_1_kys_gift.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/042-1-kys-gift.ts` (32) + `packages/cards/test/classic-plus/042-1-kys-gift.test.ts` (192) |
| `crates/cards/src/scripts/classic_plus/c042_kys_test.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/042-kys-test.ts` (18) + `packages/cards/test/classic-plus/042-kys-test.test.ts` (363) |
| `crates/cards/src/scripts/classic_plus/c043_ai_slop.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/043-ai-slop.ts` (29) + `packages/cards/test/classic-plus/043-ai-slop.test.ts` (152) |
| `crates/cards/src/scripts/classic_plus/c044_simplicity_audit.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/044-simplicity-audit.ts` (50) + `packages/cards/test/classic-plus/044-simplicity-audit.test.ts` (180) |
| `crates/cards/src/scripts/classic_plus/c045_complexity_audit.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/045-complexity-audit.ts` (50) + `packages/cards/test/classic-plus/045-complexity-audit.test.ts` (144) |
| `crates/cards/src/scripts/classic_plus/c046_1_felinor_flagbearer_prime.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/046-1-felinor-flagbearer-prime.ts` (40) + `packages/cards/test/classic-plus/046-1-felinor-flagbearer-prime.test.ts` (145) |
| `crates/cards/src/scripts/classic_plus/c046_felinor_flagbearer.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/046-felinor-flagbearer.ts` (49) + `packages/cards/test/classic-plus/046-felinor-flagbearer.test.ts` (177) |
| `crates/cards/src/scripts/classic_plus/c047_joggs_box.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/047-joggs-box.ts` (27) + `packages/cards/test/classic-plus/047-joggs-box.test.ts` (366) |
| `crates/cards/src/scripts/classic_plus/c048_jlockheeds_lobbyist.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/048-jlockheeds-lobbyist.ts` (27) + `packages/cards/test/classic-plus/048-jlockheeds-lobbyist.test.ts` (147) |
| `crates/cards/src/scripts/classic_plus/c049_jay_fungus.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/049-jay-fungus.ts` (22) + `packages/cards/test/classic-plus/049-jay-fungus.test.ts` (147) |

Do **not** touch: any `Cargo.toml`, `lib.rs`, `mod.rs` or `main.rs` (part 1's), other parts' files, `packages/` and `apps/server/` (read only; part 37 deletes them).

### 3. Steps

1. Port `packages/cards/src/kyTestBank.ts` to `crates/cards/src/ky_test_bank.rs` before C+ #42 KY's Test, which uses it.
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
- [ ] `.fullsend/notes/part-15.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- A card that calls an engine helper the prelude does not re-export: name it by its module path (`jackioh_engine::zones::…`) and list it under GAPS.
- Object literals with a function inside (a filter callback) cannot go through `json!`: build that argument in Rust and set the function field directly.
- Seed-pinned tests (`createRng("…")` with expected picks) must keep their seeds; the Rust RNG is bit-identical (§6.3).

<!-- /jackioh-bot:plan -->
