# v0.3.0 (part 13 of 40): cards lane 5: Classic #33–#68

Part 13 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | nothing (the bot works from `main`) |
| Branch | the night bot's own branch and pull request to `main` (new files only; the orchestrator merges `main` into `staging`) |
| Builder | the night bot, `difficulty:hard` (Opus builds and reviews) |
| Notes | `.fullsend/notes/part-13.md` and `.assumptions` (SURFACE §16) |

Queued for the night bot by #306's orchestrator (`bot:build`, `difficulty:hard`). The Plan below is between the night bot's plan markers: build from it without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port every card in cards lane 5 (the table below, in order): each TypeScript script and its test file become one Rust file holding the script and, at the bottom, its tests (SURFACE.md §7 and §8). The behaviour must match the TypeScript exactly; the golden traces (part 23) and these tests decide it in Wave 3.

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
| `crates/cards/src/scripts/classic/c033_joro.rs` | new | script and tests from `packages/cards/src/scripts/classic/033-joro.ts` (45) + `packages/cards/test/classic/033-joro.test.ts` (316) |
| `crates/cards/src/scripts/classic/c034_ancient_acquisition.rs` | new | script and tests from `packages/cards/src/scripts/classic/034-ancient-acquisition.ts` (30) + `packages/cards/test/classic/034-ancient-acquisition.test.ts` (246) |
| `crates/cards/src/scripts/classic/c035_prep.rs` | new | script and tests from `packages/cards/src/scripts/classic/035-prep.ts` (44) + `packages/cards/test/classic/035-prep.test.ts` (164) |
| `crates/cards/src/scripts/classic/c036_burn.rs` | new | script and tests from `packages/cards/src/scripts/classic/036-burn.ts` (67) + `packages/cards/test/classic/036-burn.test.ts` (137) |
| `crates/cards/src/scripts/classic/c037_last_hurrah.rs` | new | script and tests from `packages/cards/src/scripts/classic/037-last-hurrah.ts` (33) + `packages/cards/test/classic/037-last-hurrah.test.ts` (202) |
| `crates/cards/src/scripts/classic/c038_jackiestan_auctioneer.rs` | new | script and tests from `packages/cards/src/scripts/classic/038-jackiestan-auctioneer.ts` (75) + `packages/cards/test/classic/038-jackiestan-auctioneer.test.ts` (270) |
| `crates/cards/src/scripts/classic/c039_outbreak.rs` | new | script and tests from `packages/cards/src/scripts/classic/039-outbreak.ts` (79) + `packages/cards/test/classic/039-outbreak.test.ts` (253) |
| `crates/cards/src/scripts/classic/c040_mc_tech.rs` | new | script and tests from `packages/cards/src/scripts/classic/040-mc-tech.ts` (89) + `packages/cards/test/classic/040-mc-tech.test.ts` (261) |
| `crates/cards/src/scripts/classic/c041_state_of_the_game.rs` | new | script and tests from `packages/cards/src/scripts/classic/041-state-of-the-game.ts` (27) + `packages/cards/test/classic/041-state-of-the-game.test.ts` (214) |
| `crates/cards/src/scripts/classic/c042_transmutable_toxins.rs` | new | script and tests from `packages/cards/src/scripts/classic/042-transmutable-toxins.ts` (63) + `packages/cards/test/classic/042-transmutable-toxins.test.ts` (191) |
| `crates/cards/src/scripts/classic/c043_plague_nuke.rs` | new | script and tests from `packages/cards/src/scripts/classic/043-plague-nuke.ts` (94) + `packages/cards/test/classic/043-plague-nuke.test.ts` (250) |
| `crates/cards/src/scripts/classic/c044_back_from_the_gy.rs` | new | script and tests from `packages/cards/src/scripts/classic/044-back-from-the-gy.ts` (63) + `packages/cards/test/classic/044-back-from-the-gy.test.ts` (257) |
| `crates/cards/src/scripts/classic/c045_nature_titan.rs` | new | script and tests from `packages/cards/src/scripts/classic/045-nature-titan.ts` (58) + `packages/cards/test/classic/045-nature-titan.test.ts` (226) |
| `crates/cards/src/scripts/classic/c046_divine_favor.rs` | new | script and tests from `packages/cards/src/scripts/classic/046-divine-favor.ts` (48) + `packages/cards/test/classic/046-divine-favor.test.ts` (156) |
| `crates/cards/src/scripts/classic/c047_recurring_felinor.rs` | new | script and tests from `packages/cards/src/scripts/classic/047-recurring-felinor.ts` (39) + `packages/cards/test/classic/047-recurring-felinor.test.ts` (221) |
| `crates/cards/src/scripts/classic/c048_hired_shrimp.rs` | new | script and tests from `packages/cards/src/scripts/classic/048-hired-shrimp.ts` (38) + `packages/cards/test/classic/048-hired-shrimp.test.ts` (283) |
| `crates/cards/src/scripts/classic/c049_anti_greed_machine.rs` | new | script and tests from `packages/cards/src/scripts/classic/049-anti-greed-machine.ts` (33) + `packages/cards/test/classic/049-anti-greed-machine.test.ts` (246) |
| `crates/cards/src/scripts/classic/c050_voidwalker.rs` | new | script and tests from `packages/cards/src/scripts/classic/050-voidwalker.ts` (32) + `packages/cards/test/classic/050-voidwalker.test.ts` (287) |
| `crates/cards/src/scripts/classic/c051_back_breaker.rs` | new | script and tests from `packages/cards/src/scripts/classic/051-back-breaker.ts` (34) + `packages/cards/test/classic/051-back-breaker.test.ts` (162) |
| `crates/cards/src/scripts/classic/c052_final_gambit.rs` | new | script and tests from `packages/cards/src/scripts/classic/052-final-gambit.ts` (43) + `packages/cards/test/classic/052-final-gambit.test.ts` (439) |
| `crates/cards/src/scripts/classic/c053_plague_crawler.rs` | new | script and tests from `packages/cards/src/scripts/classic/053-plague-crawler.ts` (67) + `packages/cards/test/classic/053-plague-crawler.test.ts` (328) |
| `crates/cards/src/scripts/classic/c054_rewind.rs` | new | script and tests from `packages/cards/src/scripts/classic/054-rewind.ts` (28) + `packages/cards/test/classic/054-rewind.test.ts` (257) |
| `crates/cards/src/scripts/classic/c055_book_of_wildfire.rs` | new | script and tests from `packages/cards/src/scripts/classic/055-book-of-wildfire.ts` (42) + `packages/cards/test/classic/055-book-of-wildfire.test.ts` (391) |
| `crates/cards/src/scripts/classic/c056_spell_tyrant.rs` | new | script and tests from `packages/cards/src/scripts/classic/056-spell-tyrant.ts` (41) + `packages/cards/test/classic/056-spell-tyrant.test.ts` (245) |
| `crates/cards/src/scripts/classic/c057_echo.rs` | new | script and tests from `packages/cards/src/scripts/classic/057-echo.ts` (49) + `packages/cards/test/classic/057-echo.test.ts` (298) |
| `crates/cards/src/scripts/classic/c058_common_resources.rs` | new | script and tests from `packages/cards/src/scripts/classic/058-common-resources.ts` (39) + `packages/cards/test/classic/058-common-resources.test.ts` (226) |
| `crates/cards/src/scripts/classic/c059_plague_doctor.rs` | new | script and tests from `packages/cards/src/scripts/classic/059-plague-doctor.ts` (91) + `packages/cards/test/classic/059-plague-doctor.test.ts` (195) |
| `crates/cards/src/scripts/classic/c060_pile_on.rs` | new | script and tests from `packages/cards/src/scripts/classic/060-pile-on.ts` (24) + `packages/cards/test/classic/060-pile-on.test.ts` (239) |
| `crates/cards/src/scripts/classic/c061_plague_bringer_goliath.rs` | new | script and tests from `packages/cards/src/scripts/classic/061-plague-bringer-goliath.ts` (41) + `packages/cards/test/classic/061-plague-bringer-goliath.test.ts` (291) |
| `crates/cards/src/scripts/classic/c062_living_bomb.rs` | new | script and tests from `packages/cards/src/scripts/classic/062-living-bomb.ts` (45) + `packages/cards/test/classic/062-living-bomb.test.ts` (281) |
| `crates/cards/src/scripts/classic/c063_crop_dusting.rs` | new | script and tests from `packages/cards/src/scripts/classic/063-crop-dusting.ts` (51) + `packages/cards/test/classic/063-crop-dusting.test.ts` (266) |
| `crates/cards/src/scripts/classic/c064_malzahars_recycler.rs` | new | script and tests from `packages/cards/src/scripts/classic/064-malzahars-recycler.ts` (66) + `packages/cards/test/classic/064-malzahars-recycler.test.ts` (308) |
| `crates/cards/src/scripts/classic/c065_ace_in_the_hole.rs` | new | script and tests from `packages/cards/src/scripts/classic/065-ace-in-the-hole.ts` (51) + `packages/cards/test/classic/065-ace-in-the-hole.test.ts` (260) |
| `crates/cards/src/scripts/classic/c066_eu_striker.rs` | new | script and tests from `packages/cards/src/scripts/classic/066-eu-striker.ts` (73) + `packages/cards/test/classic/066-eu-striker.test.ts` (217) |
| `crates/cards/src/scripts/classic/c067_felinor_feeler.rs` | new | script and tests from `packages/cards/src/scripts/classic/067-felinor-feeler.ts` (35) + `packages/cards/test/classic/067-felinor-feeler.test.ts` (102) |
| `crates/cards/src/scripts/classic/c068_small_card_lobbyist.rs` | new | script and tests from `packages/cards/src/scripts/classic/068-small-card-lobbyist.ts` (37) + `packages/cards/test/classic/068-small-card-lobbyist.test.ts` (240) |

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
- [ ] `.fullsend/notes/part-13.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- A card that calls an engine helper the prelude does not re-export: name it by its module path (`jackioh_engine::zones::…`) and list it under GAPS.
- Object literals with a function inside (a filter callback) cannot go through `json!`: build that argument in Rust and set the function field directly.
- Seed-pinned tests (`createRng("…")` with expected picks) must keep their seeds; the Rust RNG is bit-identical (§6.3).

<!-- /jackioh-bot:plan -->
