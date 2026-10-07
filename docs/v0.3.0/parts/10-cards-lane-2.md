# v0.3.0 (part 10 of 40): cards lane 2: Core #49–#81

Part 10 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | nothing (the bot works from `main`) |
| Branch | the night bot's own branch and pull request to `main` (new files only; the orchestrator merges `main` into `staging`) |
| Builder | the night bot, `difficulty:hard` (Opus builds and reviews) |
| Notes | `.fullsend/notes/part-10.md` and `.assumptions` (SURFACE §16) |

Queued for the night bot by #306's orchestrator (`bot:build`, `difficulty:hard`). The Plan below is between the night bot's plan markers: build from it without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port every card in cards lane 2 (the table below, in order): each TypeScript script and its test file become one Rust file holding the script and, at the bottom, its tests (SURFACE.md §7 and §8). The behaviour must match the TypeScript exactly; the golden traces (part 23) and these tests decide it in Wave 3.

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
| `crates/cards/src/scripts/core/c049_snom_bunny_mind_control.rs` | new | script and tests from `packages/cards/src/scripts/049-snom-bunny-mind-control.ts` (50) + `packages/cards/test/049-snom-bunny-mind-control.test.ts` (182) |
| `crates/cards/src/scripts/core/c050_k_pop_fanatic.rs` | new | script and tests from `packages/cards/src/scripts/050-k-pop-fanatic.ts` (184) + `packages/cards/test/050-k-pop-fanatic.test.ts` (737) |
| `crates/cards/src/scripts/core/c051_1_kys_empty_notebook.rs` | new | script and tests from `packages/cards/src/scripts/051-1-kys-empty-notebook.ts` (42) + `packages/cards/test/051-1-kys-empty-notebook.test.ts` (131) |
| `crates/cards/src/scripts/core/c051_kys_private_tutor.rs` | new | script and tests from `packages/cards/src/scripts/051-kys-private-tutor.ts` (277) + `packages/cards/test/051-kys-private-tutor.test.ts` (460) |
| `crates/cards/src/scripts/core/c052_silly_silas.rs` | new | script and tests from `packages/cards/src/scripts/052-silly-silas.ts` (93) + `packages/cards/test/052-silly-silas.test.ts` (258) |
| `crates/cards/src/scripts/core/c053_reno.rs` | new | script and tests from `packages/cards/src/scripts/053-reno.ts` (51) + `packages/cards/test/053-reno.test.ts` (83) |
| `crates/cards/src/scripts/core/c054_straaza.rs` | new | script and tests from `packages/cards/src/scripts/054-straaza.ts` (71) + `packages/cards/test/054-straaza.test.ts` (234) |
| `crates/cards/src/scripts/core/c055_lava_golem.rs` | new | script and tests from `packages/cards/src/scripts/055-lava-golem.ts` (54) + `packages/cards/test/055-lava-golem.test.ts` (300) |
| `crates/cards/src/scripts/core/c056_jilliax.rs` | new | script and tests from `packages/cards/src/scripts/056-jilliax.ts` (33) + `packages/cards/test/056-jilliax.test.ts` (177) |
| `crates/cards/src/scripts/core/c057_conjure_ky.rs` | new | script and tests from `packages/cards/src/scripts/057-conjure-ky.ts` (51) + `packages/cards/test/057-conjure-ky.test.ts` (159) |
| `crates/cards/src/scripts/core/c058_rush_token_farm.rs` | new | script and tests from `packages/cards/src/scripts/058-rush-token-farm.ts` (48) + `packages/cards/test/058-rush-token-farm.test.ts` (182) |
| `crates/cards/src/scripts/core/c059_unbiased_immigration.rs` | new | script and tests from `packages/cards/src/scripts/059-unbiased-immigration.ts` (59) + `packages/cards/test/059-unbiased-immigration.test.ts` (230) |
| `crates/cards/src/scripts/core/c060_bear_honeypot.rs` | new | script and tests from `packages/cards/src/scripts/060-bear-honeypot.ts` (134) + `packages/cards/test/060-bear-honeypot.test.ts` (476) |
| `crates/cards/src/scripts/core/c061_prejudiced_postdoc.rs` | new | script and tests from `packages/cards/src/scripts/061-prejudiced-postdoc.ts` (68) + `packages/cards/test/061-prejudiced-postdoc.test.ts` (191) |
| `crates/cards/src/scripts/core/c062_friend_of_felinors.rs` | new | script and tests from `packages/cards/src/scripts/062-friend-of-felinors.ts` (36) + `packages/cards/test/062-friend-of-felinors.test.ts` (92) |
| `crates/cards/src/scripts/core/c063_plastic_surgery.rs` | new | script and tests from `packages/cards/src/scripts/063-plastic-surgery.ts` (49) + `packages/cards/test/063-plastic-surgery.test.ts` (153) |
| `crates/cards/src/scripts/core/c064_gifted_program.rs` | new | script and tests from `packages/cards/src/scripts/064-gifted-program.ts` (40) + `packages/cards/test/064-gifted-program.test.ts` (190) |
| `crates/cards/src/scripts/core/c065_1_spikey_pillow.rs` | new | script and tests from `packages/cards/src/scripts/065-1-spikey-pillow.ts` (61) + `packages/cards/test/065-1-spikey-pillow.test.ts` (134) |
| `crates/cards/src/scripts/core/c065_masochism_mask.rs` | new | script and tests from `packages/cards/src/scripts/065-masochism-mask.ts` (112) + `packages/cards/test/065-masochism-mask.test.ts` (156) |
| `crates/cards/src/scripts/core/c066_the_rock.rs` | new | script and tests from `packages/cards/src/scripts/066-the-rock.ts` (50) + `packages/cards/test/066-the-rock.test.ts` (183) |
| `crates/cards/src/scripts/core/c067_zoomerbin_oomen.rs` | new | script and tests from `packages/cards/src/scripts/067-zoomerbin-oomen.ts` (76) + `packages/cards/test/067-zoomerbin-oomen.test.ts` (249); imports `crate::query::*` (part 9) |
| `crates/cards/src/scripts/core/c068_twisted_sorcerer.rs` | new | script and tests from `packages/cards/src/scripts/068-twisted-sorcerer.ts` (89) + `packages/cards/test/068-twisted-sorcerer.test.ts` (167) |
| `crates/cards/src/scripts/core/c069_call_to_arms.rs` | new | script and tests from `packages/cards/src/scripts/069-call-to-arms.ts` (52) + `packages/cards/test/069-call-to-arms.test.ts` (205) |
| `crates/cards/src/scripts/core/c070_spiteful_stab.rs` | new | script and tests from `packages/cards/src/scripts/070-spiteful-stab.ts` (88) + `packages/cards/test/070-spiteful-stab.test.ts` (227) |
| `crates/cards/src/scripts/core/c071_intern_stimmy.rs` | new | script and tests from `packages/cards/src/scripts/071-intern-stimmy.ts` (82) + `packages/cards/test/071-intern-stimmy.test.ts` (174) |
| `crates/cards/src/scripts/core/c072_reminisce.rs` | new | script and tests from `packages/cards/src/scripts/072-reminisce.ts` (68) + `packages/cards/test/072-reminisce.test.ts` (217) |
| `crates/cards/src/scripts/core/c073_anti_oneshot_armor.rs` | new | script and tests from `packages/cards/src/scripts/073-anti-oneshot-armor.ts` (49) + `packages/cards/test/073-anti-oneshot-armor.test.ts` (228) |
| `crates/cards/src/scripts/core/c074_adaptive_ui.rs` | new | script and tests from `packages/cards/src/scripts/074-adaptive-ui.ts` (83) + `packages/cards/test/074-adaptive-ui.test.ts` (218) |
| `crates/cards/src/scripts/core/c075_infinite_reserves.rs` | new | script and tests from `packages/cards/src/scripts/075-infinite-reserves.ts` (41) + `packages/cards/test/075-infinite-reserves.test.ts` (165) |
| `crates/cards/src/scripts/core/c076_field_of_dreams.rs` | new | script and tests from `packages/cards/src/scripts/076-field-of-dreams.ts` (64) + `packages/cards/test/076-field-of-dreams.test.ts` (136) |
| `crates/cards/src/scripts/core/c077_professor_curvature.rs` | new | script and tests from `packages/cards/src/scripts/077-professor-curvature.ts` (52) + `packages/cards/test/077-professor-curvature.test.ts` (209) |
| `crates/cards/src/scripts/core/c078_fullsend.rs` | new | script and tests from `packages/cards/src/scripts/078-fullsend.ts` (100) + `packages/cards/test/078-fullsend.test.ts` (290) |
| `crates/cards/src/scripts/core/c079_twinspell.rs` | new | script and tests from `packages/cards/src/scripts/079-twinspell.ts` (37) + `packages/cards/test/079-twinspell.test.ts` (201) |
| `crates/cards/src/scripts/core/c080_zao_gao.rs` | new | script and tests from `packages/cards/src/scripts/080-zao-gao.ts` (71) + `packages/cards/test/080-zao-gao.test.ts` (304) |
| `crates/cards/src/scripts/core/c081_radiant_saintess.rs` | new | script and tests from `packages/cards/src/scripts/081-radiant-saintess.ts` (89) + `packages/cards/test/081-radiant-saintess.test.ts` (314) |

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
- [ ] `.fullsend/notes/part-10.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- A card that calls an engine helper the prelude does not re-export: name it by its module path (`jackioh_engine::zones::…`) and list it under GAPS.
- Object literals with a function inside (a filter callback) cannot go through `json!`: build that argument in Rust and set the function field directly.
- Seed-pinned tests (`createRng("…")` with expected picks) must keep their seeds; the Rust RNG is bit-identical (§6.3).

<!-- /jackioh-bot:plan -->
