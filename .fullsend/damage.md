# Damage (fullsend Phase 3, part 31)

The first build of `staging` after Wave 1 (`d8a87d9`, marker `wave-1`) and what the reconcile left. Per-crate detail, with the four fullsend lists, is in `.fullsend/damage/<crate>.md`; decisions in `.fullsend/notes/reconcile-decisions.md`; SURFACE holes in `.fullsend/notes/spec-gaps.md` (now `SURFACE.md` §17).

| Crate / target | Errors before | Errors after reconcile | How measured |
|---|---|---|---|
| engine lib | 187 (E0432/E0433 seams, E0308, missing fields) | 45, all local E0308 | real build |
| engine tests (`tests/rules`) | 2,775 (lib stubbed) | 384 local, then 346 on the compiling lib | shadow, then real |
| cards (lib, card tests, `tests/cross`) | 183 (engine stubbed) | 0 errors, 273 clippy warnings | real build |
| ai (lib, tests) | 62 (21 + 41) | 0 | shadow build |
| server (lib, tests) | 115 (lib) | lib 0, tests 21 local | shadow build on the real engine |
| tools, wasm | 3 | 0 errors, 37 clippy lints | shadow build |

`BUILDS-RUN`: 0 in every Wave 1 part's notes except part 1 (allowed by its brief).

`git diff --shortstat` over `crates/` from the `wave-1` marker to the end of the reconcile: more deletions than insertions (merges would show the reverse).

## Collisions left for part 37's cull (small helpers copied across crates; each compiles, none is a second design)

`is_js_space` (engine, server, tools), `MAX_SAFE_INTEGER` (engine, server, tools), `PERCENT` (cards, engine, server), `panic_message` (ai, server, tools), `utf16_len` and `whole_number` (engine, server); `crates/cards/tests/cross/registry.rs`'s copy of tools' `naming` (a test binary cannot import a binary crate: move those tests into `patches.rs`); Classic #28's two-line `recalled` reader; `crates/tools/src/arena.rs`'s referee loop beside `jackioh_ai::play_match` (needs an external-agent seat in the AI crate); `wire/codes.rs`'s hand-written NFKC tables, `patches::js::Json` and the hand-written SHA-1 (crates exist; SURFACE §2 names none).

## Seams still open for part 35

Two R149 room-code tests and three R263 compare-and-set tests (need an injectable code source and a fake-store hook), the test app always running with `E2E=1`, and `season_start.rs`'s v0.1 season.
