# v0.3.0 (part 11 of 40): cards lane 3: Core #82–#100 and the Core tokens to Ghoul

Part 11 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | nothing (the bot works from `main`) |
| Branch | the night bot's own branch and pull request to `main` (new files only; the orchestrator merges `main` into `staging`) |
| Builder | the night bot, `difficulty:hard` (Opus builds and reviews) |
| Notes | `.fullsend/notes/part-11.md` and `.assumptions` (SURFACE §16) |

Queued for the night bot by #306's orchestrator (`bot:build`, `difficulty:hard`). The Plan below is between the night bot's plan markers: build from it without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port every card in cards lane 3 (the table below, in order): each TypeScript script and its test file become one Rust file holding the script and, at the bottom, its tests (SURFACE.md §7 and §8). The behaviour must match the TypeScript exactly; the golden traces (part 23) and these tests decide it in Wave 3.

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
| `crates/cards/src/scripts/core/c082_kys_trial.rs` | new | script and tests from `packages/cards/src/scripts/082-kys-trial.ts` (93) + `packages/cards/test/082-kys-trial.test.ts` (179) |
| `crates/cards/src/scripts/core/c083_transmogulate.rs` | new | script and tests from `packages/cards/src/scripts/083-transmogulate.ts` (146) + `packages/cards/test/083-transmogulate.test.ts` (267); imports `crate::query::*` (part 9) |
| `crates/cards/src/scripts/core/c084_going_long.rs` | new | script and tests from `packages/cards/src/scripts/084-going-long.ts` (50) + `packages/cards/test/084-going-long.test.ts` (262) |
| `crates/cards/src/scripts/core/c085_unlicensed_experimentation.rs` | new | script and tests from `packages/cards/src/scripts/085-unlicensed-experimentation.ts` (176) + `packages/cards/test/085-unlicensed-experimentation.test.ts` (408) |
| `crates/cards/src/scripts/core/c086_miss_mrow.rs` | new | script and tests from `packages/cards/src/scripts/086-miss-mrow.ts` (40) + `packages/cards/test/086-miss-mrow.test.ts` (213) |
| `crates/cards/src/scripts/core/c087_pocket_chaos.rs` | new | script and tests from `packages/cards/src/scripts/087-pocket-chaos.ts` (99) + `packages/cards/test/087-pocket-chaos.test.ts` (355) |
| `crates/cards/src/scripts/core/c088_twisting_nether.rs` | new | script and tests from `packages/cards/src/scripts/088-twisting-nether.ts` (67) + `packages/cards/test/088-twisting-nether.test.ts` (181) |
| `crates/cards/src/scripts/core/c089_corpse_eater.rs` | new | script and tests from `packages/cards/src/scripts/089-corpse-eater.ts` (89) + `packages/cards/test/089-corpse-eater.test.ts` (248) |
| `crates/cards/src/scripts/core/c090_1_cn_virus.rs` | new | script and tests from `packages/cards/src/scripts/090-1-cn-virus.ts` (85) |
| `crates/cards/src/scripts/core/c090_cn_viral_injection.rs` | new | script and tests from `packages/cards/src/scripts/090-cn-viral-injection.ts` (52) + `packages/cards/test/090-cn-viral-injection.test.ts` (607) |
| `crates/cards/src/scripts/core/c091_fed_fauci.rs` | new | script and tests from `packages/cards/src/scripts/091-fed-fauci.ts` (95) + `packages/cards/test/091-fed-fauci.test.ts` (198) |
| `crates/cards/src/scripts/core/c092_felinor_fiender.rs` | new | script and tests from `packages/cards/src/scripts/092-felinor-fiender.ts` (87) + `packages/cards/test/092-felinor-fiender.test.ts` (310) |
| `crates/cards/src/scripts/core/c093_1_combo_fodder.rs` | new | script and tests from `packages/cards/src/scripts/093-1-combo-fodder.ts` (55) |
| `crates/cards/src/scripts/core/c093_combo_index.rs` | new | script and tests from `packages/cards/src/scripts/093-combo-index.ts` (96) + `packages/cards/test/093-combo-index.test.ts` (971) |
| `crates/cards/src/scripts/core/c094_genns_greed.rs` | new | script and tests from `packages/cards/src/scripts/094-genns-greed.ts` (80) + `packages/cards/test/094-genns-greed.test.ts` (418) |
| `crates/cards/src/scripts/core/c095_1_chaos_golem.rs` | new | script and tests from `packages/cards/src/scripts/095-1-chaos-golem.ts` (41) |
| `crates/cards/src/scripts/core/c095_call_to_chaos.rs` | new | script and tests from `packages/cards/src/scripts/095-call-to-chaos.ts` (41) + `packages/cards/test/095-call-to-chaos.test.ts` (738) |
| `crates/cards/src/scripts/core/c096_my_pawn.rs` | new | script and tests from `packages/cards/src/scripts/096-my-pawn.ts` (130) + `packages/cards/test/096-my-pawn.test.ts` (470) |
| `crates/cards/src/scripts/core/c097_zephyrs.rs` | new | script and tests from `packages/cards/src/scripts/097-zephyrs.ts` (83) + `packages/cards/test/097-zephyrs.test.ts` (381) |
| `crates/cards/src/scripts/core/c098_heroic_power.rs` | new | script and tests from `packages/cards/src/scripts/098-heroic-power.ts` (62) + `packages/cards/test/098-heroic-power.test.ts` (450) |
| `crates/cards/src/scripts/core/c099_craft_a_card.rs` | new | script and tests from `packages/cards/src/scripts/099-craft-a-card.ts` (120) + `packages/cards/test/099-craft-a-card.test.ts` (504) |
| `crates/cards/src/scripts/core/c100_ceaseless_void.rs` | new | script and tests from `packages/cards/src/scripts/100-ceaseless-void.ts` (103) + `packages/cards/test/100-ceaseless-void.test.ts` (440) |
| `crates/cards/src/scripts/core/t_bread.rs` | new | script and tests from `packages/cards/src/scripts/t-bread.ts` (37) + `packages/cards/test/t-bread.test.ts` (251) |
| `crates/cards/src/scripts/core/t_coin.rs` | new | script and tests from `packages/cards/src/scripts/t-coin.ts` (38) + `packages/cards/test/t-coin.test.ts` (320) |
| `crates/cards/src/scripts/core/t_felinor.rs` | new | script and tests from `packages/cards/src/scripts/t-felinor.ts` (31) + `packages/cards/test/t-felinor.test.ts` (234) |
| `crates/cards/src/scripts/core/t_ghoul.rs` | new | script and tests from `packages/cards/src/scripts/t-ghoul.ts` (29) + `packages/cards/test/t-ghoul.test.ts` (177) |

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
- [ ] `.fullsend/notes/part-11.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- A card that calls an engine helper the prelude does not re-export: name it by its module path (`jackioh_engine::zones::…`) and list it under GAPS.
- Object literals with a function inside (a filter callback) cannot go through `json!`: build that argument in Rust and set the function field directly.
- Seed-pinned tests (`createRng("…")` with expected picks) must keep their seeds; the Rust RNG is bit-identical (§6.3).

<!-- /jackioh-bot:plan -->
