# v0.4.0: the card scripts in Luau

The record of [#442](https://github.com/Jlore-Studios/JackiOh/issues/442): every card script that is
a list of engine verbs moved from Rust to Luau, run by an embedded VM (`mlua`, `crates/luau`) on the
server, in the tools and in the browser. It says what was built, what each part decided that the
code does not explain, which cards stay in Rust and why, and what each part measured. It folds the
notes each part left beside this file while the tree was built; they stay in git history.

- **[PORTING.md](PORTING.md)** is how a card moves from Rust to Luau and how the move is proved.
- **[api-gaps.md](api-gaps.md)** is what the Luau API binds, what it does not, and the fifteen gaps
  part 28 fixed with their Luau forms.
- **[perf.json](perf.json)** is the performance baseline at the branch point (section 5.5).
- **`crates/luau/README.md`** is the host's contract: the sandbox, the integer boundary, the lint,
  the VM, the `J` bindings. **`crates/cards/README.md`** is the cards crate's.

Contents:

1. What v0.4.0 is
2. Decisions
3. How it was built
4. The cards kept in Rust
5. The record, part by part
6. What the batches found
7. The reconcile
8. v0.5.0: official cards back in Rust

---

## 1. What v0.4.0 is

- **409 of the 450 cards and tokens run from Luau.** Each card has its `.rs`
  (`crates/cards/src/scripts/<set>/<file>.rs`: `pub const ID`, the header comment, `mod tests`) and
  one script: `<file>.luau` beside it, or `pub fn script()` in the `.rs` for one of the 41 cards of
  section 4. By set: Core 103 of 111, Classic 82 of 91, Classic+ 104 of 116, Meditative 120 of 132.
- **The tests stay Rust** (L16): a ported card's tests are the ones it had, unchanged in what they
  assert, run against the Luau script through the registry (`crate::scripts_of()[ID]`).
- **`crates/luau`** hosts the scripts: the sandbox (L5), the integer boundary (L6), the build-time
  lint (L6, L7), one VM per thread holding compiled chunks and no game state (L8), the `J` module
  (`src/api/`, one file per `effects/` module; `reads.rs`; `hooks.rs`) and `load_card`, which turns
  a compiled script into the engine's `CardScripts`. `crates/cards/build.rs` lints each `.luau`,
  compiles it to bytecode and embeds it (L11).
- **`crates/cards/luau/jackioh.luau`** types `J` for the scripts, generated from the Rust through
  `ts-rs` by `crates/luau/tests/export_api.rs` (L10).
- **The browser** runs the same engine, cards and Luau as a `wasm32-unknown-emscripten` binary
  (`crates/wasm`, built by `scripts/build-wasm.sh` with a pinned emsdk), since `mlua` builds Luau for
  no other WebAssembly target.
- **No catalog version** (L1): the game plays the same; the catalog and its patch history did not
  change for the port.

## 2. Decisions

Decided in #442; the full list (L1–L27) is in the issue. These are the ones the code and this
record lean on.

| # | Decision |
|---|---|
| L1 | v0.4.0 ships no catalog version: nothing a player sees changes. |
| L3 | `mlua` with Luau 0.740, pinned (`mlua = "=0.12.2"`, `luau0-src 0.22.0+luau740`). |
| L5 | The sandbox: safe standard libraries only; `print`, `math.random`, `os`, `debug` and the like are gone. |
| L6 | Only integers cross the boundary (`numbers.rs`, `i32`); the lint refuses `/`, `^` and most of `math.` (`J.div`, `J.rem` instead). |
| L7 | Deterministic order: no `pairs`, `next` or `table.sort` (`ipairs` and `J.sort_by` instead). |
| L8 | Stateless: one VM per thread, and each hook call re-runs its chunk. |
| L9 | The API mirrors the Rust: one `J` function per effect verb, under its name; `ctx`'s readers are the read helpers, under theirs. |
| L10 | The typed module is generated from the Rust through `ts-rs`. |
| L11 | Scripts are embedded at build time as bytecode. |
| L12 | The browser's budget: module and glue gzipped within 1 MiB of `main`'s; `register_all` under 200 ms; a golden step at most 2× `main`'s. |
| L13 | The performance gate, `luau bench`: the slowest gate decision at most 1.5× the parent's, `fuzz --seeds 200` wall time and a golden step at most 2×. |
| L14 | Fourteen cards stay Rust by decision: glue to an engine subsystem written for that one card (section 4). |
| L15 | Each batch proves equivalence with `luau diff` against a parent build. |
| L16 | The tests stay Rust, beside the script. |
| L21 | The night bot builds the tree as fullsend: parts built blind onto `bot/issue-442`, then one reconcile. |
| L22 | Part 5's go/no-go gates the merge. |
| L25 | Meditative gets six batches (14–19). |
| L26, L27 | Part 28 (the API gaps) moves ahead of the last nine batches; those batches take the cards earlier batches held back; the contracts (CLAUDE.md, the READMEs, the spec, BUILD, REVIEW) are the reconcile's. |

## 3. How it was built

A fullsend tree: each part a sub-issue of #442, built by the night bot onto `bot/issue-442` without
seeing the others, then one reconcile that merges, makes the checks green and opens one pull request
into `main`. A batch could not touch `crates/luau`, `crates/engine`, `api-gaps.md` or another card's
files, so every gap a batch met is written down in section 4 or 5 and left for part 28 or the
reconcile.

| Part | Issue | What | Where it is |
|---|---|---|---|
| — | #558, #559, #560 | Groundwork: the host crate on `main` (#633), the proof tools on `main` (#615), `cargo jackioh luau surface` | `main`; #560 on the branch |
| 1, 2 | #443, #444 | Game records for analysis; the first notebooks (from an earlier, 23-part plan) | `main` (#501, #514) |
| 3 | #445 | The notebooks in the daily super run | a person's; not on the branch |
| 4 | #446 | The host wired into the cards; three pilots | branch |
| 5 | #447 | The browser on emscripten; the go/no-go | branch |
| 6 | #448 | The whole card API in Luau; the typed module | branch |
| 7 | #449 | The proof tools checked; the baseline; PORTING.md | branch |
| 8 | #450 | emsdk and `luau-analyze` in CI | a person's; not on the branch |
| 9–27 | #451–#463, #761–#766 | Batches 1–19: the cards | branch |
| 28 | #464 | The API gaps; three Classic ports | branch |
| 29 | #465 | The deploys after the merge | a person's, after the merge |

Parts 3 and 8 have commits (`266de4306`, `4a4f20ae6`) on `claude/confident-clarke-vtq5xq`; neither
is on `main` or on `bot/issue-442`.

## 4. The cards kept in Rust

Forty-one cards keep `pub fn script()` in their `.rs` (`grep -l "pub fn script()"
crates/cards/src/scripts/*/*.rs`). Fourteen are L14's. The other 27 a batch tried or judged and kept,
for one of PORTING.md §5's three reasons: the API cannot say it (a missing reader, verb or data
shape), `luau diff` proved a divergence the batch could not fix, or the card breaks `luau bench`.
A future port that keeps a card in Rust adds its row here.

| Id | Card | Kept by | Why |
|---|---|---|---|
| `core-093` | Combo-Index | L14 | Glue to `subsystems::combo_index` (its grades and end of turn). |
| `core-095` | Call to Chaos (Core Edition) | L14 | Glue to `subsystems::call_to_chaos` and its effect roll. |
| `core-096` | My Pawn | L14 | Glue to the turn the AI plays out (`subsystems::ai_policy::play_out_turn`, `is_lethal`, `defending_hero`). |
| `core-097` | Zephyrs | L14 | Glue to the scorer's `subsystems::top_three`. |
| `core-098` | Heroic Power | L14 | Glue to the start-of-game power roll (`start_of_game`, `subsystems::roll_power`, `power_abilities`), a hook the Luau loader refuses. |
| `core-099` | Craft a Card | L14 | Glue to `subsystems/fuse.rs` for the card its Discovers fuse. |
| `classic-057` | Echo | L14 | `records_play_as` (refused by the loader) and `subsystems::copied_text_of`. |
| `classic-090` | In Too Deep | L14 | `quests` (refused by the loader) and the quest subsystem (`open_quest`, `quest_reward_of`, `hold_quest_aura`, …). |
| `classicplus-042` | KY's Test | L14 | Glue to `subsystems::ky_test_script`. |
| `classicplus-044` | Simplicity Audit | L14 | Glue to `subsystems::audit_targets` and `lines_of_code`. |
| `classicplus-045` | Complexity Audit | L14 | Glue to `subsystems::lines_of_code`, as Simplicity Audit. |
| `classicplus-062` | KY's Papaya | L14 | Glue to `subsystems::papaya_begin` and `papaya_answered`. |
| `classicplus-073` | Call to Chaos (Classic+ Edition) | L14 | Glue to `subsystems::call_to_chaos` with this edition's table. |
| `classicplus-074` | Twice Forward One Step Backwards | L14 | Glue to `subsystems::twice_forward_plays` and `twice_forward_trigger`. |
| `meditative-095` | Call to Chaos (Meditative Edition) | batch 15 | Missing API: glue to `subsystems::call_to_chaos` with `CHAOS_MED_EFFECTS`, as the other two editions; no binding exists. |
| `meditative-042` | CN Flea Market | batch 18 | Missing API: the whole card is `subsystems::night_market_script` (the stall roll, the yuan, the `market` prompt's steps), unbound. |
| `meditative-071` | Pareto Optimality | batch 17 | Missing API: `subsystems::cane_strike` and `cane_return` on the remembered Cane, unbound. |
| `meditative-054` | Money Machine | batch 19 | Missing API: it rolls over `MONEY_MACHINE_ODDS` with `roll_weighted`, deliberately unbound; `J.config` holds `i32`s only, and writing the odds into the script would restate an engine constant (rule 9). |
| `meditative-087` | Tatches the Totem | batch 16 | Missing API: its deck trigger tests tags against the engine's `TRIBAL_TAGS`; `J.config` holds `i32`s only, and five literal tags would restate the constant. |
| `classic-082` | Sheeople | batch 9, batch 19 | Missing API (`crate::card_def`): `staticFlags.tributeWorth` is the catalog's `worth` (2, Radiant 3), read when the script is built; a face's `staticFlags` is load-time data with no context, and `J.query` at module level reads a registered catalog that is empty until `register_all()`. A literal would drift under a tune. The shared Death adapter it first waited on is fixed (gap 14). |
| `meditative-088` | The True Sheep | batch 18 | Missing API (`crate::card_def`): `staticFlags.tributeWorth` is the catalog's `worth` (5, Radiant 500), read when the script is built; only a literal could stand in. |
| `meditative-089` | Jlarna | batch 17 | Missing API (`crate::card_def`): its static flags carry the catalog's `creditLine` and `creditInstalments`; `J.cost`, `J.query` and `def_by_index` do not reach flag params. |
| `classicplus-t-ai-06` | Datacenter Fire | batch 9, batch 15 | Missing API: its `preview` needs `field_spells_doomed`, bound on the effect context only (`reads.rs`, outside `shared_readers!`), so gap 3 did not bring it to the condition context. An AI token: `luau diff` cannot force it. |
| `classic-014` | Shadowstep | batch 13 | Missing API: its trigger reads the whole §4.5 pass it answers from the action's full event list; `ctx:events()` (gap 6) answers only the script's own window, which opens after that pass. |
| `classicplus-012-8` | Frostspatula | batch 7, batch 11 | Missing API, as Shadowstep: its Death reads the whole action's events (`ctx.events`) for the kills made in the pass of its own death, which never reached its `destroyed` trigger, and `ctx:events()` (gap 6) opens when the hook runs. Batch 7 kept it for want of any events reader; batch 11 kept it and wrote no notes. The reconcile's reading of the Rust script and of gap 6; batch 11's builder journal (`461.md`, commit `dcd37d943`) agrees: the Death "needs pre-window action events", and two of its tests fail without them. |
| `classic-032` | Felinor Feelings | batch 13 | Missing API: its Radiant `choose_target_where` filters through a `where_` closure, which the Luau verb's serde skips, so it would offer every enemy permanent; the lanes must be read after the Cry's own summon, so a precomputed set will not do. |
| `classicplus-037` | Wardrum | batch 12 | Missing API: its end-of-turn casts go through `cast_new` with `CastNewDef::Read`, a definition read as each cast applies (R86); `J.cast_new` takes only a fixed `def`. |
| `meditative-027` | Clip-Farming Lawyer | batch 16 | Missing API: it asks whether any zone of either side and either row is Locked; `ctx:is_locked(player, lane)` reads the unit row only. |
| `meditative-098` | Showdown | batch 18 | Missing API: its Cry locks the zones of the other four lanes, both rows, that are not already Locked; no reader answers a backrow Lock. |
| `meditative-060` | Eschews | batch 16 | Missing API: its `tributeWhen` sums every `deaths@…` key of its memory (R102); Luau cannot list a record's keys (L7), and no reader answers them. |
| `meditative-065` | Keymaster Keenus | batch 16 | Missing API: its Radiant Death grants `keywords_of` the card as it died; the reader resolves the live instance (the reset graveyard card). A trial port failed 1 of its 9 tests. Needs a snapshot read, as gap 9 gave `killer_of`. |
| `meditative-094` | Shrinking Felinor | batch 16 | Missing API: its Death reads `base_stats_of` the card as it died (R1181); the reader resolves the graveyard copy, whose override is gone. A trial port failed 3 of its 8 tests. |
| `classic-069` | Plague Charger | batch 12 | Missing API: its aura raises its own attack only; the generic aura (gap 8) has no instance filter, and a named `aura_filter` gets the candidate alone, so a `defId` filter would lend the bonus to every other Charger. |
| `classic-042` | Transmutable Toxins | batch 13 | Missing API: one aura entry per Plague-carrying unit, each for that unit's id with its count; the generic aura cannot name one instance. |
| `classicplus-012-5` | Anti-Waffle Shell | batch 9, batch 19 | Missing API: its aura applies to the ids `active_units_of` answers, carried Units in the backrow included; the data filter's `rows` either misses them or takes every backrow card, and `aura_filter` has no state to tell them apart. |
| `classicplus-046` | Felinor Flagbearer | batch 9, batch 19 | Missing API: "your other Felinors" turns the source away and reads granted tags (`tags_of`, R923); the filter has no instance or tag field, and `aura_filter` gets the candidate alone. |
| `classicplus-046-1` | Felinor Flagbearer Prime | batch 9, batch 19 | The same "other Felinors" aura, on both faces. |
| `classic-036` | Burn | batch 9, batch 15 | A divergence: ported, then restored, on two `luau diff` games. A Radiant Burn fused into a base fusion ran its base `cry` (the host picked the face by `ctx.radiant`); and an Echo copying Burn read `effective_cost` off the live Echo (cost 1) where the Rust reads the copied text face (0), with no `effective_cost` over an instance table. The first is fixed by the reconcile (section 7); the card was not ported again in this run. |
| `classic-088` | Siphon Squad | batch 12 | Over the bench: a faithful port (0 mismatches, its tests green) took `fuzz --seeds 200` to ×2.08 of the parent's. Its `aura` cost about 148 µs a call, the most of any hook. |
| `core-014` | Jlockeed's Weapons | batch 8, batch 19 | Over the bench: first held for the aura verb gap 8 added; with it, ×1.75 alone and ×2.14 on top of batch 19's ports. |
| `core-065-1` | Spikey Pillow | batch 8, batch 19 | Over the bench, as Weapons: ×1.75 alone, ×2.04 on top of batch 19's ports. |

## 5. The record, part by part

Each part's numbers are against its own parent, on the box it ran on: a ratio is that part's change,
not the run's total.

### 5.1 Groundwork (#558, #559, #560)

Before the tree, three issues landed what every part used: the host crate on `main`, used by nothing
yet (#558, as #633); the proof tools on `main` (#559, as #615): `golden record`, `golden check
--file`, `luau diff` (200 forced games by §10.7's random policy, a token forced through the deckable
card whose `refs` name it, R279, replayed by the parent; any mismatch prints seed, step, which hash
and the action, and exits 1) and `luau bench` (`BENCH_BUDGETS` 1.5× / 2× / 2× and the absolute
`AI_GATE.max_decision_ms` cap); and `cargo jackioh luau surface` (#560), which read all 447 catalog
cards then, marked 14 to stay Rust and left none unreadable or unclassified.

### 5.2 Part 4 (#446): the host in the cards, and three pilots

- `mlua = "=0.12.2"`, no default features, `luau` and `serde`; Luau 0.740 from `luau0-src
  0.22.0+luau740`. `jackioh-luau` is a dependency and a build-dependency of `jackioh-cards`.
- The sandbox keeps `string`, `table`, `math`, `bit32` and `utf8`. Scripts compile at
  `LUAU_OPTIMIZATION_LEVEL` 1 with `sandbox::DISABLED_BUILTINS`, so a fastcall cannot reach a
  removed global. Numbers cross through `numbers.rs` as `i32`s (L6).
- `crates/cards/build.rs` enforces one script source per card (a `.luau` or a Rust `pub fn
  script()`), lints and compiles each `.luau` to `$OUT_DIR/<set>_<module>.bin`, and the generated
  registry loads it with `jackioh_luau::load_card(ID, BYTECODE)`.
- The pilots: Core #1 Big D-Fender (an `aura` of `defense_armor_aura`, `ctx:param("armor")`,
  `radiant = "base"`), #2 Bigot (declared `targets` with `notTags`; `destroy`, Radiant
  `destroy_all`) and #3 Right-House Defender (`base = {}`, a Radiant `death` summon). Their 18
  tests passed unchanged.
- The server's Docker build stage gained `g++` (Luau compiles from source); `scripts/vercel-ignore.sh`
  builds on `.luau` files and `crates/luau` changes.
- Measured: `fuzz --seeds 200` clean in 21 s; `gate` 20/20, 16/20, 18/20; `golden check` 240 of 240
  (22,713 steps); `spec check` ok.
- Left behind: `sh scripts/build-wasm.sh` red on the old target (part 5's fix), and two `luau
  surface` tests that read Bigot's Rust `script()` (section 6).

### 5.3 Part 5 (#447): the browser, and the go/no-go

- `crates/wasm` is a binary (`src/main.rs`): `rustc` emits emscripten's JavaScript glue only for a
  program link, and a `cdylib` through `emcc` is a bare `.wasm`. The bindings are
  `#[unsafe(no_mangle)] extern "C"` functions over NUL-terminated UTF-8 strings with the old names
  and JSON, plus `jackioh_free` (answers and spent arguments share emscripten's heap) and
  `jackioh_last_error` (a failure answers NULL). `wasm-bindgen`, `js-sys` and
  `console_error_panic_hook` left the workspace. `loadWasmSync` is async now (emscripten has no
  synchronous instantiate).
- The link (`.cargo/config.toml`): `-O3`, since emcc's default `-O0` ran the golden step at 2.5×
  `main`; `-sINITIAL_MEMORY=64MB`, since `register_all`'s Luau init outgrew the 16 MB default
  (162–213 ms, over budget); `-sSTACK_SIZE=1MB`, parity with the old module. emsdk 5.0.7 is pinned
  in `scripts/build-wasm.sh`; its clang 23.0.0 is one major ahead of Rust 1.97.0's LLVM 22.1.6, and
  the link holds because thin LTO stays inside `rustc`.
- The go/no-go, against `main` at the branch point `f01a13d5d` (this tree cannot build the old
  `wasm32-unknown-unknown` module, since `mlua` does not compile there, so the delta covers Luau and
  the emscripten runtime together):

  | Number | `main` | Branch (3 pilots in Luau) | Budget | Verdict |
  |---|---|---|---|---|
  | Module + glue, `gzip -9` | 2,945,562 B | 3,772,347 B (+0.79 MiB) | ≤ 4,045,338 B | GO |
  | Golden step, median of 40 | 5.49 ms quiet, 10.01 ms loaded | 8.97 and 9.54 ms | ≤ 2× | GO (about 1.6×) |
  | `register_all` | 50–167 ms | 92–187 ms over 9 runs | < 200 ms | GO |
  | e2e smoke (01, 06, 10, 13, 19) | | not run: no Chrome on the box | green in CI | the reconcile's |

  The golden step is this part's own probe (one practice step through the wrapper, over 40 states
  of seeded games, in vitest); the #306 script was gone. Luau's compiler stays in the module: `mlua`
  has no bytecode-only build, and the size passed with about 222 KB to spare with three cards'
  bytecode in it. Rejected: a `TextEncoder.encodeInto` input path (the gate passed at 1.6×) and
  `-Os` (the speed gate needed `-O3`).

### 5.4 Part 6 (#448): the whole card API

- All 176 verbs bind (`crates/luau/src/api/`, one file per `effects/` module), the two `luau
  surface` leaves unclassified (`replace_hand_with_perfect`, `roll_back`) included. The readers are
  methods of a scoped `ctx` userdata (`reads.rs`), plus `J.query`, `J.pool`, `J.cost`, `J.div`,
  `J.rem`, `J.sort_by` and `J.TRAP_TYPES`. 42 of 45 hook types have adapters (`hooks.rs`,
  `script.rs`); `quests`, `records_play_as` and `start_of_game` are L14's and refused at load
  (`start_of_game` had been a key that loaded and never fired).
- The typed module (`crates/cards/luau/jackioh.luau`) is generated by
  `crates/luau/tests/export_api.rs` from the engine's types (an off-by-default `ts` feature, no
  behaviour change) and committed; `crates/cards/src/scripts/.luaurc` aliases `@jackioh` for editors
  and the checker. `crates/luau/tests/api.rs` holds one test per hook family and Luau-vs-Rust parity
  per verb group (equal `hash_state` and events).
- **Closure verbs resolve by name, with no registry.** The first `after_state_check` kept script
  callbacks in a thread-local map behind an atomic counter, alive across calls (against L8) and
  impure (rule 4). A script now passes a hook name and data (`J.after_state_check({ hook =
  "after_check", data = … })`), and the closure re-runs the chunk to call `module[face][hook]`.
  `draw_while` and `for_each_card` do the same (`draw_more`, `each_cards`, `each_card`).
  `vm::SUB_HOOKS` allowlists those names, with a `_<n>` suffix for a second use in one face; any
  other face key is still a load error.
- The host threads each card's `&'static` bytecode to the nested builders that resolve named
  sub-hooks. Rejected: a card-to-bytecode map (a growing static) and a thread-local "current card".
- `with_kill_credit`'s `pairs` and `then` stayed unbound (their dead callback-id fields removed);
  part 28 bound them. `crate::card_def` stayed a gap, judged per card by the batches.
- `api-gaps.md` is this part's: the surface minus what binds, per name, with the cards that wait.

### 5.5 Part 7 (#449): the proof tools, the baseline and the porting guide

- #615 had already built the commands; this part checked them against the plan, added
  `luau_diff_with_the_pilot_cards_forced_reports_0_mismatches` (`crates/tools/tests/luau.rs`), wrote
  PORTING.md against part 6's landed API (the pilots' pre-port Rust quoted from `8d29cd5e7^`) and
  took the baseline.
- **The gate compares two binaries live; `perf.json` is a witness.** The plan said to compare
  against `perf.json`; `luau bench` measures parent and current on one machine in one run, which a
  committed file cannot. Rejected: a `--baseline perf.json` mode.
- The baseline, 2026-10-13, on linux x86_64, 2 threads, Xeon Platinum 8488C, `main` at `f01a13d5d`
  (all cards Rust) as the parent:

  | Number | Parent | This branch | Ratio | Budget |
  |---|---|---|---|---|
  | `fuzz --seeds 200` wall time | 20,094 ms | 21,638 ms | ×1.08 | ≤ 2× |
  | Slowest `gate` decision (scaled) | 306.5 ms | 227.5 ms | ×0.74 | ≤ 1.5×, < 1,500 ms |
  | Golden step (`RAYON_NUM_THREADS=1`) | 1.708 ms | 1.766 ms | ×1.03 | ≤ 2× |

  `luau diff` with the three pilots forced: 200 games, 21,181 steps, 0 mismatches.

### 5.6 Part 28 (#464): the API gaps

Re-scoped on 2026-10-13 (L27) to land before the last nine batches, so they port onto the fixed API.
Gaps 1–13 were built on the `bot/issue-464` work in progress; this part finished 14 and 15, ported
Classic #26 Rapid Draw, #30 Recycle and #35 Prep, and rewrote `api-gaps.md` (its "Fixed in part 28"
section gives each gap its Luau form) and PORTING.md §5. The gaps: (1) empty tables round-trip as
lists; (2) `for_each_card` passes `data`; (3) one reader table on every context; (4) `ctx:turn()`;
(5) `given_lucky_on`; (6) `ctx:events()`, the script's window; (7) `J.config`; (8) a generic aura;
(9) `left_field_since_resolved`, a self-snapshot `killer_of`, `chancePercent`, kill credit's
`pairs` and `then`; (10) one adapter for one function under two hooks; (11) the typed-module check
ungated; (12) `None` is `nil`; (13) `ctx:data()`; (14) one adapter across both faces (the cache
moved into `load_card`); (15) `.luaurc` maps `jackioh` with no extension, because luau-analyze
0.740 refuses the extension as `Unknown require`, which had silenced every earlier strict run
(777 cascading errors).

A strict run over its three ports (Luau 0.740's release binary) caught three generator bugs in
`export_api.rs`: a swallowed newline that commented out `ConditionContext`, four types named but
never emitted, and `PlayerModifierSpec` as an intersection strict checks no literal against.
Rapid Draw's work-in-progress port named sub-hooks outside `vm::SUB_HOOKS`, so `register_all()`
panicked every card test until they were renamed. Its notes left the crate tests, `luau diff`,
clippy, `spec check`, `golden check`, fuzz and bench as PENDING. The batches built on it
(`5e292a8fb`) ran the crate tests, `golden check` and fuzz green on that base; no `luau diff` of its
three ports is recorded.

### 5.7 The batches (parts 9–27)

Each batch ran PORTING.md §1: a parent built from its base, each card's `.luau` written and its
`script()` removed together, the card's tests, `luau diff` (in two or three runs where the forced
cards outgrew a 20-card deck), `golden check`, `fuzz --seeds 200`, `luau bench`. Ported counts
include held-back cards a batch took (L27). Bench is fuzz wall time / slowest decision / golden step.

| Part | Issue | Batch | Scope | Ported | Kept in Rust | `luau diff` | Bench |
|---|---|---|---|---|---|---|---|
| 9 | #451 | 1 | Core Cry, first half | 20 of 22 | core-035, core-042 | 0 (11 + 11 cards) | ×1.00 / ×1.03 / ×1.00 |
| 10 | #452 | 2 | Core Cry, second half; Core tokens | 20 of 22 | core-077, core-083 | 0 | ×1.02 / ×1.01 / ×1.00 |
| 11 | #453 | 3 | Classic Cry; Glitch | 20 of 25 | classic-026, -030, -035, -039, -079 | 0 | ×0.97 / ×1.01 / ×0.98 |
| 12 | #454 | 4 | Classic+ Cry, first half | 24 of 26 | classicplus-019, -023 | 0 | ×1.01 / ×0.99 / ×1.01 |
| 13 | #455 | 5 | Classic+ Cry, second half; AI tokens | 25 of 26 | classicplus-t-ai-04 | 0 | ×1.02 / ×1.04 / ×1.00 |
| 14 | #456 | 6 | Start and end of turn, Death | 20 of 25 | core-023, core-086, classicplus-019-2, -019-4, -070 | 0 | ×1.01 / ×1.01 / ×1.01 |
| 15 | #457 | 7 | Triggers | 12 of 18 | core-041, classic-010, -065, classicplus-002, -012-8, -t-ai-08 | 0 (two runs) | ×1.07 / ×0.97 / ×1.03 |
| 16 | #458 | 8 | Auras, flags, costs, conditions, previews (Core, Classic) | 18 of 29 | core-010, -014, -031, -046, -058, -060, -065-1, -070, -085, -100, core-t-sheep | 0 (three runs) | ×1.00 / ×0.97 / ×1.02 |
| 17 | #459 | 9 | The same, Classic and Classic+ | 15 of 29 | classic-022, -036, -043, -059, -082; classicplus-012-5, -012-6, -018, -046, -046-1, -047, -050, -064, -t-ai-06 | 0 | ×0.99 / ×1.00 / ×1.00 |
| 18 | #460 | 10 | Prompts, modes, continuations (Core) | 13 of 16 | core-050, -051, -078 | 0 (two runs) | ×0.98 / ×1.02 / ×1.01 |
| 19 | #461 | 11 | The same (one Core card, Classic, Classic+); 12 held back | 26 of 27 | classicplus-012-8 | not clean (section 5.8) | not recorded |
| 20 | #462 | 12 | The rare hooks; 3 held back | 26 of 29 | classic-069, -088, classicplus-037 | 0 (three runs) | ×1.35 / ×1.01 / ×1.27 |
| 21 | #463 | 13 | Activate, replacements, target checks; 5 held back | 24 of 27 | classic-014, -032, -042 | 0 (three runs) | ×1.13 / ×0.88 / ×1.10 |
| 22 | #761 | 14 | Meditative Cry, first half; data-only cards; 3 held back | 27 of 27 | none | 0 (two runs) | ×1.00 / ×1.01 / ×0.98 |
| 23 | #762 | 15 | Meditative Cry, second half; 7 held back | 23 of 26 | classic-036, classicplus-t-ai-06, meditative-095 | 0 (two runs) | ×1.03 / ×1.03 / ×1.11, by hand |
| 24 | #763 | 16 | Meditative turn, Death and trigger hooks | 24 of 29 | meditative-027, -060, -065, -087, -094 | 0 (two runs) | ×1.06 / ×0.98 / ×1.02 |
| 25 | #764 | 17 | Meditative flags, costs, attack mods, conditions | 27 of 29 | meditative-071, -089 | 0 (two runs) | ×1.02 / ×0.99 / not measured, by hand |
| 26 | #765 | 18 | Meditative prompts, modes, continuations, rare hooks; 8 held back | 21 of 25 | meditative-022, -042, -088, -098 | 0 (two runs) | ×1.04 / ×1.04 / ×1.06 |
| 27 | #766 | 19 | Meditative Activate, replacements, target checks; 10 held back | 17 of 24 | classic-082; classicplus-012-5, -046, -046-1; core-014, -065-1; meditative-054 | 0 (three runs) | ×1.18 / ×0.39 / ×1.14 |

Every batch left each ported card's assertions as they were: the edits are `script()` read from the
registry, imports the removed `use jackioh_engine::prelude::*` had brought in, and test-only
constants moved into `mod tests`. Every batch that recorded them saw `golden check` 240 of 240 and
`fuzz --seeds 200` clean, except where section 5.8 says otherwise.

Where the held-back cards went: batch 13 took core-023, -035, -042, -077 and -083; batch 11 took
classic-010, -022, -039, -065, -079, classicplus-002, -023, -t-ai-08 and core-050, -051, -078;
batch 12 took core-041, -086 and classicplus-019-2; batch 14 classicplus-019, -019-4 and -t-ai-04;
batch 15 classic-043, -059 and classicplus-012-6, -018, -050; batch 18 core-010, -031, -060, -070,
-085, -100, core-t-sheep and classicplus-064; batch 19 core-046, -058 and classicplus-047, -070.
Part 28 took classic-026, -030 and -035. The rest are in section 4.

### 5.8 What each batch decided

- **Batch 1.** Empty faces spell `radiant = "base"`; identical `targets` are one module-level
  table (read-only declaration data, so L8 holds). Strict analysis on staged copies with a relative
  `require`: clean but for five `ImportUnused` warnings on the empty cards.
- **Batch 2.** Genn's Greed reads its ids in the Cry and passes them outright (`cards = ids`),
  since an empty answer from a `cards` function panicked. A reader's `None` arrived as mlua's
  truthy `null`, found by the first `luau diff` (Zoomerbin Oomen's Cry with its unit gone). Faces
  mirror the Rust's sharing, because tests assert `Arc::ptr_eq` (or its negation).
- **Batch 3.** As a check that forced cards are played, a Book of Knowledge drawing one too many
  diverged in 14 of 20 games. Glitch cannot be forced (it reaches a hand only through R673's roll);
  its tests and the golden traces prove it.
- **Batch 4.** Mid Loser and Soul Shot write `Rng::lucky_coin` and `Rng::lucky` out draw for draw
  (`ctx:rng_chance(1, 2)`, `ctx:rng_pick`). The Pancake tokens are forced through The Mother
  Pancake. Two patterns make a possibly empty `for_each_card` equal to the Rust without gap 1:
  ids read in the hook when the clause is the list's only effect, or `cards = {}` passed only in a
  case the hook can see is empty.
- **Batch 5.** J-Jungle EX-plorer names its pack's id directly (its `card_def(...).id` is the id),
  a cover `api-gaps.md` foresaw; optional `radiant` flags are `if radiant then true else nil`, since
  `false` crosses as a value. Strict analysis on staged copies: clean.
- **Batch 6.** Tests read the registry, not a `#[cfg(test)] fn script()` shim beside `ID`, which
  would put a script back in a Luau card's file (batches 14 and 18 declare a private `fn script()`
  reading the registry inside `mod tests` where assertions call it inline). "Miss" Mrow's trial
  port on `ctx:killer_of` failed 4 of 10 tests (the live instance in a Death hook is the reset
  graveyard card): gap 9's self-snapshot.
- **Batch 7.** Doom Shroom and Refusal reorder two reads where both are pure and nothing between
  them draws or writes. Its twelve cards played 64 to 437 times each in the 200 games, firing 5 to
  1,518 times (counted from `golden record` by a script outside the repository, as later batches
  did).
- **Batch 8.** No aura ported, so no aura was benched. Condition contexts bound too few readers
  (gap 3).
- **Batch 9.** Second Amendment Snake and Book Worm read their counters off the hook's snapshot
  (`ctx:self_()`), since `plague_on` reads the live (reset) instance in a Death hook. Ivory Tower
  checks its slot is in the backrow before `carried_at`.
- **Batch 10.** `resume` keys, steps, prompt texts and options are the Rust ones, because paused
  states and replayed logs name them. Recycling Initiative still stores `data.selfId`, which no
  Luau reader read back then, so the delayed entry in state stays byte for byte the Rust's.
- **Batch 11.** No notes. Built in two passes (the first stopped at a usage limit before any card;
  the second, `03771fb3a`, ported 26 of 27). From the commit and the builder's journal: the
  resumes of Mind Melt, Back from the GY and Spell Tyrant probe their picks through
  `instance_of({ of = "chosen", index })`; Ace in the Hole's Lucky coin is `1 + x`
  `rng_chance(1, 2)` draws; Glitch in the System names its 0–10 bounds locally. Its `luau diff`
  mismatched in about 156 of 200 games, every one a Felinor Fuser play, and the batch landed before
  the builder found why. Batch 19 found it: the Fuser's first Discover stored
  `data = { picks = {} }`, and an empty table in untyped data crossed as `{}` where the Rust stored
  `[]`. On every base that held it, `golden check` diverged in 14 of 240 games until batch 19
  restored the Fuser's Rust; batches 15 and 17 showed their diverged seed sets equal to their
  parents' and timed the bench by hand, since `luau bench` stops when the parent's golden replay
  diverges. The reconcile re-ported the Fuser with `J.list` (section 7). No clean `luau diff` of
  the batch's 26 cards was recorded; the reconcile runs it.
- **Batch 12.** Siphon Squad over the bench (section 4); a throwaway timer put Core #1's `aura`,
  already Luau, at about 113 µs a call, so the cost is mostly the call itself. Plague Chalice's
  upvalue-free `cry` is one closure for both faces in Luau, so the faces share one adapter where
  the Rust built two; no test reads it.
- **Batch 13.** InfiniScepter reads its held Spell in the hook (a `CastNewDef::Read` closure has no
  Luau form; the hook's list is that one effect). `ChosenTuningNumber` crosses with its Rust field
  names (`instance_id`), not camelCase. The `J.pool` stub typed one argument where the binding
  takes two, so Transmogulate called it through `any`; the reconcile fixed the stub (section 7).
- **Batch 14.** The Cane is catalog data (its Rust was two empty scripts; the forced attack is
  Pareto Optimality's), so it ported. CN Peptides writes `Rng::lucky` out; Prime Time parses an
  index as Rust's `parse::<u32>()` does, not with `tonumber`.
- **Batch 15.** Burn's two divergences came from `luau diff` seeds 70 and 63. Condition-context
  readers the typed stub does not declare (they are bound at runtime by gap 3) are called through
  `any`.
- **Batch 16.** Catnip reads the resolved face's type off `ctx:def_of(event.defId)`, since
  `ctx:card_type_of_face` asks the running card's own face. Keenus's and Shrinking Felinor's trial
  ports named the failing tests, then were reverted.
- **Batch 17.** Its oracle was `03771fb3a`, not the nominal base `5e292a8fb`: against the base,
  batch 11's Classic ports showed 3 mismatches in run A. Feng Shui's `luck = 1` stays a literal,
  mirroring the Rust's `const LUCK` (a Buff must not raise it, R987).
- **Batch 18.** Seed 91 is left out of one `luau diff` run (seeds 1–90 and 92–201): with that forced
  list its game never ends on either build. A logging parent build showed M #97.6 University
  activating `study` 25 times on one turn though its `uses` is 1; a build of `e374658dc`, before
  batch 19 ported University, stalls the same, so the stall is the card's or the engine's, not a
  port's. The face bug that kept Mind Games was latent in six ported cards of the batch (CN in a
  bottle, both Knowledge Breakers, Do or Die, Meditative Journey, Disruptive Disruptor); none
  differed in its 400 games, so they stayed ported.
- **Batch 19.** First restored Felinor Fuser's Rust (`5e292a8`'s file) and removed `/fullsend`'s
  unused import, which brought `golden check` back to 240 of 240 and clippy clean. With all 19 of
  its cards first ported the bench was over: fuzz ×2.56. The auras ran their chunk inside every
  stat read of every unit (L8), a named predicate adding a call and a serialised `CardInstance` per
  candidate. Timed one aura at a time on the 15 other ports (×1.05): Weapons ×1.75, Suppressive
  Aura ×1.32, Rush Token Farm ×1.07, Spikey Pillow ×1.75; the cheapest two together ×1.28, either
  of the others on top over 2×. The auras that name no zone use named predicates, one per
  controller (`aura_filter` for p1, `aura_filter_2` for p2), since a data filter matches field rows
  only. Its notes' workspace-test section was left blank.

## 6. What the batches found

- **Empty tables.** An empty Luau table crossed into Rust as the record `{}`, so a `cards` function
  with nothing to answer panicked (batches 2, 3, 4, 9, 10) and untyped data stored `{}` where the
  Rust stored `[]` (batch 11). Gap 1 fixed typed arguments and `cards` answers; untyped values
  (a `remember`'s value, a prompt's `data`) take `J.list(t)`, the reconcile's (section 7).
- **An absent answer.** Until gap 12 a reader's `None` reached Luau as mlua's `null`, a truthy
  userdata, so batches 2–10 test `type(x) == "table"`; from gap 12 on it is `nil`, and both
  spellings are right.
- **Hook identity.** Card tests assert `Arc::ptr_eq` between faces or hooks. `radiant = "base"`
  clones base's `Script`, sharing every hook; one function value named twice shares one adapter
  (gaps 10 and 14). Luau makes one closure of an upvalue-free literal however often it is
  evaluated, so faces that must differ (`!ptr_eq`: The Coin, Classic Golem) spell separate closures.
- **The face a shared adapter runs.** Until the reconcile, every direct hook ran the function of
  the face `ctx.radiant` named, which differs from the adapter's own face in one case: a Radiant
  ingredient fused into a base fusion (R469). It kept Burn and Mind Games in Rust and was latent in
  every two-face card.
- **Death hooks read the dead card.** Readers that take an id resolve the live instance, which in a
  Death hook is the reset graveyard card (R78). Gap 9 gave `killer_of` a snapshot read; Keenus and
  Shrinking Felinor wait for the same on `keywords_of` and `base_stats_of`, and the ports that
  could read their own snapshot (`ctx:self_()`) did.
- **Auras in Luau are slow.** An aura runs on every stat read: about 113–148 µs a call (batch 12),
  which batch 12 put down to L8's re-run of the chunk. Siphon Squad, Jlockeed's Weapons and Spikey
  Pillow broke the bench; a cheaper aura call in `crates/luau` would let them port. The reconcile
  found that, once the readers were registered once per VM, most of what was left was mlua's
  protected calls under the memory cap, not the re-run, and cut it (section 7); the three were not
  tried again.
- **`luau diff`'s reach.** A deck holds 20 forced cards, so big batches split their runs. A token is
  forced through the card whose `refs` name it; one that nothing names (Glitch, the AI tokens,
  Anti-Waffle Shell, AI Girlfriend, the last two Growing Felinors) is proved by its own tests and by
  the cards that bring it into play.
- **Strict analysis barely ran.** Only batches 1 and 5 (on staged copies) and part 28 recorded a
  `luau-analyze --mode=strict` run; the other boxes mostly had no toolchain and no network to fetch
  Luau 0.740's release, and `luau0-src` ships no Analysis sources to build it from. Until part 28's
  gap 15 an in-place run failed on the `.luaurc` alias. Batches disagreed on an unused `require` in
  a file that calls nothing (an `ImportUnused` warning); the reconcile removed them.
- **Tests the batches could not fix.** From part 7 on, the `jackioh-tools` runs failed
  `luau::surface::tests::luau_core_002_bigot` and `luau_reports_print_as_markdown_and_json`
  (`luau surface` reads `script()` from `.rs` files, and Bigot has been Luau since part 4), and from
  part 10 on, the first to run it, `acceptance.rs`'s
  `v2_the_pure_crates_link_only_serde_serde_json_indexmap_and_each_other` (`crates/cards` depends on
  `jackioh-luau` since part 4). On part 28's build `cargo fmt --check` failed on one `assert!` in
  `crates/luau/tests/api.rs`, which the reconcile reformatted.
- **Gaps outside `api-gaps.md`.** The batches could not edit it, so the gaps they met after part 28
  are in section 4's table: an events reader before a script's window, a named `where` for
  `choose_target_where`, a by-name `cast_new` definition, an instance or tag filter (or data) for
  auras, a backrow `is_locked`, snapshot reads, memory keys, non-`i32` engine constants and weighted
  odds in `J.config`, `field_spells_doomed` on condition contexts and a catalog def at load time.

## 7. The reconcile

What the reconcile changed, beyond merging the parts:

- **`main` merged in** (`252e21770`, `8e23efd60`, `d98ab0bf6`). It brought #857, a memory bound for
  the Luau sandbox, and v0.3.7's card patches (#823), ported into the `.luau` files: Classic #5
  Tesla through a new reader, `ctx:immutable_bars`; #6 Cloaked Toe Cracker reading the played card
  as it stands (`ctx:card_type_of`); Classic+ #1 Doom Shroom without its `lock_own_zone` (its Crater
  Locks the zone); Meditative #45 Knowledge Breaker and #45.1 Prime through a new aura verb,
  `J.cloak_hand_aura`; and #99 Paranoia through that verb and the new `revealEcho` hook, which
  replaces `faceDownPlay`, gone from the engine.
- **`J.list(t)`** (`crates/luau/src/api/mod.rs`) marks a table as a list with mlua's array
  metatable, so an empty one crosses as `[]`. Felinor Fuser (`classicplus-030`) is Luau again on it,
  and Risky Die's `remember` uses it.
- **The face fix** (`crates/luau/src/script.rs`, `shared_functions`): only an adapter both faces
  share picks its face by `ctx.radiant`; any other runs its own face's function, as a Rust face
  runs its own closure whatever the context says. It removes the divergence that kept Burn and
  Mind Games in Rust. Mind Games (`meditative-022`) is Luau again, batch 18's port on the fixed
  host: its tests pass and `luau diff` finds no mismatch. Burn was not ported again.
- **Exile** (`classic-010`): its Luau `run` now checks the play again, as the Rust closure did.
  `when` is read for a Trap alone (R99), so with Exile fused into a card kept as a Field Spell the
  port countered its own controller's plays (`luau diff` seed 149). The other 27 ports with a `when`
  keep the guards their Rust `run` had.
- **Speed.** The readers are registered once per VM (`reads.rs`), not on every call; the test
  profile builds `mlua-sys` and `mlua` at opt-level 2, since at 0 every card hook a test ran went
  through an unoptimised Luau interpreter. Timed by hook, `fuzz --seeds 200` spent most of its Luau
  time in `aura` calls (about 350,000 a thread, 4.5–14 µs each) and about half of every call
  in mlua's protected calls: with a memory cap set (#857), mlua builds each value the host hands
  Luau under its own `pcall`. A hook call still runs under the cap (`vm::BudgetGuard`), but a
  reader, `J.query` and `J.pool` lift it while they build their answer (`sandbox::MemoryCap`), and
  `J`'s verbs are Luau closures rather than Rust functions. That took the fuzz from 11.98 s to
  10.2 s on a box where `main` takes 6.6 s. Building each module once per VM instead of on every
  call (a change to L8) saved under 0.3 s, so L8 stands.
- **Smaller fixes.** `jackioh-luau`'s `ts` feature is optional and off by default; `cargo jackioh
  catalog loc` counts a ported card's `.luau` (`count_luau_loc`); the workspace version is 0.4.0;
  for strict analysis, Curse of the Forgotten Classic hoists its `J.TargetSpec` into an annotated
  local and the files that call nothing drop their `require`; `crates/luau/tests/api.rs` is
  formatted.
- **The typed module** (`export_api.rs`, then `jackioh.luau` regenerated): `ActivationUses` is
  `number | "unlimited"` (ts-rs read the `"unlimited"` variant as `nil`), with `uses` optional on a
  declaration as the loader reads it; `J.pool` takes the excluded id and the query, as its binding
  does, so Transmogulate calls it directly; and `TargetCheckArgs`, a `targetChecks` predicate's
  argument, is exported with its readers (`tests/api.rs` holds them to the binding).
- **The crate edges.** `jackioh-cards` dropped an `mlua` build-dependency its `build.rs` never used
  (it compiles through `jackioh_luau::compile`), and `crates/tools/tests/acceptance.rs`'s V2 holds
  rule 4's new list: the four pure crates, `jackioh-luau` alone taking `mlua`, the AI never
  `jackioh-luau`, `ts-rs` optional in the engine and the host.
- **The contracts** (L27): CLAUDE.md rules 4–6, `crates/cards/README.md`, `crates/luau/README.md`,
  `docs/ADDING_CARDS.md`, SPEC §10.9, BUILD, REVIEW and `docs/architecture.md` describe the cards as
  Luau scripts with Rust tests, and the cards kept in Rust as this file's section 4.
- **The parts' notes** are folded into this file and deleted.
- **The final numbers**, on the reconcile's box (4 threads, AMD EPYC 9V45), against `main` at
  `e20df15ea`, the last `main` merged in, built on the same machine:
  - `luau bench` (L13): `fuzz --seeds 200` 6,388 → 9,767 ms (×1.53, budget 2×); the slowest gate
    decision 205 → 126 ms (×0.62, budget 1.5×); a golden step 1.151 → 1.579 ms (×1.37, budget 2×).
  - `luau diff` against that `main`, where every card is Rust: the 341 ported cards that can be
    forced into a deck (every port but the 27 tokens no deckable card's `refs` name), 20 to a run,
    200 games each, 0 mismatches. Seed 149 found the Exile divergence above before its fix.
  - `fuzz --seeds 1000` and `fuzz --seeds 1000 --handicap`: clean. `golden check`: 240 of 240.
    The workspace's tests pass, the card tests (5,883 unit, 752 cross-card) among them.
  - `gate --full` gives `main`'s counts: 98 of 100 against random, 32 of 50 against greedy, 46 of
    50 hard against easy, and 170 perf decisions, the wide ones at the same node counts. Only which
    decision is the slowest by the clock differs; the slowest, scaled as `gate` reports it, is
    352 ms (`main` 480 ms), under `AI_GATE.max_decision_ms` (1,500).
  - The browser (L12), on the whole branch, against the module `main` builds at `e20df15ea`, both
    built on one box (4 threads, AMD EPYC 9V45), with part 5's probes (section 5.3):
    - module and glue, `gzip -9`: 2,905,567 → 3,933,262 B (+1,027,695 B, 0.98 MiB; budget 1 MiB).
      `apps/web/src/wasm/size.test.ts` holds it to part 5's anchor, 2,945,562 + 1 MiB, wherever the
      web's tests build the module, CI's `web` job among them;
    - a golden step through the wrapper (`chooseAction`, `reduce`, `viewFor` twice, `legalActions`,
      `seatToAct`, `aiToAct`, in vitest), median of 406 steps of seeded `cheap20` games after 5
      warm-up steps, two runs a side: 2.40 and 2.39 ms → 4.02 and 3.91 ms (×1.65, budget 2×);
    - `register_all` (the module's `init()` in node, five runs): 115–145 ms (budget 200 ms;
      `main`'s 28–33 ms).
  - `luau-analyze --mode=strict` did not run: the box had no Luau 0.740 release. Part 8's CI step
    runs it. Its first run found 31 errors, all fixed in #872: 18 were readers the context types
    left out, and 13 were annotations in the scripts.

## 8. v0.5.0: official cards back in Rust

[#902](https://github.com/Jlore-Studios/JackiOh/issues/902) undid the port for official cards and
kept the host for custom cards ([#915](https://github.com/Jlore-Studios/JackiOh/issues/915)).

- **Why.** The game played the same, but everything it touched got slower or bigger, and official
  cards gain nothing from a VM (the reconcile's numbers, section 7, against `main` on one box):

  | | `main` | v0.4.0 |
  |---|---|---|
  | `fuzz --seeds 200` | 6,388 ms | 9,767 ms (×1.53) |
  | A golden step (server, tools) | 1.151 ms | 1.579 ms (×1.37) |
  | A golden step in the browser | 2.40 ms | 4.02 ms (×1.65) |
  | The browser module, gzipped | 2,905,567 B | 3,933,262 B (+0.98 MiB) |
  | `register_all` in the browser | 28–33 ms | 115–145 ms |

  The AI pays this many times over, since every simulated move runs card hooks. The VM is only
  needed for code that is not compiled into the game: custom cards.
- **What came back.** Each of the 409 ported cards' `.rs` came back from `d787d2a8` (`main` just
  before v0.4.0 merged), with `pub fn script()` and its tests, and its `.luau` was deleted; since
  v0.4.0 no card script changed but #876's type annotations, so nothing needed porting. Call to
  Chaos reads its golem's `script()` again. `crates/cards` builds without `jackioh-luau`
  (`Cargo.toml` and `build.rs` from `d787d2a8`), and `cargo tree -i mlua` finds `mlua` under
  `jackioh-luau` alone. The browser module is `wasm32-unknown-unknown` with wasm-bindgen again
  (`crates/wasm`, `scripts/build-wasm.sh`, `rust-toolchain.toml`, `.cargo/config.toml` and the web's
  wrapper from `d787d2a8`), and the contracts say official cards are Rust (CLAUDE.md rules 4–6,
  `crates/cards/README.md`, `docs/ADDING_CARDS.md`, SPEC §10.9, BUILD, REVIEW,
  `docs/architecture.md`). The workspace is at 0.5.0.
- **What stayed.** `crates/luau` (the sandbox, the integer boundary, the lint, the VM, the `J`
  bindings and `load_card`), built and tested as a workspace member; `crates/cards/luau/jackioh.luau`
  and its generator, which CI still checks; the verbs and readers v0.4.0 added to the engine; the
  `cargo jackioh luau` tools; and the notebooks.
- **The proof**, on a box with 4 threads (Intel Xeon @ 2.80 GHz), against `main` at `d2623ca4`
  built on the same box:
  - The card tests: 5,884 unit and 754 cross-card, all passing.
  - `luau diff` with `main` as the parent: the 382 restored cards that can be forced into a deck
    (all but the 27 tokens no deckable card's `refs` name), 20 to a run, 200 games each: 0
    mismatches, and every run's `golden check` 200 of 200.
  - `luau bench`: `fuzz --seeds 200` 20,685 → 11,234 ms (×0.54); a golden step 4.340 → 2.865 ms
    (×0.66). The slowest gate decision read 50 → 146 ms: one clock reading of whichever decision
    ran slowest, as section 7 found, and far under `AI_GATE.max_decision_ms` (1,500).
  - The browser module and its glue, `gzip -9`: 2,921,195 B (v0.4.0's 3,933,262 B; the module
    before it 2,905,567 B). `register_all` (the module's `init()` in node, five runs on this box,
    which runs `main`'s fuzz about twice as slow as section 7's): 53–78 ms.
