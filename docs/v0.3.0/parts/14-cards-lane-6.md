# v0.3.0 (part 14 of 40): cards lane 6: Classic #69–#90 and tokens, Classic+ #1–#18

Part 14 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | nothing (the bot works from `main`) |
| Branch | the night bot's own branch and pull request to `main` (new files only; the orchestrator merges `main` into `staging`) |
| Builder | the night bot, `difficulty:hard` (Opus builds and reviews) |
| Notes | `.fullsend/notes/part-14.md` and `.assumptions` (SURFACE §16) |

Queued for the night bot by #306's orchestrator (`bot:build`, `difficulty:hard`). The Plan below is between the night bot's plan markers: build from it without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port every card in cards lane 6 (the table below, in order): each TypeScript script and its test file become one Rust file holding the script and, at the bottom, its tests (SURFACE.md §7 and §8). The behaviour must match the TypeScript exactly; the golden traces (part 23) and these tests decide it in Wave 3.

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
| `crates/cards/src/scripts/classic_plus/c001_doom_shroom.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/001-doom-shroom.ts` (27) + `packages/cards/test/classic-plus/001-doom-shroom.test.ts` (160) |
| `crates/cards/src/scripts/classic_plus/c002_groom_shroom.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/002-groom-shroom.ts` (49) + `packages/cards/test/classic-plus/002-groom-shroom.test.ts` (169) |
| `crates/cards/src/scripts/classic_plus/c003_second_amendment_snake.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/003-second-amendment-snake.ts` (30) + `packages/cards/test/classic-plus/003-second-amendment-snake.test.ts` (221) |
| `crates/cards/src/scripts/classic_plus/c004_juhan_biggest_bat.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/004-juhan-biggest-bat.ts` (15) + `packages/cards/test/classic-plus/004-juhan-biggest-bat.test.ts` (140) |
| `crates/cards/src/scripts/classic_plus/c005_guy_att.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/005-guy-att.ts` (15) + `packages/cards/test/classic-plus/005-guy-att.test.ts` (190) |
| `crates/cards/src/scripts/classic_plus/c006_wrong_house_attacker.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/006-wrong-house-attacker.ts` (13) + `packages/cards/test/classic-plus/006-wrong-house-attacker.test.ts` (91) |
| `crates/cards/src/scripts/classic_plus/c007_the_house.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/007-the-house.ts` (27) + `packages/cards/test/classic-plus/007-the-house.test.ts` (160) |
| `crates/cards/src/scripts/classic_plus/c008_withering_storm.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/008-withering-storm.ts` (37) + `packages/cards/test/classic-plus/008-withering-storm.test.ts` (206) |
| `crates/cards/src/scripts/classic_plus/c009_silence.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/009-silence.ts` (18) + `packages/cards/test/classic-plus/009-silence.test.ts` (184) |
| `crates/cards/src/scripts/classic_plus/c010_new_wraps.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/010-new-wraps.ts` (21) + `packages/cards/test/classic-plus/010-new-wraps.test.ts` (167) |
| `crates/cards/src/scripts/classic_plus/c011_anime_armor.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/011-anime-armor.ts` (14) + `packages/cards/test/classic-plus/011-anime-armor.test.ts` (211) |
| `crates/cards/src/scripts/classic_plus/c012_1_devour.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/012-1-devour.ts` (35) + `packages/cards/test/classic-plus/012-1-devour.test.ts` (104) |
| `crates/cards/src/scripts/classic_plus/c012_2_death_boil.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/012-2-death-boil.ts` (34) + `packages/cards/test/classic-plus/012-2-death-boil.test.ts` (75) |
| `crates/cards/src/scripts/classic_plus/c012_3_fluffy_grip.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/012-3-fluffy-grip.ts` (19) + `packages/cards/test/classic-plus/012-3-fluffy-grip.test.ts` (110) |
| `crates/cards/src/scripts/classic_plus/c012_4_powder_spray.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/012-4-powder-spray.ts` (17) + `packages/cards/test/classic-plus/012-4-powder-spray.test.ts` (71) |
| `crates/cards/src/scripts/classic_plus/c012_5_anti_waffle_shell.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/012-5-anti-waffle-shell.ts` (28) + `packages/cards/test/classic-plus/012-5-anti-waffle-shell.test.ts` (71) |
| `crates/cards/src/scripts/classic_plus/c012_6_frozen_wastes.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/012-6-frozen-wastes.ts` (46) + `packages/cards/test/classic-plus/012-6-frozen-wastes.test.ts` (126) |
| `crates/cards/src/scripts/classic_plus/c012_7_legion_of_the_hungry.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/012-7-legion-of-the-hungry.ts` (32) + `packages/cards/test/classic-plus/012-7-legion-of-the-hungry.test.ts` (105) |
| `crates/cards/src/scripts/classic_plus/c012_8_frostspatula.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/012-8-frostspatula.ts` (60) + `packages/cards/test/classic-plus/012-8-frostspatula.test.ts` (228) |
| `crates/cards/src/scripts/classic_plus/c012_the_mother_pancake.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/012-the-mother-pancake.ts` (20) + `packages/cards/test/classic-plus/012-the-mother-pancake.test.ts` (96) |
| `crates/cards/src/scripts/classic_plus/c013_mommy_barker.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/013-mommy-barker.ts` (18) + `packages/cards/test/classic-plus/013-mommy-barker.test.ts` (89) |
| `crates/cards/src/scripts/classic_plus/c014_forever.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/014-forever.ts` (24) + `packages/cards/test/classic-plus/014-forever.test.ts` (176) |
| `crates/cards/src/scripts/classic_plus/c015_conjure_rush_token.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/015-conjure-rush-token.ts` (25) + `packages/cards/test/classic-plus/015-conjure-rush-token.test.ts` (169) |
| `crates/cards/src/scripts/classic_plus/c016_conjure_rush_token.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/016-conjure-rush-token.ts` (25) + `packages/cards/test/classic-plus/016-conjure-rush-token.test.ts` (102) |
| `crates/cards/src/scripts/classic_plus/c017_conjure_rush_token.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/017-conjure-rush-token.ts` (25) + `packages/cards/test/classic-plus/017-conjure-rush-token.test.ts` (110) |
| `crates/cards/src/scripts/classic_plus/c018_gullible_treatler.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/018-gullible-treatler.ts` (37) + `packages/cards/test/classic-plus/018-gullible-treatler.test.ts` (200) |
| `crates/cards/src/scripts/classic/c069_plague_charger.rs` | new | script and tests from `packages/cards/src/scripts/classic/069-plague-charger.ts` (52) + `packages/cards/test/classic/069-plague-charger.test.ts` (173) |
| `crates/cards/src/scripts/classic/c070_book_of_plague.rs` | new | script and tests from `packages/cards/src/scripts/classic/070-book-of-plague.ts` (21) + `packages/cards/test/classic/070-book-of-plague.test.ts` (237) |
| `crates/cards/src/scripts/classic/c071_lane_eater.rs` | new | script and tests from `packages/cards/src/scripts/classic/071-lane-eater.ts` (54) + `packages/cards/test/classic/071-lane-eater.test.ts` (175) |
| `crates/cards/src/scripts/classic/c072_grand_counterspell.rs` | new | script and tests from `packages/cards/src/scripts/classic/072-grand-counterspell.ts` (54) + `packages/cards/test/classic/072-grand-counterspell.test.ts` (222) |
| `crates/cards/src/scripts/classic/c073_nurse_cleaver.rs` | new | script and tests from `packages/cards/src/scripts/classic/073-nurse-cleaver.ts` (22) + `packages/cards/test/classic/073-nurse-cleaver.test.ts` (63) |
| `crates/cards/src/scripts/classic/c074_corpse_plantation.rs` | new | script and tests from `packages/cards/src/scripts/classic/074-corpse-plantation.ts` (36) + `packages/cards/test/classic/074-corpse-plantation.test.ts` (269) |
| `crates/cards/src/scripts/classic/c075_argusland.rs` | new | script and tests from `packages/cards/src/scripts/classic/075-argusland.ts` (20) + `packages/cards/test/classic/075-argusland.test.ts` (271) |
| `crates/cards/src/scripts/classic/c076_plague_bringer.rs` | new | script and tests from `packages/cards/src/scripts/classic/076-plague-bringer.ts` (24) + `packages/cards/test/classic/076-plague-bringer.test.ts` (217) |
| `crates/cards/src/scripts/classic/c077_anti_magic_monkey.rs` | new | script and tests from `packages/cards/src/scripts/classic/077-anti-magic-monkey.ts` (27) + `packages/cards/test/classic/077-anti-magic-monkey.test.ts` (179) |
| `crates/cards/src/scripts/classic/c078_mutate_spell.rs` | new | script and tests from `packages/cards/src/scripts/classic/078-mutate-spell.ts` (50) + `packages/cards/test/classic/078-mutate-spell.test.ts` (430) |
| `crates/cards/src/scripts/classic/c079_risky_die.rs` | new | script and tests from `packages/cards/src/scripts/classic/079-risky-die.ts` (60) + `packages/cards/test/classic/079-risky-die.test.ts` (179) |
| `crates/cards/src/scripts/classic/c080_boom_big_max.rs` | new | script and tests from `packages/cards/src/scripts/classic/080-boom-big-max.ts` (30) + `packages/cards/test/classic/080-boom-big-max.test.ts` (107) |
| `crates/cards/src/scripts/classic/c081_the_power_to_thrive.rs` | new | script and tests from `packages/cards/src/scripts/classic/081-the-power-to-thrive.ts` (36) + `packages/cards/test/classic/081-the-power-to-thrive.test.ts` (175) |
| `crates/cards/src/scripts/classic/c082_sheeople.rs` | new | script and tests from `packages/cards/src/scripts/classic/082-sheeople.ts` (25) + `packages/cards/test/classic/082-sheeople.test.ts` (221) |
| `crates/cards/src/scripts/classic/c083_flame_lance.rs` | new | script and tests from `packages/cards/src/scripts/classic/083-flame-lance.ts` (33) + `packages/cards/test/classic/083-flame-lance.test.ts` (121) |
| `crates/cards/src/scripts/classic/c084_lockdown.rs` | new | script and tests from `packages/cards/src/scripts/classic/084-lockdown.ts` (55) + `packages/cards/test/classic/084-lockdown.test.ts` (184) |
| `crates/cards/src/scripts/classic/c085_king_wagtoggle.rs` | new | script and tests from `packages/cards/src/scripts/classic/085-king-wagtoggle.ts` (30) + `packages/cards/test/classic/085-king-wagtoggle.test.ts` (106) |
| `crates/cards/src/scripts/classic/c086_genn.rs` | new | script and tests from `packages/cards/src/scripts/classic/086-genn.ts` (19) + `packages/cards/test/classic/086-genn.test.ts` (43) |
| `crates/cards/src/scripts/classic/c087_plague_chalice.rs` | new | script and tests from `packages/cards/src/scripts/classic/087-plague-chalice.ts` (57) + `packages/cards/test/classic/087-plague-chalice.test.ts` (350) |
| `crates/cards/src/scripts/classic/c088_siphon_squad.rs` | new | script and tests from `packages/cards/src/scripts/classic/088-siphon-squad.ts` (50) + `packages/cards/test/classic/088-siphon-squad.test.ts` (210) |
| `crates/cards/src/scripts/classic/c089_paul_allens_ghost.rs` | new | script and tests from `packages/cards/src/scripts/classic/089-paul-allens-ghost.ts` (34) + `packages/cards/test/classic/089-paul-allens-ghost.test.ts` (269) |
| `crates/cards/src/scripts/classic/c090_in_too_deep.rs` | new | script and tests from `packages/cards/src/scripts/classic/090-in-too-deep.ts` (255) + `packages/cards/test/classic/090-in-too-deep.test.ts` (782) |
| `crates/cards/src/scripts/classic/t_glitch_glitch.rs` | new | script and tests from `packages/cards/src/scripts/classic/t-glitch-glitch.ts` (22) + `packages/cards/test/classic/t-glitch-glitch.test.ts` (104) |

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
- [ ] `.fullsend/notes/part-14.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- A card that calls an engine helper the prelude does not re-export: name it by its module path (`jackioh_engine::zones::…`) and list it under GAPS.
- Object literals with a function inside (a filter callback) cannot go through `json!`: build that argument in Rust and set the function field directly.
- Seed-pinned tests (`createRng("…")` with expected picks) must keep their seeds; the Rust RNG is bit-identical (§6.3).

<!-- /jackioh-bot:plan -->
