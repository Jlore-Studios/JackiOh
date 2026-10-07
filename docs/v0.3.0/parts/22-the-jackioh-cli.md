# v0.3.0 (part 22 of 40): the jackioh CLI: fuzz, catalog, patches, gates, sweep, stats

Part 22 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | nothing (the bot works from `main`) |
| Branch | the night bot's own branch and pull request to `main` (new files only; the orchestrator merges `main` into `staging`) |
| Builder | the night bot, `difficulty:hard` (Opus builds and reviews) |
| Notes | `.fullsend/notes/part-22.md` and `.assumptions` (SURFACE §16) |

Queued for the night bot by #306's orchestrator (`bot:build`, `difficulty:hard`). The Plan below is between the night bot's plan markers: build from it without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Write the `jackioh` CLI's commands that replace the TypeScript tooling: `fuzz`, `replay`, `trace`, `catalog check|loc`, `catalog-version`, `patches` (new, check, ship), `gate` (and `gate merge`), `sweep`, `stats` (SURFACE §12), and port the card data tests and the patch-history tests that go with them.

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
| `crates/tools/src/{replay,catalog,fuzz,gate,sweep,stats,trace,patches}.rs` | new | each `pub struct Args` (clap) + `pub fn run(args) -> anyhow::Result<()>` |
| `crates/tools/src/gate.rs` | new | port of `packages/ai/scripts/gate-merge.ts` (70 lines).; port of `packages/ai/test/gate-greedy.test.ts` (172 lines). the gates become `cargo jackioh gate` (smoke run as a #[test] with 4 games); port of `packages/ai/test/gate-hard-easy.test.ts` (123 lines). the gates become `cargo jackioh gate` (smoke run as a #[test] with 4 games); port of `packages/ai/test/gate-perf.test.ts` (145 lines). the gates become `cargo jackioh gate` (smoke run as a #[test] with 4 games); port of `packages/ai/test/gate-random.test.ts` (152 lines). the gates become `cargo jackioh gate` (smoke run as a #[test] with 4 games) |
| `crates/tools/src/stats.rs` | new | port of `packages/ai/scripts/stats.ts` (121 lines). |
| `crates/tools/src/sweep.rs` | new | port of `packages/ai/scripts/sweep.ts` (272 lines). |
| `crates/tools/src/trace.rs` | new | port of `packages/ai/scripts/trace.ts` (97 lines). |
| `crates/tools/src/patches.rs` | new | port of `packages/cards/scripts/naming.ts` (249 lines).; port of `packages/cards/scripts/patch.ts` (64 lines).; port of `packages/cards/scripts/patches-io.ts` (418 lines).; port of `packages/cards/scripts/patches.ts` (372 lines).; port of `packages/cards/scripts/versions.ts` (39 lines).; port of `packages/cards/test/patches-ship.test.ts` (545 lines).; port of `packages/cards/test/patches.test.ts` (437 lines).; port of `packages/cards/test/versions.test.ts` (26 lines). |
| `crates/tools/src/catalog.rs` | new | port of `packages/cards/scripts/validate-catalog.ts` (655 lines). |
| `crates/cards/tests/cross/card_text.rs` | new | port of `packages/cards/test/card-text.test.ts` (689 lines). |
| `crates/cards/tests/cross/catalog.rs` | new | port of `packages/cards/test/catalog.test.ts` (820 lines). |
| `crates/cards/tests/cross/flavour.rs` | new | port of `packages/cards/test/flavour.test.ts` (59 lines). |
| `crates/tools/src/fuzz.rs` | new | port of `packages/cards/test/fuzz-handicap.test.ts` (154 lines). the fuzz loop becomes `cargo jackioh fuzz`; its tests seeds 1–20 as `#[cfg(test)]`; port of `packages/cards/test/fuzz.test.ts` (555 lines). the fuzz loop becomes `cargo jackioh fuzz`; its tests seeds 1–20 as `#[cfg(test)]` |
| `crates/cards/tests/cross/params.rs` | new | port of `packages/cards/test/params.test.ts` (128 lines). |
| `crates/cards/tests/cross/pools_and_randomness.rs` | new | port of `packages/cards/test/pools-and-randomness.test.ts` (157 lines). |
| `crates/cards/tests/cross/query.rs` | new | port of `packages/cards/test/query.test.ts` (402 lines). |
| `crates/cards/tests/cross/radiant_standard.rs` | new | port of `packages/cards/test/radiant-standard.test.ts` (103 lines). |
| `crates/cards/tests/cross/references.rs` | new | port of `packages/cards/test/references.test.ts` (157 lines). |
| `crates/cards/tests/cross/registry.rs` | new | port of `packages/cards/test/registry.test.ts` (331 lines). |

Do **not** touch: any `Cargo.toml`, `lib.rs`, `mod.rs` or `main.rs` (part 1's), other parts' files, `packages/` and `apps/server/` (read only; part 37 deletes them).

### 3. Steps

1. Read SURFACE.md §3, §7.5, §12 and §13.1 fully, and each TS source in your table.
2. `fuzz.rs`: seeds and deals exactly as `fuzz.test.ts` (pool = all non-token ids sorted, `createRng("jackioh-fuzz-decks-<seed>")`, game seed `jackioh-fuzz-<seed>`, policy `subsystems::choose_action` with `jackioh-fuzz-policy-<seed>`, mulligan order rng), the same assertions (termination reasons, turn cap, step cap, every pick accepted, `testkit::invariants::Monitor` I1–I4 each step, `fold` hash equals the live hash), `--handicap` as `fuzz-handicap.test.ts`; rayon over seeds; a failure prints the seed, the step and both decks. Its `#[cfg(test)]` runs seeds 1–20.
3. `replay.rs`: stdin JSON → `fold` → `{"hash": hash_state, "errors": […]}` on stdout.
4. `catalog.rs`: `validate-catalog.ts`'s checks and the data tests in the table (catalog tables, card-text, flavour, params, references, registry counts, radiant-standard against `docs/radiant-audit.md` by index and name only, query, pools-and-randomness), `loc` per SURFACE §7.5, `catalog-version`.
5. `patches.rs`: `patches.ts`, `patch.ts`, `patches-io.ts`, `versions.ts`, `naming.ts` with the same files and formats under `crates/cards/patches/`, and `ship` bumping `CATALOG_VERSION` everywhere it lives after the rewrite (`render.yaml`, `crates/server/.env.example`, `apps/web/.env.production`'s `VITE_CATALOG_VERSION` if present); the patch tests in the table.
6. `gate.rs`: the three matchups of `gate.ts` (`AI_GATE`, `gateNeeded`, seeds `gate:v3:<matchup>:<n>`, subject on p1 for odd n, decks per `gameConfig`, fold-back hash check, turn-cap draws reported), `--shard k/K`, `--out`, `merge`, and the perf gate; its `#[cfg(test)]` plays 4 games of each.
7. `sweep.rs`, `stats.rs`, `trace.rs`: the TS scripts' behaviour and output, over `jackioh_ai`.
8. Commit after every 3 files on your branch.

### 4. Tests

- fuzz seeds 1–20 and gate smoke (4 games per matchup) as unit tests; the catalog and patches tests in the table.

### 5. Done when

- [ ] Every command of SURFACE §12 owned by part 22 runs its TS predecessor's checks.
- [ ] `.fullsend/notes/part-22.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- `patches ship` runs in `patches-ship.yml` on main after cutover; part 30 changes that workflow to call it.

<!-- /jackioh-bot:plan -->
