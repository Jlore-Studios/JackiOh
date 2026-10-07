# v0.3.0 (part 16 of 40): cards lane 8: Classic+ #50–#78 and the AI tokens

Part 16 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | nothing (the bot works from `main`) |
| Branch | the night bot's own branch and pull request to `main` (new files only; the orchestrator merges `main` into `staging`) |
| Builder | the night bot, `difficulty:hard` (Opus builds and reviews) |
| Notes | `.fullsend/notes/part-16.md` and `.assumptions` (SURFACE §16) |

Queued for the night bot by #306's orchestrator (`bot:build`, `difficulty:hard`). The Plan below is between the night bot's plan markers: build from it without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port every card in cards lane 8 (the table below, in order): each TypeScript script and its test file become one Rust file holding the script and, at the bottom, its tests (SURFACE.md §7 and §8). The behaviour must match the TypeScript exactly; the golden traces (part 23) and these tests decide it in Wave 3.

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
| `crates/cards/src/scripts/classic_plus/c050_adaptive_growth.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/050-adaptive-growth.ts` (46) + `packages/cards/test/classic-plus/050-adaptive-growth.test.ts` (158) |
| `crates/cards/src/scripts/classic_plus/c051_jlockheeds_j15_fighter.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/051-jlockheeds-j15-fighter.ts` (20) + `packages/cards/test/classic-plus/051-jlockheeds-j15-fighter.test.ts` (120) |
| `crates/cards/src/scripts/classic_plus/c052_jlockheeds_permanent_defense_contract.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/052-jlockheeds-permanent-defense-contract.ts` (50) + `packages/cards/test/classic-plus/052-jlockheeds-permanent-defense-contract.test.ts` (225) |
| `crates/cards/src/scripts/classic_plus/c053_book_of_tokens.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/053-book-of-tokens.ts` (44) + `packages/cards/test/classic-plus/053-book-of-tokens.test.ts` (127) |
| `crates/cards/src/scripts/classic_plus/c054_book_of_books.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/054-book-of-books.ts` (25) + `packages/cards/test/classic-plus/054-book-of-books.test.ts` (163) |
| `crates/cards/src/scripts/classic_plus/c055_book_of_greed.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/055-book-of-greed.ts` (22) + `packages/cards/test/classic-plus/055-book-of-greed.test.ts` (128) |
| `crates/cards/src/scripts/classic_plus/c056_book_of_pain.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/056-book-of-pain.ts` (21) + `packages/cards/test/classic-plus/056-book-of-pain.test.ts` (135) |
| `crates/cards/src/scripts/classic_plus/c057_book_of_stats.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/057-book-of-stats.ts` (26) + `packages/cards/test/classic-plus/057-book-of-stats.test.ts` (118) |
| `crates/cards/src/scripts/classic_plus/c058_fruit_basket.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/058-fruit-basket.ts` (22) + `packages/cards/test/classic-plus/058-fruit-basket.test.ts` (125) |
| `crates/cards/src/scripts/classic_plus/c059_all_purpose_apple.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/059-all-purpose-apple.ts` (35) + `packages/cards/test/classic-plus/059-all-purpose-apple.test.ts` (118) |
| `crates/cards/src/scripts/classic_plus/c060_doctors_orders.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/060-doctors-orders.ts` (28) + `packages/cards/test/classic-plus/060-doctors-orders.test.ts` (156) |
| `crates/cards/src/scripts/classic_plus/c061_bauble_bubble.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/061-bauble-bubble.ts` (29) + `packages/cards/test/classic-plus/061-bauble-bubble.test.ts` (188) |
| `crates/cards/src/scripts/classic_plus/c062_kys_papaya.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/062-kys-papaya.ts` (23) + `packages/cards/test/classic-plus/062-kys-papaya.test.ts` (392) |
| `crates/cards/src/scripts/classic_plus/c063_fruit_tree.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/063-fruit-tree.ts` (39) + `packages/cards/test/classic-plus/063-fruit-tree.test.ts` (196) |
| `crates/cards/src/scripts/classic_plus/c064_mulch_muncher.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/064-mulch-muncher.ts` (29) + `packages/cards/test/classic-plus/064-mulch-muncher.test.ts` (167) |
| `crates/cards/src/scripts/classic_plus/c065_1_rotten_grape.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/065-1-rotten-grape.ts` (25) + `packages/cards/test/classic-plus/065-1-rotten-grape.test.ts` (111) |
| `crates/cards/src/scripts/classic_plus/c065_2_normal_grape.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/065-2-normal-grape.ts` (42) + `packages/cards/test/classic-plus/065-2-normal-grape.test.ts` (209) |
| `crates/cards/src/scripts/classic_plus/c065_3_large_grape.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/065-3-large-grape.ts` (33) + `packages/cards/test/classic-plus/065-3-large-grape.test.ts` (143) |
| `crates/cards/src/scripts/classic_plus/c065_4_golden_grape.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/065-4-golden-grape.ts` (56) + `packages/cards/test/classic-plus/065-4-golden-grape.test.ts` (173) |
| `crates/cards/src/scripts/classic_plus/c065_5_mythic_grape.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/065-5-mythic-grape.ts` (35) + `packages/cards/test/classic-plus/065-5-mythic-grape.test.ts` (115) |
| `crates/cards/src/scripts/classic_plus/c065_two_grapes.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/065-two-grapes.ts` (30) + `packages/cards/test/classic-plus/065-two-grapes.test.ts` (220) |
| `crates/cards/src/scripts/classic_plus/c066_vine_of_grapes.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/066-vine-of-grapes.ts` (26) + `packages/cards/test/classic-plus/066-vine-of-grapes.test.ts` (116) |
| `crates/cards/src/scripts/classic_plus/c067_pear.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/067-pear.ts` (36) + `packages/cards/test/classic-plus/067-pear.test.ts` (152) |
| `crates/cards/src/scripts/classic_plus/c068_organic_produce.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/068-organic-produce.ts` (29) + `packages/cards/test/classic-plus/068-organic-produce.test.ts` (196) |
| `crates/cards/src/scripts/classic_plus/c069_buff_billy.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/069-buff-billy.ts` (25) + `packages/cards/test/classic-plus/069-buff-billy.test.ts` (164) |
| `crates/cards/src/scripts/classic_plus/c070_chaos_machine.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/070-chaos-machine.ts` (28) + `packages/cards/test/classic-plus/070-chaos-machine.test.ts` (228) |
| `crates/cards/src/scripts/classic_plus/c071_book_of_buff.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/071-book-of-buff.ts` (20) + `packages/cards/test/classic-plus/071-book-of-buff.test.ts` (132) |
| `crates/cards/src/scripts/classic_plus/c072_book_of_nerf.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/072-book-of-nerf.ts` (25) + `packages/cards/test/classic-plus/072-book-of-nerf.test.ts` (171) |
| `crates/cards/src/scripts/classic_plus/c073_1_classic_golem.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/073-1-classic-golem.ts` (59) + `packages/cards/test/classic-plus/073-1-classic-golem.test.ts` (206) |
| `crates/cards/src/scripts/classic_plus/c073_call_to_chaos_classic_edition.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/073-call-to-chaos-classic-edition.ts` (26) + `packages/cards/test/classic-plus/073-call-to-chaos-classic-edition.test.ts` (386) |
| `crates/cards/src/scripts/classic_plus/c074_twice_forward_one_step_backwards.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/074-twice-forward-one-step-backwards.ts` (42) + `packages/cards/test/classic-plus/074-twice-forward-one-step-backwards.test.ts` (329) |
| `crates/cards/src/scripts/classic_plus/c075_1_j_lease_j_jungle_ex_plorer_pack.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/075-1-j-lease-j-jungle-ex-plorer-pack.ts` (40) + `packages/cards/test/classic-plus/075-1-j-lease-j-jungle-ex-plorer-pack.test.ts` (142) |
| `crates/cards/src/scripts/classic_plus/c075_j_lease_j_jungle_ex_plorer.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/075-j-lease-j-jungle-ex-plorer.ts` (30) + `packages/cards/test/classic-plus/075-j-lease-j-jungle-ex-plorer.test.ts` (122) |
| `crates/cards/src/scripts/classic_plus/c076_1_brother_ping.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/076-1-brother-ping.ts` (43) + `packages/cards/test/classic-plus/076-1-brother-ping.test.ts` (214) |
| `crates/cards/src/scripts/classic_plus/c076_brother_lar.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/076-brother-lar.ts` (23) + `packages/cards/test/classic-plus/076-brother-lar.test.ts` (112) |
| `crates/cards/src/scripts/classic_plus/c077_anti_softlock.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/077-anti-softlock.ts` (39) + `packages/cards/test/classic-plus/077-anti-softlock.test.ts` (166) |
| `crates/cards/src/scripts/classic_plus/c078_claudes_datacenter.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/078-claudes-datacenter.ts` (36) + `packages/cards/test/classic-plus/078-claudes-datacenter.test.ts` (128) |
| `crates/cards/src/scripts/classic_plus/t_ai_01_helpful_assistant.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/t-ai-01-helpful-assistant.ts` (28) + `packages/cards/test/classic-plus/t-ai-01-helpful-assistant.test.ts` (129) |
| `crates/cards/src/scripts/classic_plus/t_ai_02_scaling_law.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/t-ai-02-scaling-law.ts` (27) + `packages/cards/test/classic-plus/t-ai-02-scaling-law.test.ts` (112) |
| `crates/cards/src/scripts/classic_plus/t_ai_03_hallucination.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/t-ai-03-hallucination.ts` (27) + `packages/cards/test/classic-plus/t-ai-03-hallucination.test.ts` (157) |
| `crates/cards/src/scripts/classic_plus/t_ai_04_chain_of_thought.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/t-ai-04-chain-of-thought.ts` (26) + `packages/cards/test/classic-plus/t-ai-04-chain-of-thought.test.ts` (146) |
| `crates/cards/src/scripts/classic_plus/t_ai_05_autocomplete.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/t-ai-05-autocomplete.ts` (34) + `packages/cards/test/classic-plus/t-ai-05-autocomplete.test.ts` (199) |
| `crates/cards/src/scripts/classic_plus/t_ai_06_datacenter_fire.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/t-ai-06-datacenter-fire.ts` (42) + `packages/cards/test/classic-plus/t-ai-06-datacenter-fire.test.ts` (143) |
| `crates/cards/src/scripts/classic_plus/t_ai_07_alignment_tax.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/t-ai-07-alignment-tax.ts` (27) + `packages/cards/test/classic-plus/t-ai-07-alignment-tax.test.ts` (127) |
| `crates/cards/src/scripts/classic_plus/t_ai_08_rate_limit.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/t-ai-08-rate-limit.ts` (43) + `packages/cards/test/classic-plus/t-ai-08-rate-limit.test.ts` (185) |
| `crates/cards/src/scripts/classic_plus/t_ai_09_refusal.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/t-ai-09-refusal.ts` (60) + `packages/cards/test/classic-plus/t-ai-09-refusal.test.ts` (223) |
| `crates/cards/src/scripts/classic_plus/t_ai_10_fine_tuning.rs` | new | script and tests from `packages/cards/src/scripts/classic-plus/t-ai-10-fine-tuning.ts` (24) + `packages/cards/test/classic-plus/t-ai-10-fine-tuning.test.ts` (119) |

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
- [ ] `.fullsend/notes/part-16.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- A card that calls an engine helper the prelude does not re-export: name it by its module path (`jackioh_engine::zones::…`) and list it under GAPS.
- Object literals with a function inside (a filter callback) cannot go through `json!`: build that argument in Rust and set the function field directly.
- Seed-pinned tests (`createRng("…")` with expected picks) must keep their seeds; the Rust RNG is bit-identical (§6.3).

<!-- /jackioh-bot:plan -->
