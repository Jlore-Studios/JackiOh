# v0.3.0 Surface: the interface freeze

This is the fullsend `SPEC.md` for the v0.3.0 rewrite (#306). Every part's builder reads it before
writing a line and matches it **exactly** at every boundary: crate names, module paths, type names,
function names, argument order, JSON shapes, routes, CLI flags, file formats. Inside a module, do
what you like. At an edge, this file is law. It does not change during Waves 1–2. If it is wrong, the
contact part (part 31) records the hole in `.fullsend/notes/spec-gaps.md` and the orchestrator
patches this file between waves, never a builder in the middle of one.

The plan that uses it is [README.md](README.md). "Research A" to "D" are the inventories written for this plan at `91cc43c`: [A-engine](research/A-engine.md), [B-ai](research/B-ai.md), [C-server](research/C-server.md), [D-ci-spec](research/D-ci-spec.md); they give file:line facts the rules below lean on. Every TypeScript file's fate is in
[PORT-MAP.md](PORT-MAP.md). The briefs of the parts are in [parts/](parts/).

Contents:

1. Workspace layout
2. Stack (toolchain and pinned dependencies)
3. Purity (CLAUDE.md rule 4, in Rust)
4. Translation rules, TypeScript to Rust
5. Wire compatibility (serde conventions, the hash)
6. The engine crate
7. Card scripts
8. The testkit (`scenario()` in Rust)
9. The AI crate
10. The WASM bindings and the web client's seams
11. The server
12. The `jackioh` CLI
13. Golden traces
14. The training arena and the promotion gate
15. The spec graph
16. Fullsend notes and the fixed assumption keys

---

## 1. Workspace layout

One Cargo workspace at the repository root. Rust replaces everything that is not the browser UI:
`packages/shared`, `packages/engine`, `packages/validator`, `packages/cards` (its code; its data
moves), `packages/ai`, `apps/server` and `ladder/`. `apps/web` (TypeScript/React) and `e2e/`
(Cypress) stay TypeScript. `bot/` stays Python and is not part of this rewrite, apart from part 39.

```
Cargo.toml                 workspace manifest: members = ["crates/*"], resolver = "3", [workspace.dependencies]
Cargo.lock                 committed
rust-toolchain.toml        channel "1.97.0", components rustfmt + clippy, targets wasm32-unknown-unknown
rustfmt.toml               max_width = 110, edition = "2024"
.cargo/config.toml         [alias] jackioh = "run --release -p jackioh-tools --"
crates/
  engine/                  package jackioh-engine (lib)   ← packages/shared, packages/engine/src, packages/validator
    Cargo.toml             features: testkit (off), ts (off)
    clippy.toml            purity (§3)
    src/lib.rs             module tree (frozen by part 1) + the re-exports of §6
    src/prelude.rs         what a card script may name (§7.1)
    src/wire/<x>.rs        ← packages/shared/src/<x>.ts
    src/<module>.rs        one per packages/engine/src/<module>.ts
    src/effects/<x>.rs     one per packages/engine/src/effects/<x>.ts (effects/index.ts → effects/mod.rs)
    src/subsystems/<x>.rs  one per packages/engine/src/subsystems/<x>.ts (index.ts → mod.rs)
    src/validator.rs       ← packages/validator/src/index.ts
    src/testkit/           feature "testkit": scenario() (§8), invariants, glow
    tests/rules.rs         the one integration-test binary for ported engine tests: `mod rules;`
    tests/rules/mod.rs     `pub mod <x>;` per file below, `pub mod fixtures;`
    tests/rules/<x>.rs     one per ported packages/engine/test/<x>.test.ts
    tests/rules/fixtures/  ← packages/engine/test/fixtures/*.ts (test-only scripts and catalogs)
    tests/golden.rs        golden trace replay (§13)
    tests/golden/games.jsonl
    tests/golden/01-hotseat-full-game.json
    tests/fixtures/code-input-cases.json  shared with apps/web (§10.4)
  cards/                   package jackioh-cards (lib)    ← packages/cards/src, catalog.json, flavour.json, patches/
    Cargo.toml
    clippy.toml            purity (§3)
    build.rs               generates the registry from src/scripts/** (§7.4)
    catalog.json           copied from packages/cards/catalog.json, byte for byte (part 1); the original is deleted by part 37
    flavour.json           copied likewise
    patches/               copied likewise
    src/lib.rs             include!(registry) + register_all() + catalog accessors
    src/query.rs           ← packages/cards/src/query.ts
    src/ky_test_bank.rs    ← packages/cards/src/kyTestBank.ts
    src/scripts/core/      ← packages/cards/src/scripts/*.ts            (one .rs per card: script + its tests)
    src/scripts/classic/   ← packages/cards/src/scripts/classic/*.ts
    src/scripts/classic_plus/ ← packages/cards/src/scripts/classic-plus/*.ts
    tests/cards.rs         the one integration-test binary for cross-card tests: `mod cross;`
    tests/cross/mod.rs, tests/cross/<x>.rs   one per ported packages/cards/test/<x>.test.ts that is not a per-card file
  ai/                      package jackioh-ai (lib)       ← packages/ai/src (minus personas, which go to the web)
    Cargo.toml, clippy.toml
    generation.json        the main AI's generation record (§14.3)
    src/<module>.rs        one per packages/ai/src/<module>.ts
    tests/ai.rs, tests/ai/<x>.rs   ported packages/ai/test/<x>.test.ts
  wasm/                    package jackioh-wasm (cdylib)  the bindings of §10, nothing else
  server/                  package jackioh-server (bin "jackioh-server" + lib) ← apps/server
    Cargo.toml
    Dockerfile
    .env.example
    migrations/            copied from apps/server/src/db/migrations/, byte for byte (part 1)
    src/main.rs, src/lib.rs, src/…   §11
    tests/server.rs        the one integration-test binary: `mod api; mod actor; mod store; mod support;`
    tests/<area>/<x>.rs
    tests/sql/             copied from apps/server/test/sql/ (unchanged SQL)
    tests/db/run.sh, tests/deploy/rehearse.sh
  tools/                   package jackioh-tools (bin "jackioh") ← packages/*/scripts, ladder/, scripts/catalog-version.mjs
    src/main.rs            clap dispatch (§12), written by part 1
    src/<command>.rs       one per subcommand
training/                  the AI training lanes (§14): README.md, improve.md, unban.md, loop.sh, history/
spec/                      SPEC.md split into notes (§15)
apps/web/src/wasm/         the TS wrapper over the WASM module (§10); pkg/ is generated and gitignored
apps/web/src/wire/         the client's wire layer (§10.4): generated types and constants, hand-kept helpers
scripts/build-wasm.sh      builds crates/wasm into apps/web/src/wasm/pkg
```

Rules that follow from the tree:

- `match` is a Rust keyword, so the server's match actor lives in `crates/server/src/actor/`.
- **Every `lib.rs`, `mod.rs`, `main.rs` and `Cargo.toml` is written once, by part 1**, and declares
  every module listed in [PORT-MAP.md](PORT-MAP.md) for its crate. A Wave 1 part fills module files;
  it never edits those files. A dependency or module it wishes existed goes under `GAPS` in its notes
  (§16) and part 31 adds it.
- Integration tests are one binary per crate (`tests/rules.rs`, `tests/cards.rs`, `tests/ai.rs`,
  `tests/server.rs`, plus `tests/golden.rs`). Each file directly under `tests/` is its own binary
  and its own link step, so 300 of them would cost minutes per `cargo test`.
- Data files are **copied** by part 1, not moved: the TypeScript engine must keep working on
  `staging` until part 23 has recorded the golden traces and Wave 3 has used them. Part 37 deletes
  the originals.

## 2. Stack

Toolchain: Rust **1.97.0**, edition **2024**, pinned in `rust-toolchain.toml`. Node **24.19.0** and
pnpm **11.3.0** remain for `apps/web` and `e2e` only.

`[workspace.dependencies]` in the root `Cargo.toml` pins every third-party crate. A crate's own
`Cargo.toml` names them with `{ workspace = true }` and nothing else. Nobody adds a crate that is not
on this list during Waves 1–3.

| Crate | Version | Features | Used by |
|---|---|---|---|
| serde | 1.0.229 | derive | engine, cards, ai, wasm, server, tools |
| serde_json | 1.0.151 | — (no `preserve_order`) | engine, cards, ai, wasm, server, tools |
| indexmap | 2.14.2 | serde | engine, cards, ai |
| ts-rs | 12.0.1 | serde-compat, no-serde-warnings | engine (feature `ts`, off by default) |
| wasm-bindgen | =0.2.129 | — | wasm |
| js-sys | 0.3.106 | — | wasm (the clock for the AI's `should_stop`) |
| console_error_panic_hook | 0.1.7 | — | wasm |
| axum | 0.8.9 | ws, macros | server |
| tokio | 1.53.2 | full, test-util | server |
| tower | 0.5.3 | util | server (tests: `oneshot`) |
| tower-http | 0.7.1 | cors, trace | server |
| sqlx | 0.9.0 | runtime-tokio, tls-rustls, postgres, json, time, uuid | server |
| jsonwebtoken | 11.1.0 | — | server |
| reqwest | 0.13.5 | json, rustls-tls (default-features = false) | server (JWKS, Supabase Auth `/user` and admin calls) |
| hmac | 0.13.0 | — | server |
| sha2 | 0.11.0 | — | server |
| base64 | 0.23.1 | — | server |
| uuid | 1.27.0 | v4, serde | server |
| time | 0.3.55 | serde, formatting, parsing | server |
| getrandom | 0.4.3 | — | server (invite codes, seeds) |
| tracing | 0.1.44 | — | server |
| tracing-subscriber | 0.3.23 | json, env-filter | server |
| anyhow | 1.0.104 | — | server (main and CLIs only), tools |
| thiserror | 2.0.21 | — | server |
| clap | 4.6.7 | derive | server (main), tools |
| rayon | 1.12.0 | — | tools (parallel games) |

Crate edges (path dependencies; every crate also takes the third-party crates its rows above name):

| Crate | `[dependencies]` | `[dev-dependencies]` |
|---|---|---|
| `jackioh-engine` | — (features: `testkit`, `ts` = `dep:ts-rs`) | `jackioh-cards` (golden traces, hotseat fixture) |
| `jackioh-cards` | `jackioh-engine` | `jackioh-engine` with `features = ["testkit"]` |
| `jackioh-ai` | `jackioh-engine` | `jackioh-cards`, `jackioh-engine` with `testkit` |
| `jackioh-wasm` | `jackioh-engine`, `jackioh-cards`, `jackioh-ai` | — |
| `jackioh-server` | `jackioh-engine`, `jackioh-cards`, `jackioh-ai` | `jackioh-engine` with `testkit` |
| `jackioh-tools` | `jackioh-engine` with `testkit` (fuzz's invariant monitor), `jackioh-cards`, `jackioh-ai` | — |

The engine's own integration tests run with `cargo test -p jackioh-engine --features testkit`. A dev-dependency
on a crate that depends on you is legal for integration tests (`tests/`), which link the normal library.

`wasm-bindgen-cli` **0.2.129** (the same version as the crate, or the build refuses) is installed by
`scripts/build-wasm.sh` from its GitHub release tarball, never compiled.

## 3. Purity (CLAUDE.md rule 4, in Rust)

`jackioh-engine`, `jackioh-cards` and `jackioh-ai` are pure: no clock, no I/O, no environment, no
threads, no OS randomness, no hash-ordered collections, **no mutable statics**. Two mechanisms
enforce it, and nothing else is needed (the ESLint purity rules go):

1. **Their `Cargo.toml` depends only on `serde`, `serde_json`, `indexmap`, each other, and `ts-rs`
   behind the engine's off-by-default `ts` feature.** No `rand`, no `getrandom`, no `tokio`, no
   `time`. A crate you cannot name, you cannot call.
2. **Each has the same `clippy.toml`** (clippy reads the one in the crate's own directory) and CI
   runs `cargo clippy --workspace --all-targets -- -D warnings`:

```toml
disallowed-types = [
  { path = "std::collections::HashMap", reason = "iteration order is random per process: use indexmap::IndexMap (insertion order, like a JS Map) or BTreeMap" },
  { path = "std::collections::HashSet", reason = "iteration order is random per process: use indexmap::IndexSet or BTreeSet" },
  { path = "std::time::Instant", reason = "CLAUDE.md rule 4: no clocks in pure crates" },
  { path = "std::time::SystemTime", reason = "CLAUDE.md rule 4: no clocks in pure crates" },
  { path = "std::fs::File", reason = "CLAUDE.md rule 4: no I/O in pure crates" },
  { path = "std::thread::Thread", reason = "CLAUDE.md rule 4: no threads in pure crates" },
  { path = "std::sync::Mutex", reason = "no mutable statics: per-game state lives in GameState, per-call state in EngineSink" },
  { path = "std::sync::RwLock", reason = "no mutable statics" },
  { path = "std::cell::RefCell", reason = "no interior mutability in the rules" },
]
disallowed-methods = [
  { path = "std::time::Instant::now", reason = "CLAUDE.md rule 4" },
  { path = "std::time::SystemTime::now", reason = "CLAUDE.md rule 4" },
  { path = "std::env::var", reason = "CLAUDE.md rule 4" },
  { path = "std::env::vars", reason = "CLAUDE.md rule 4" },
  { path = "std::fs::read", reason = "CLAUDE.md rule 4" },
  { path = "std::fs::read_to_string", reason = "CLAUDE.md rule 4" },
  { path = "std::fs::write", reason = "CLAUDE.md rule 4" },
  { path = "std::thread::spawn", reason = "CLAUDE.md rule 4" },
  { path = "std::process::exit", reason = "CLAUDE.md rule 4" },
]
```

- **The only statics** in the pure crates are the catalog and the script registry, each a
  `std::sync::OnceLock` set once by `jackioh_cards::register_all()` and read-only after. Everything
  that varies per game (transient defs, fused scripts, digests) is read from `GameState`;
  everything that varies per call (re-entrancy flags such as TS's `let converting` in
  `replacements.ts:383` and `let dryRunning` in `subsystems/scorer.ts:225`) is a field of the
  per-call `EngineSink` (§6.5). The tools run games on many threads at once; a mutable static would
  let one game see another's cards.
- Data reaches a pure crate only at compile time: `include_str!("../catalog.json")`. Test modules of
  a pure crate use `include_str!` too (`clippy.toml` applies to them).

## 4. Translation rules, TypeScript to Rust

Every Wave 1 builder translates by these rules. They are mechanical on purpose: two builders who
both need `zones.activeUnitsOf` write the same call without ever seeing each other.

### 4.1 Paths

| TypeScript | Rust |
|---|---|
| `packages/engine/src/<camelName>.ts` | `crates/engine/src/<snake_name>.rs` (`playSteps.ts` → `play_steps.rs`) |
| `packages/engine/src/effects/<x>.ts` | `crates/engine/src/effects/<snake_x>.rs` (`effects/index.ts` → `effects/mod.rs`) |
| `packages/engine/src/subsystems/<x>.ts` | `crates/engine/src/subsystems/<snake_x>.rs` (`index.ts` → `mod.rs`) |
| `packages/shared/src/<x>.ts` | `crates/engine/src/wire/<snake_x>.rs` (`catalog-types.ts` → `catalog_types.rs`) |
| `packages/validator/src/index.ts` | `crates/engine/src/validator.rs` |
| `packages/ai/src/<x>.ts` | `crates/ai/src/<snake_x>.rs` |
| `packages/cards/src/scripts/NNN-slug.ts` | `crates/cards/src/scripts/core/cNNN_slug.rs` |
| `packages/cards/src/scripts/NNN-K-slug.ts` | `crates/cards/src/scripts/core/cNNN_K_slug.rs` |
| `packages/cards/src/scripts/t-slug.ts` | `crates/cards/src/scripts/core/t_slug.rs` |
| `packages/cards/src/scripts/classic/…` | `crates/cards/src/scripts/classic/…` (same rule) |
| `packages/cards/src/scripts/classic-plus/…` | `crates/cards/src/scripts/classic_plus/…` (same rule) |
| `packages/cards/test/<card path>.test.ts` | the `#[cfg(test)] mod tests` at the bottom of that card's `.rs` |
| `packages/engine/test/<x>.test.ts` | `crates/engine/tests/rules/<snake_x>.rs` (`effects-core.test.ts` → `effects_core.rs`) |
| `packages/engine/test/fixtures/<x>.ts` | `crates/engine/tests/rules/fixtures/<snake_x>.rs` |
| `packages/cards/test/<x>.test.ts` (not a card) | `crates/cards/tests/cross/<snake_x>.rs` |
| `packages/ai/test/<x>.test.ts` | `crates/ai/tests/ai/<snake_x>.rs` |
| `apps/server/src/api/<x>.ts` | `crates/server/src/api/<snake_x>.rs` |
| `apps/server/src/match/<x>.ts` | `crates/server/src/actor/<snake_x>.rs` |
| `apps/server/src/ranked/<x>.ts` | `crates/server/src/ranked/<snake_x>.rs` |
| `apps/server/src/db/<x>.ts` | `crates/server/src/db/<snake_x>.rs` |
| `apps/server/test/<area>/<x>.test.ts` | `crates/server/tests/<area>/<snake_x>.rs` (`match/` → `actor/`) |

Slugs: every `-` and `.` becomes `_`, and a name that would start with a digit gets a `c` prefix
(`012-1-devour.ts` → `c012_1_devour.rs`). A module named after a Rust keyword takes a trailing
underscore (`type.ts` → `type_.rs`, `match` → `actor`). [PORT-MAP.md](PORT-MAP.md) lists every
resulting path, so nobody has to apply these by hand.

### 4.2 Names

- Functions and variables: `camelCase` → `snake_case` (`activeUnitsOf` → `active_units_of`,
  `viewFor` → `view_for`). Acronyms lower-case whole (`aiToAct` → `ai_to_act`).
- Types, interfaces, type aliases: unchanged (`GameState`, `CardInstance`, `PlayerView`). A TS type
  that is only a string union becomes a Rust `enum` with the same name.
- Constants: unchanged (`TURN_CAP_PLAYER_TURNS`). A constant object (`AI_GATE = { … }`) becomes a
  `pub const AI_GATE: AiGate = AiGate { … }` with a struct named in PascalCase after it. A constant
  array becomes a `pub const X: &[T]`.
- An exported function keeps its module: a call to `zones.activeUnitsOf` is
  `crate::zones::active_units_of`. Never move a function to another module during Wave 1.
- TS namespaces keep their path: `subsystems.chooseAction` is `crate::subsystems::ai_policy::choose_action`
  and is also re-exported as `crate::subsystems::choose_action` by part 1's `subsystems/mod.rs`.

### 4.3 Types

| TypeScript | Rust | serde |
|---|---|---|
| `number` (game quantity: stat, cost, count, turn, damage) | `i32` | — |
| `number` (index into an array) | `usize` | — |
| `number` (rng cursor, `nextId`, `nextSeq`) | `u32` | — |
| `number` that can be fractional (AI scores, probabilities, scorer weights) | `f64` | — |
| `string` | `String` (`&'static str` for constants) | — |
| string literal union `"a" \| "b"` | `enum` | `#[serde(rename_all = "camelCase")]`, or `#[serde(rename = "…")]` per variant when the literal is not camelCase (`"hero-death"`, `"Field Spell"`) |
| `boolean` | `bool` | — |
| `T[]`, `readonly T[]` | `Vec<T>` | — |
| `[A, B]` tuple | `(A, B)` | a 2-array, as TS |
| `x?: T` (optional property) | `Option<T>` | `#[serde(default, skip_serializing_if = "Option::is_none")]` |
| `x?: boolean` that TS writes as `false` on purpose (e.g. `faceUp`) | `Option<bool>` | as `x?:` — `Some(false)` serialises `false`, `None` is absent, exactly TS's three states |
| `x?: true` | `Option<bool>` holding only `Some(true)` | as `x?:` |
| `x: T \| null` | `Option<T>` | **no** skip: serialises `null`, as TS |
| `Record<string, T>` / `{ [k: string]: T }` | `IndexMap<String, T>` (insertion order, as JS) | — |
| `Record<PlayerId, T>` | `PerPlayer<T>` (§6.4) | `{ "p1": …, "p2": … }` |
| `Partial<Record<PlayerId, T>>` | `PerPlayerOpt<T>` = `{ p1: Option<T>, p2: Option<T> }` with skip | — |
| `Map<K, V>` | `IndexMap<K, V>` | — |
| `Set<T>` | `IndexSet<T>` | — |
| `Record<string, unknown>` (the `data` and `memory` bags) | `IndexMap<String, serde_json::Value>` | — |
| `unknown` / `any` crossing JSON | `serde_json::Value` | — |
| discriminated union `{ type: "x"; … } \| …` | `enum` | `#[serde(tag = "type", rename_all = "camelCase")]` |
| union discriminated on another key (`kind`, `pick`, `z`) | `enum` | `#[serde(tag = "<that key>")]` |
| `Effect` (TS: `{ kind, apply(ctx), expand?(ctx, memo) }`, a closure object) | `Effect` (§6.6) | never serialised |
| a hook (TS: `(ctx) => Effect[]` and friends) | `Option<Hook>` / the typed hook aliases of §6.6 | never serialised |

### 4.4 Semantics that must survive the port exactly

These are the places where a straightforward port silently changes behaviour. The golden traces
(§13) catch every one, at the cost of a debugging session; follow the rule and there is none.

1. **Sorting.** `Array.prototype.sort` is stable; so are `slice::sort` and `sort_by`. Never use
   `sort_unstable*` in the pure crates. A comparator returning `a - b` becomes `a.cmp(&b)`. String
   `<` compares UTF-16 code units; every compared string in the engine is ASCII (ids, keys), so
   `str::cmp` equals it. The 18 sort sites are listed in research A §3.4; `instanceView.ts:49`'s
   comparator-less `.sort()` on strings is a plain `sort()` on `String`s.
2. **Iteration order.** JS `Map`, `Set` and string-keyed objects iterate in insertion order, hence
   `IndexMap`/`IndexSet`. `[...new Set(xs)]` (dedupe keeping first-seen order) is
   `xs.into_iter().collect::<IndexSet<_>>().into_iter().collect()`.
3. **`JSON.stringify` as equality.** `enchantments.ts:27` and `subsystems/scorer.ts:246-247` compare
   or key by `JSON.stringify` (insertion order). Port as `==` on `#[derive(PartialEq)]` types and as
   `serde_json::to_string` of the same struct (field order = TS's literal order) for keys.
4. **Integer arithmetic.** `Math.floor(a / b)` on non-negative integers is `a / b`; `Math.ceil(a / b)`
   is `(a + b - 1) / b` for `a >= 0, b > 0`; `Math.round(x)` rounds .5 up (`(x + 0.5).floor()`);
   `Math.trunc` is `as i32` on an `f64`; `x | 0` is `x as i32`; `Math.imul` is `wrapping_mul`;
   `>>> 0` is `as u32`; `Math.min(...[])` is `Infinity`, so guard it where TS guards it.
   `Number.parseFloat(index)` on catalog indexes (`"12"`, `"12.3"`, `"T-…"`) is
   `index.parse::<f64>().unwrap_or(f64::INFINITY)` (no current index hits JS's lenient-prefix case).
5. **Randomness.** `rng.ts` ports bit for bit (§6.3). Every draw happens in the same order as in TS:
   never hoist, cache, skip or add a `next()`/`int`/`pick`/`shuffle` call. The side streams keep
   their seeds: `"{seed}:instance-ids{stream}"` (`state.ts:890`), `"{seed}:instance-ids:{nextId}"`
   (`state.ts:950`), `"zephyrs-dry-run"` cursor 0 (`subsystems/scorer.ts:346`).
6. **Truthiness.** `if (x)` on a number is `x != 0`; on a string `!x.is_empty()`; on an optional
   `is_some()`. `a || b` on numbers picks `b` when `a == 0`: `if a != 0 { a } else { b }`.
7. **Presence.** TS deletes fields (92 `delete` statements) and leaves optionals absent so that old
   hashes stay stable. Rust sets the `Option` to `None` exactly where TS deletes, and to `Some(false)`
   exactly where TS writes `false`. The hash (§5.2) sees the difference.
8. **Cloning.** `cloneState` (`JSON.parse(JSON.stringify(state))`) is `GameState::clone()`. Its JSON
   side effects (dropping `undefined`, `-0` to `0`) cannot happen in Rust, so nothing to port.
9. **Errors.** A TS function that returns `{ error }` or a refusal string returns
   `Result<T, EngineError>` (§6.2) **with the TS message text verbatim**: 83 TS test assertions match
   it and the client shows it. A TS `throw` on an impossible state is `panic!` with the same message.
10. **Defensive re-parsing of bags.** The ~58 `typeof`/`Array.isArray` guards on `data`/`memory`
    values port as `Value::as_i64()`/`as_str()`/`as_array()` with the same fallbacks. Do not type the
    bags: their JSON is hashed (§5.2).

## 5. Wire compatibility

### 5.1 JSON

The client keeps its TypeScript. Whatever JSON the TS engine and server produced, the Rust ones
produce: the same keys, casing, discriminators, `null`s and absences. The golden traces (§13) prove it
for state, views and events; the server's tests prove it for REST and WebSocket bodies.

- Every serialised struct: `#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]` and
  `#[serde(rename_all = "camelCase")]`.
- Every serialised enum: as §4.3. Unit-variant string unions also derive
  `Copy, Eq, Hash, PartialOrd, Ord`.
- `PlayerId` is `enum PlayerId { P1, P2 }`, serialised `"p1"`/`"p2"`.
- **The client's TS types are generated, not hand-written.** With `--features ts`, every type in
  `crates/engine/src/wire/` and every type reachable from `PlayerView`, `GameEvent`, `Action`,
  `CardDef`, `GameRecord` and `GameState` (the web's tests read the state's fields) derives `ts_rs::TS` with
  `#[ts(export, export_to = "../../apps/web/src/wire/generated/")]`, and
  `cargo test -p jackioh-engine --features ts export_bindings` writes them. They are committed; CI
  regenerates them and fails on a diff (V20).
- **The client's constants are generated too**, by two plain tests (no new dependency):
  `crates/engine/tests/export_config.rs` writes `apps/web/src/wire/engineConfig.ts` (the 12 engine
  constants and 2 types the web imports: `DECK_SIZE`, `MAX_COPIES`, `DIFFICULTIES`, `AI_DIFFICULTY`,
  `AI_TUTORIAL`, `HUMAN_HANDICAP`, `DRAWS_PER_TURN`, `TURN_CAP_PLAYER_TURNS`, `HERO_HEALTH`,
  `MAX_MANA`, `COIN_DEF_ID`, `GLITCH_DEF_ID`, `type Handicap`, `type Difficulty`), and
  `crates/server/tests/export_config.rs` writes `apps/web/src/wire/serverConfig.ts` (every constant of
  `apps/server/src/config.ts` that a file under `apps/web/src` or `e2e/` imports today; part 18 lists
  them). Each line is `export const NAME = <JSON value>;`. Same diff check.

### 5.2 The state hash

`hash_state(state)` reproduces TS's `hashState` (`packages/engine/src/replay.ts:12-37`) **exactly**,
because practice saves on players' devices store it (`apps/web/src/practice/core.ts:279` refuses a
save that does not fold to its hash), `packages/cards/test/fixtures/01-hotseat-full-game.json` pins
`"a798906b"`, and e2e specs 01, 13 and 22 compare browser and Node hashes:

1. Serialise the state to `serde_json::Value`; remove the top-level keys `applied` and `opening`.
2. `canonical(v)`: a string, number, bool or null as `JSON.stringify` writes it (serde_json writes the
   same for integers and for the escapes `\" \\ \b \f \n \r \t \u00xx`, lower-case hex, non-ASCII
   raw); an array as `[` + items joined by `,` + `]`; an object as `{` + `"key":canonical(value)`
   pairs, keys sorted ascending, joined by `,` + `}`.
3. FNV-1a 32 over the **UTF-16 code units** of that string (`for u in s.encode_utf16()`: `h ^= u as
   u32; h = h.wrapping_mul(0x0100_0193)`; start `0x811c_9dc5`), printed as 8 lower-case hex digits.
   UTF-8 bytes would differ: state strings carry `× − – ♾ ³ ²`.

`canonical(&serde_json::Value) -> String` and `fnv1a32_utf16(&str) -> String` are public in
`crates/engine/src/replay.rs` (part 5); the golden traces (§13) and the server's migration checksum
use the same two functions.

## 6. The engine crate

### 6.1 Entry points

`crates/engine/src/lib.rs` re-exports what `packages/engine/src/index.ts` exported (by §4.2), the
wire types, `effects` and `subsystems` as modules, and these entry points every other crate uses:

```rust
pub fn create_game(args: &CreateGameArgs) -> GameState;            // TS createGame({seed, decks, catalog?, handicaps?, lastBoards?, glitchBoards?, dealt?})
pub fn begin_game(state: &GameState) -> ReduceResult;
pub fn reduce(state: &GameState, action: &Action) -> ReduceResult;
pub fn legal_actions(state: &GameState, player: PlayerId) -> Vec<ActionBody>;
pub fn view_for(state: &GameState, player: PlayerId) -> PlayerView;
pub fn seat_to_act(state: &GameState) -> Option<PlayerId>;
pub fn hash_state(state: &GameState) -> String;                     // §5.2
pub fn fold(args: &FoldArgs) -> FoldResult;                         // TS fold: { state, errors: [{nonce, error}] }
pub fn last_board_for(state: &GameState, seat: PlayerId) -> Vec<LastBoardEntry>;
pub fn seat_played_by(state: &GameState, home: PlayerId) -> PlayerId;
pub fn mulligan_owed(state: &GameState) -> Vec<PlayerId>;
pub fn seats_swapped(state: &GameState) -> bool;
pub fn summarize_game(args: &FoldArgs) -> Option<GameSummary>;
pub fn registered_catalog() -> &'static CardDefs;                   // after jackioh_cards::register_all()

pub struct ReduceResult {                                           // serialises as { state, events, error? }
    pub state: GameState,
    pub events: Vec<GameEvent>,
    pub error: Option<String>,                                      // Some ⇔ refused; then `state` is the input, unchanged, and `events` is empty
}
```

`reduce` never panics on an illegal action: it returns the input state with `error: Some(msg)`,
`msg` being TS's text. The `rng` argument TS's `reduce` takes is dropped: the match rng is always
rebuilt from `(state.seed, state.rng_cursor)` (`reduce.ts:436`), as TS already does when no rng is
passed. The `activatePower` action stays (a legacy alias of `activate`): stored match logs carry it
and `match_actions` rows are immutable by trigger, so it is load-bearing, not an exception to remove.

### 6.2 Errors

```rust
pub struct EngineError { pub message: String }                      // Display = message
```

One type, one field.

### 6.3 Randomness (`crates/engine/src/rng.rs`, part 1)

```rust
pub struct Rng { seed_int: u32, cursor: u32 }
impl Rng {
    pub fn new(seed: &str, cursor: u32) -> Rng;    // seed_int = xmur3 over seed.encode_utf16()
    pub fn cursor(&self) -> u32;
    pub fn next(&mut self) -> f64;                 // mulberry32 at (seed_int, cursor) / 4294967296.0, then cursor += 1
    pub fn int(&mut self, n: i32) -> i32;          // n <= 0 → 0 with no draw; else (next() * n as f64).floor() as i32 % n
    pub fn pick<'a, T>(&mut self, list: &'a [T]) -> Option<&'a T>;   // empty → None, no draw
    pub fn shuffle<T: Clone>(&mut self, list: &[T]) -> Vec<T>;       // Fisher–Yates top down: for i in (1..len).rev() { j = int(i+1) }
    pub fn coin(&mut self) -> bool;                // next() < 0.5
    pub fn chance(&mut self, p: f64) -> bool;      // next() < p
    pub fn lucky<T>(&mut self, x: i32, roll: impl FnMut(&mut Rng) -> T, better: impl Fn(T, T) -> T) -> T;  // 1 + max(0, x) rolls
}
```

`rng.ts`'s arithmetic line for line: `Math.imul` → `wrapping_mul` on `u32`, `>>> n` → `>> n` on
`u32`, the `a + Math.imul(…)` double sum → `wrapping_add` on `u32`. Part 1's unit test pins the first
ten `next()` values of `Rng::new("golden", 0)` and of `Rng::new("jackioh-fuzz-1", 0)` to the ones TS
prints (part 1 runs `pnpm exec tsx -e` once to get them).

### 6.4 State, wire and script types

Part 1 writes these by translating, field for field and by §4.3, every type **and function** in:

- `packages/shared/src/{catalog-types,actions,events,view}.ts` → `crates/engine/src/wire/*.rs`
- `packages/engine/src/state.ts` → `state.rs` (types, `create_game`, `clone`, `new_instance`,
  `find_instance`, `validate_deck`, the handicap helpers)
- `packages/engine/src/script.ts` → `script.rs`
- `packages/engine/src/config.ts` → `config.rs` (values verbatim; plus the rule-9 constants research A
  §4 found outside `config.ts`, moved in, and the training constants of §14.2)
- `packages/engine/src/rng.ts` → `rng.rs`

(`replay.ts` is part 5's, with `reduce`: its `fold` calls `reduce`. Part 5 writes `canonical`,
`fnv1a32_utf16`, `hash_state` and `fold` in `replay.rs` by §5.2; everyone else calls them by those names.)

and makes `cargo check -p jackioh-engine` pass on them with every other module file present and
empty. That compiled set **is** the freeze for state, wire and script types. A Wave 1 builder who
needs a field that is not there writes the code against the field it wishes existed (the TS name,
snake_cased) and lists it under `GAPS`; part 31 adds it.

```rust
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Default)]
pub struct PerPlayer<T> { pub p1: T, pub p2: T }
impl<T> std::ops::Index<PlayerId> for PerPlayer<T> { type Output = T; /* … */ }
impl<T> std::ops::IndexMut<PlayerId> for PerPlayer<T> { /* … */ }
```

### 6.5 Mutation convention and the per-call sink

A TS function that mutates its `state` argument takes `&mut GameState`; one that only reads takes
`&GameState`. TS's `EngineSink = {state, events, rng}` (`resolve.ts:13`) is

```rust
pub struct EngineSink<'a> {
    pub state: &'a mut GameState,
    pub events: &'a mut Vec<GameEvent>,
    pub rng: &'a mut Rng,
    pub converting: u32,      // replacements.ts:383's re-entrancy counter, formerly a module `let`
    pub dry_running: bool,    // subsystems/scorer.ts:225's flag, formerly a module `let`
}
```

and every TS mutator that took a sink takes `&mut EngineSink`. A builder who calls into another
module and cannot tell which form it takes guesses from the TS (assigns to anything reachable from
`state`, or calls something that does → `&mut`); the compiler finds every miss in Wave 3.

### 6.6 Effects and hooks are shared closures

TS effects are closure objects and TS hooks are functions; fused cards compose them at runtime from
ingredient ids. Rust keeps that shape:

```rust
pub type Hook = std::sync::Arc<dyn Fn(&EffectContext) -> Vec<Effect> + Send + Sync>;
pub fn hook(f: impl Fn(&EffectContext) -> Vec<Effect> + Send + Sync + 'static) -> Hook;

#[derive(Clone)]
pub struct Effect {
    pub kind: &'static str,
    pub apply: std::sync::Arc<dyn Fn(&mut EffectContext) + Send + Sync>,
    pub expand: Option<std::sync::Arc<dyn Fn(&mut EffectContext, &Memo) -> EffectPart + Send + Sync>>,
}
```

- A TS effect constructor `damage(args) => ({ kind: "damage", apply(ctx) { … } })` becomes
  `pub fn damage(args: DamageEffectArgs) -> Effect { Effect::new("damage", move |ctx| { … }) }`.
- Every other function-valued `Script` field (`cost`, `setStat`, `aura`, `conditionMet`, `preview`,
  `costAura`, `heroGuard`, …) is `Option<Arc<dyn Fn(<its TS argument struct>) -> <its TS return> +
  Send + Sync>>` with a type alias named after the TS type (`AuraHook`, `ConditionHook`, …). Part 1
  writes the aliases in `script.rs`.
- **The registration hooks go.** TS registers ~25 handlers at import time only to break import
  cycles (`registerWorkHandler` ×16, `registerDefaultWorkHandler`, `registerPromptAnswerer` ×4,
  `registerTargetingHooks`, `registerCastDriver`, `registerDeclarationCheck`,
  `registerGraveyardRedirect`). Rust modules in one crate call each other directly: `work.rs`'s
  dispatcher is a `match resume.hook.as_str() { "@setup" => setup::…, "@deaths" => state_check::…,
  … }` over the table in research A §2.7, and `prompts.rs` matches answerer keys the same way.
  `registerAttackBar` (never registered) is not ported.
- **Effect argument types are data.** Every struct and enum an effect constructor takes
  (`TargetSpec`, `BoardScope`, `PlayerSpec`, `DamageEffectArgs`, `SummonArgs`, … research A §2.9's
  50 types) derives `Serialize, Deserialize, Clone, Debug, PartialEq` with camelCase serde, and
  `Default` when all its fields are optional, so a card can build one from the TS object literal
  with `json_as(json!({ … }))`. A field that holds a function (a filter callback) is skipped by
  serde (`#[serde(skip)]`) and set in Rust.
- **Fused scripts are built on lookup, never registered.** `scripts::script_of(state, def_id)`
  returns the registry's `CardScripts` for a catalog id, or, for a fused/crafted id found in
  `state.transient_defs`, a `CardScripts` it composes then and there from the ingredients' scripts
  with TS's `combineObjects` rules by calling
  `crate::subsystems::fuse::compose_fused_scripts(state: &GameState, def: &CardDef) -> CardScripts`
  (part 8 writes it; part 2's `scripts.rs` calls it). `syncFusedScripts` and the process-global
  digest table (`catalog.ts:181`) are not ported; `reduce` no longer mutates its input (research A §8
  #18).

## 7. Card scripts

### 7.1 One file per card

`crates/cards/src/scripts/<set>/<file>.rs` holds the card's script and, at the bottom, its tests:

```rust
//! SPEC §8.1 #2 Bigot. <the TS file's header comment, kept>

use jackioh_engine::prelude::*;
use jackioh_engine::effects::{destroy, destroy_all};

pub const ID: &str = "core-002";            // the id the TS file passed to cardDef(...)

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            targets: vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"], "notTags": ["Human"] }))],
            cry: Some(hook(|_ctx| vec![destroy(DestroyArgs { target: TargetSpec::Chosen { index: None } })])),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| vec![destroy_all(json_as(json!({ "side": "enemy", "rows": ["units"], "notTags": ["Human"] })))])),
            ..Script::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;
    #[test]
    fn destroys_the_chosen_enemy_non_human_unit() { /* … */ }
}
```

- `ID` is mandatory: `build.rs` reads it (§7.4). TS's `def: cardDef("core-002")` is not ported:
  the def comes from the catalog by `ID`.
- A TS `export const radiant: Script = base;` is `radiant: base.clone()` (build `base` once in a
  local).
- `jackioh_engine::prelude` re-exports every engine value a TS card script imports today (research A
  §5.3's 74 values, the `subsystems` members cards use, the effect-argument types and
  `serde_json::json`) plus `json_as<T: DeserializeOwned>(Value) -> T`, which lets an object literal
  port as `json!` when its argument struct is large. Part 1 writes `prelude.rs` from that list. A
  script never touches `GameState` fields directly (CLAUDE.md rule 5).
- `packages/cards/src/query.ts`'s wrapper (`catalog.query`, `pool`, `TRAP_TYPES`, …) ports to
  `crates/cards/src/query.rs` (part 9), and the two scripts that use it import
  `crate::query::*`.

### 7.2 Hooks

Every hook is an `Option<Hook>` (or its typed alias) built with `hook(|ctx| …)` (§6.6). A TS hook
that closes over a module constant reads it directly; a TS factory (`makeCry(3)`) becomes a Rust
function returning a `Hook`. `Script.resume: Record<string, Hook>` is `IndexMap<&'static str, Hook>`.
`StaticFlags.deftDuelist` (legacy, read nowhere) and `Script.activate` (read nowhere) are not
ported.

### 7.3 Tests in the same file

The card's TS test file becomes the `mod tests` of its script file, one `#[test]` per TS `it(...)`
(and one `mod` per TS `describe`), named after the title by §4.2. A title that cites rulings keeps
them as leading tokens: `it("R90 no legal target: …")` → `fn r90_no_legal_target_…`; two rulings →
`fn r90_r81_…`. That token is what `spec check` (§15) reads. Keep every `it`, keep the header comment
above `mod tests`.

### 7.4 The registry

`crates/cards/build.rs` (part 1) walks `src/scripts/**/*.rs`, reads each file's
`pub const ID: &str = "…";` line, and writes `$OUT_DIR/registry.rs`:

```rust
pub mod scripts {
    pub mod core { #[path = "<abs>/src/scripts/core/c001_big_d_fender.rs"] pub mod c001_big_d_fender; /* … */ }
    pub mod classic { /* … */ }
    pub mod classic_plus { /* … */ }
}
pub static REGISTRY: &[(&str, fn() -> jackioh_engine::CardScripts)] = &[ /* (ID, script), sorted by ID */ ];
```

`crates/cards/src/lib.rs`:

```rust
include!(concat!(env!("OUT_DIR"), "/registry.rs"));
pub fn register_all();                       // idempotent; builds every CardScripts once and hands them and the catalog to the engine's OnceLocks
pub fn catalog_json() -> &'static str;       // include_str!("../catalog.json")
pub fn flavour_json() -> &'static str;       // include_str!("../flavour.json")
pub fn catalog_version() -> &'static str;    // the newest entry of patches/patches.json, parsed at compile time by build.rs
```

`build.rs` fails the build, naming the path, for a card file without `ID`, an `ID` the catalog
lacks, a duplicate `ID`, or a catalog id with no file. That replaces `_generated.ts`,
`gen-registry.ts`, `missing-tests.ts` and `registry.test.ts`'s completeness checks.

### 7.5 `loc` is frozen data

`catalog.json`'s `loc` (the TS script's code-line count) is gameplay data: C+ #44, C+ #45 and Classic
#48 read it. It stays exactly as it is in `catalog.json`; nothing recomputes it from Rust.
`gen-loc.ts` and `test/loc.test.ts` are deleted. A card added after v0.3.0 gets its `loc` from
`cargo jackioh catalog loc <script path>` (non-blank, non-comment, non-`use` lines above
`#[cfg(test)]`), which only prints.

## 8. The testkit

`jackioh_engine::testkit` (feature `testkit`; every test crate enables it as a dev-dependency
feature) is `packages/cards/test/_harness.ts`, `_invariants.ts` and `_glow.ts` in Rust, plus
`pub use crate::*;` so a test can name **every** engine item (TS tests reach 165 engine values,
including `make_context`, `register_scripts`, `new_instance`, `place_on_field`, `move_to_zone`,
`settle`). Its verbs are the harness's, snake_cased, and take `serde_json::Value` wherever the TS
took an object literal, so a TS test ports by copying its literals into `json!(…)`:

| TypeScript (`_harness.ts`) | Rust (`testkit`) |
|---|---|
| `const s = scenario({ p1: {…}, p2: {…}, seed?, turn?, active?, lastBoards?, glitchBoards? })` | `let mut s = scenario(json!({ "p1": {…}, "p2": {…} }));` |
| `s.play("core-002", { targets: [...] })` | `s.play("core-002", json!({ "targets": [...] }));` |
| `s.play("core-002")` | `s.play("core-002", json!({}));` |
| `s.attack(attacker, target \| "hero")` | `s.attack(attacker, "hero")` / `s.attack(attacker, target)` |
| `s.answer(selection)` | `s.answer(json!(selection))` |
| `s.endTurn()`, `s.startTurn()` | `s.end_turn()`, `s.start_turn()` |
| `s.switchPosition(card)` | `s.switch_position(card)` |
| `s.activate(card, {…})` | `s.activate(card, json!({…}))` (sends `activatePower` when TS did) |
| `expect(() => s.play(...)).toThrow()` | `s.expect_refused(\|s\| s.play(...));` |
| `expect(() => …).toThrow(/text/)` | `s.expect_refused_with(\|s\| …, "text");` |
| `s.state`, `s.events`, `s.lastEvents` | `s.state()`, `s.events()`, `s.last_events()` |
| `s.view(p?)`, `s.unit(p, lane)`, `s.backrow(p, lane)`, `s.hand(p?)`, `s.pile(p, zone)`, `s.card(ref)`, `s.stats(card)` | same names, snake_cased; `card()` returns `&CardInstance` |
| `s.expectInZone(ref, "graveyard")` | `s.expect_in_zone(ref, "graveyard");` |
| `s.expectStats(ref, { attack: 12, maxHealth: 2 })` | `s.expect_stats(ref, json!({ "attack": 12, "maxHealth": 2 }));` |
| `s.expectEvents([...])`, `s.expectHealth(p, n)`, `s.expectMana(p, n)` | `s.expect_events(json!([...]))`, `s.expect_health(p, n)`, `s.expect_mana(p, n)` |
| `expect(x).toBe(y)`, `toEqual` | `assert_eq!(x, y);` |
| `expect(x).toBeTruthy()` | `assert!(x)` (with §4.4.6's truthiness) |
| `expect(arr).toContain(x)` | `assert!(arr.contains(&x));` |
| `expect(s).toMatch(/re/)` | `assert!(s.contains("…"))` for a literal pattern; a real regex becomes a small hand check (no regex crate) |
| `DEFAULT_SEED`, `DEFAULT_TURN` | same constants |

**Test-only registries.** TS tests swap the global registries (`registerScripts`,
`registerCatalog`) to run fixture cards. Rust keeps one `OnceLock` for production and gives the
testkit a **thread-local override**: `testkit::register_scripts(scripts)` and
`testkit::register_catalog(defs)` set it for the calling thread (each `#[test]` runs on its own
thread; `#[tokio::test]` defaults to a current-thread runtime, so an actor task spawned in it sees
it too), and `catalog::registered_catalog()`/`scripts::script_of()` consult it first, under
`#[cfg(feature = "testkit")]` only. That one `thread_local!` is the only allowed exception to §3's
`RefCell` ban (`#[allow(clippy::disallowed_types)]` on that item alone).

A card reference resolves by catalog id, §5 index or name, as in TS. `testkit::invariants::Monitor`
is `_invariants.ts`'s I1–I4 monitor (used by `cargo jackioh fuzz`). `testkit::glow` is `_glow.ts`.

## 9. The AI crate

`crates/ai/src/lib.rs` re-exports what `packages/ai/src/index.ts` exported (by §4.2), except the R645
emote personas, which are presentation and move to `apps/web/src/practice/personas.ts` (part 21).
Fixed here:

```rust
pub fn decide(state: &GameState, seat: PlayerId, options: &mut AiOptions) -> Option<Decision>;
pub struct AiOptions<'a> {
    pub rng: Rng,                                   // the caller reads rng.cursor() back afterwards
    pub budget: SearchBudget,                       // default AI_BUDGET
    pub should_stop: Option<&'a dyn Fn() -> bool>,  // TS shouldStop; the browser's clock, never read by this crate
}
pub fn build_ai_deck(rng: &mut Rng, size: i32, options: &AiDeckOptions) -> Vec<String>;
pub fn redact(state: &GameState, seat: PlayerId) -> GameState;
pub fn ai_to_act(state: &GameState, seat: PlayerId) -> bool;
pub fn random_action(state: &GameState, seat: PlayerId, rng: &mut Rng) -> Option<ActionBody>;   // §10.7's policy, via subsystems::choose_action
pub fn greedy_action(state: &GameState, seat: PlayerId, rng: &mut Rng) -> Option<ActionBody>;
pub fn play_match(config: &MatchConfig, hooks: &mut MatchHooks) -> MatchRecord;
pub const SHADOW_BAN: &[(&str, &str)];              // shadow_ban.rs: (defId, reason), the 11 TS entries, sorted by id
pub const SHADOW_WATCH: &[(&str, &str)];            // empty, as TS
```

## 10. The WASM bindings and the web client's seams

### 10.1 Bindings (`crates/wasm/src/lib.rs`, part 21)

JSON strings in and out; nothing else crosses. The state is the `GameState` JSON object, which TS
keeps behind its opaque `EngineState` brand.

```rust
#[wasm_bindgen] pub fn init();                                                     // panic hook + jackioh_cards::register_all()
#[wasm_bindgen] pub fn catalog() -> String;                                        // CardDefs JSON
#[wasm_bindgen] pub fn catalog_version() -> String;
#[wasm_bindgen] pub fn create_game(args_json: &str) -> String;                     // GameState
#[wasm_bindgen] pub fn begin_game(state_json: &str) -> String;                     // ReduceResult
#[wasm_bindgen] pub fn reduce(state_json: &str, action_json: &str) -> String;      // ReduceResult
#[wasm_bindgen] pub fn legal_actions(state_json: &str, player: &str) -> String;    // ActionBody[]
#[wasm_bindgen] pub fn view_for(state_json: &str, player: &str) -> String;         // PlayerView
#[wasm_bindgen] pub fn seat_to_act(state_json: &str) -> String;                    // "p1" | "p2" | ""
#[wasm_bindgen] pub fn hash_state(state_json: &str) -> String;
#[wasm_bindgen] pub fn fold(args_json: &str) -> String;                            // { state, errors }
#[wasm_bindgen] pub fn last_board_for(state_json: &str, seat: &str) -> String;
#[wasm_bindgen] pub fn seat_played_by(state_json: &str, home: &str) -> String;
#[wasm_bindgen] pub fn ai_to_act(state_json: &str, seat: &str) -> bool;
#[wasm_bindgen] pub fn ai_decide(state_json: &str, seat: &str, options_json: &str, deadline_ms: f64) -> String;
    // options: { rngSeed, rngCursor, budget? }; answers { decision: Decision | null, rngCursor };
    // should_stop = js_sys::Date::now() >= deadline_ms (deadline_ms <= 0: no clock)
#[wasm_bindgen] pub fn build_ai_deck(options_json: &str) -> String;
    // options: { rngSeed, rngCursor, size, banned?, manaCap?, theme?, include?, boost? }; answers { deck, rngCursor }
#[wasm_bindgen] pub fn validator(call: &str, input_json: &str) -> String;
    // call ∈ validateDeck | validateTrio | validateLoadout | checkDeckDraft | checkTrioDraft | checkImportRoom | trioConflicts | normalizeName;
    // the input and output are those TS functions' argument and result, as JSON
#[wasm_bindgen] pub fn constants() -> String;                                      // { AI_BUDGET, AI_GATE_BUDGET, SHADOW_BAN_IDS } for practice, the tutorial harness and their tests
#[wasm_bindgen] pub fn find_instance(state_json: &str, instance_id: &str) -> String;  // CardInstance JSON or "null" (state.rs's find_instance)
#[wasm_bindgen] pub fn choose_action(state_json: &str, seat: &str, rng_seed: &str, rng_cursor: f64) -> String;
    // §10.7's random policy (subsystems::ai_policy::choose_action); answers { action: ActionBody | null, rngCursor }
#[wasm_bindgen] pub fn engine_tables() -> String;
    // { chaosEffects: [{label}], chaosPlusEffects: [{label}], heroPowerNames: [..], heroPowers: [{name, x, title, radiantTitle, label, radiantLabel}] }
```

### 10.2 Build

`scripts/build-wasm.sh` (part 21): installs nothing that is present; else `rustup target add
wasm32-unknown-unknown` and downloads `wasm-bindgen-cli` 0.2.129's release tarball into
`.cache/bin/`; then `cargo build -p jackioh-wasm --release --target wasm32-unknown-unknown` and
`wasm-bindgen --target web --out-dir apps/web/src/wasm/pkg target/wasm32-unknown-unknown/release/jackioh_wasm.wasm`.
`apps/web/package.json`'s `predev`, `prebuild`, `prebuild:e2e` and `pretest` run it. On Vercel,
`vercel.json`'s `installCommand` installs rustup (minimal profile) before `pnpm install`.

### 10.3 The TS wrapper (`apps/web/src/wasm/index.ts`, part 21)

`export async function loadWasm(): Promise<void>` (idempotent; `init()` + `pkg.init()`),
`export function loadWasmSync(bytes: BufferSource): void` (for jsdom tests, from
`apps/web/src/test/setup.ts`), and one typed function per binding that parses and stringifies, named
as the TS engine named it (`createGame`, `reduce`, `legalActions`, `viewFor`, `hashState`, `fold`,
`lastBoardFor`, `seatPlayedBy`, `aiToAct`, `decide`, `buildAiDeck`, `registeredCatalog`, `findInstance`, …).
`apps/web/src/main.tsx` awaits `loadWasm()` before the first render (the deck builder calls the
validator synchronously); `practice.worker.ts` awaits it before handling a message.

### 10.4 `apps/web/src/wire/` and the aliases

So that the ~150 web files that import `@jackioh/shared`, `@jackioh/engine`, `@jackioh/engine/config`,
`@jackioh/validator`, `@jackioh/cards`, `@jackioh/ai` and `@jackioh/cards/*.json` need no edit (`registerAll()` calls aside: there is nothing to register, so part 21 deletes them), `apps/web/vite.config.ts`,
`apps/web/vitest.config.ts`, `apps/web/tsconfig.json`, `e2e/tsconfig.json` and the component-test
config alias them:

| Import specifier | Resolves to |
|---|---|
| `@jackioh/shared` | `apps/web/src/wire/index.ts`: `export * from "./generated/…"` + the hand-kept helpers |
| `@jackioh/engine/config` | `apps/web/src/wire/engineConfig.ts` (generated, §5.1) |
| `@jackioh/engine` | `apps/web/src/wire/engine.ts`: the `../wasm` functions under their TS names, `type GameState` (generated), `createRng`/`type Rng` from `./rng.ts` (a copy of `packages/engine/src/rng.ts` whose object also carries `readonly seed`), and `subsystems` = `{ CHAOS_EFFECTS, CHAOS_PLUS_EFFECTS, HERO_POWER_NAMES, HERO_POWERS }` from `engine_tables()` plus `chooseAction(state, seat, rng)` over `choose_action`, which then draws the TS `rng` forward to the returned cursor |
| `@jackioh/validator` | `apps/web/src/wire/validator.ts` (types + WASM-backed functions) |
| `@jackioh/cards/catalog.json`, `@jackioh/cards/flavour.json` | `crates/cards/catalog.json`, `crates/cards/flavour.json` |
| `@jackioh/cards` | `apps/web/src/wire/cards.ts` (`CATALOG`, `CATALOG_VERSION`, `type CardFlavour`) |
| `@jackioh/ai`, `@jackioh/ai/config` | `apps/web/src/wire/ai.ts` (`decide`, `aiToAct`, `buildAiDeck` over WASM; `AI_BUDGET`, `AI_GATE_BUDGET`, `SHADOW_BAN_IDS`, `type SearchBudget`, `AI_EMOTE`, `type AiEmote`, `type EmotePersona`, `createEmotePersona`, `pickPersona` re-exported from `../practice/personas.ts`) |
| `../../server/src/config` (relative, 23 web and 10 e2e files) | replaced by `@jackioh/server-config` → `apps/web/src/wire/serverConfig.ts` (generated, §5.1); part 21 rewrites those import lines |

The hand-kept helpers are the runtime values the web imports from `@jackioh/shared` today
(`keywordKey`, `fillParams`, `opponentOf`, `hasKeyword`, `SHIPPED_SETS`, `PARAM_PLACEHOLDER`,
`readCodeInput`, `normalizeCodeText`, `findCodeInText`, `formattedCaret`, `isCodeSeparator`,
`excludedCharacters`, `portraitOrDefault`, `pickPortrait`, `isPortraitId`, `PORTRAITS`,
`PORTRAIT_IDS`, `DEFAULT_PORTRAIT`, `isEmoteId`, `isVoiceEmote`, `emoteGate`, `EMOJI_EMOTE_IDS`,
`VOICE_EMOTE_IDS`, `parseAim`, `aimKey`, and `stats.ts`'s formatters): part 21 copies their TS
sources from `packages/shared/src/` into `apps/web/src/wire/{catalog,codes,emotes,aim,stats}.ts`
unchanged. The Rust server needs the same functions; part 5 ports them to `crates/engine/src/wire/`.
Both sides test against one fixture, `crates/engine/tests/fixtures/code-input-cases.json` (from
`packages/shared/test/fixtures/code-input-cases.ts`, part 5), so they cannot drift.

## 11. The server

The Rust server serves the **same** routes, statuses, headers, JSON bodies, WebSocket frames, close
codes and database rows as `apps/server`, with the deltas listed below and nothing else. Research C
§2–§4 is the contract; parts 18–20 copy the tables they need.

### 11.1 Module tree (`crates/server/src/`; `lib.rs`, `main.rs` and every `mod.rs` are part 1's)

```
main.rs        clap: serve (default) | release (migrate, seed-catalog, serve) | migrate | seed-catalog | mint-code | seed-accounts | season-start | stats-cards | stats-import
lib.rs         pub mod env; pub mod config; pub mod app; pub mod auth; pub mod api; pub mod actor; pub mod ranked; pub mod db; pub mod cli;
env.rs         ← src/env.ts (part 18): every variable, same names, defaults and refusals (E2E with production refused)
config.rs      ← src/config.ts (part 18): every constant, server and client-only; ROOM_CODE_TTL_SECONDS = 900 added (rule 9)
app.rs         ← src/index.ts (part 18): App, the route table, boot, background loops
auth.rs        (part 18) the Auth enum (from src/api/auth.ts's provider half and src/api/e2e.ts's fixture auth)
api/<x>.rs     ← src/api/<x>.ts (parts 18 and 19, by PORT-MAP)
actor/<x>.rs   ← src/match/<x>.ts (part 19); actor.ts → match_actor.rs (clippy's module_inception)
ranked/<x>.rs  ← src/ranked/<x>.ts (part 18), pure
db/store.rs    (part 20) the Db and Tx enums and every store method (← src/api/ports.ts's Store half)
db/pg.rs       (part 20) ← src/db/store.ts: one free async fn per store method over sqlx
db/fake.rs     (part 20) ← src/api/memory-stores.ts + e2e-store.ts: one free fn per store method over FakeData
db/migrate.rs  (part 20) ← src/db/migrate.ts
cli/<x>.rs     (part 20) ← src/db/{seed-catalog,mint-code,seed-accounts,season-start,card-stats,import-dev-records}.ts
```

### 11.2 The shared shapes (frozen)

```rust
// app.rs (part 18)
pub struct App {
    pub env: env::Env,
    pub db: db::Db,
    pub auth: auth::Auth,
    pub matches: actor::registry::Registry,          // part 19
    pub limiter: api::http::RateLimiter,             // part 18
    pub catalog: api::catalog::Catalog,              // part 18: defs JSON, version, deployed commit
}
pub async fn build(env: env::Env) -> anyhow::Result<std::sync::Arc<App>>;     // PgStore or, under E2E=1, FakeStore + E2eAuth + fixtures (R144)
pub fn router(app: std::sync::Arc<App>) -> axum::Router;                       // /ws/match + one fallback that runs api::http::dispatch
pub async fn serve(env: env::Env) -> anyhow::Result<()>;                       // build, spawn loops, listen on $PORT

// api/http.rs (part 18): TS's own router, so its order and its 404-for-a-wrong-method survive
pub struct Req { pub caller: Option<Caller>, pub params: indexmap::IndexMap<String, String>,
                 pub query: indexmap::IndexMap<String, String>, pub body: serde_json::Value, pub address: String,
                 pub headers: axum::http::HeaderMap }
pub struct Caller { pub profile: db::Profile, pub user: auth::AuthUser }
pub enum AuthLevel { None, User, Active }
pub type ApiResult = Result<axum::response::Response, ApiError>;
pub struct ApiError { pub code: ApiErrorCode, pub message: String, pub details: Option<serde_json::Value>, pub retry_after_ms: Option<i64> }
pub fn json(status: u16, body: serde_json::Value) -> axum::response::Response;  // content-type and cache-control as TS
// every handler, in parts 18 and 19, has exactly this shape and the TS handler's name snake_cased:
//     pub async fn <name>(app: &App, req: Req) -> ApiResult
// app.rs's ROUTES lists (method, path, AuthLevel, handler) in TS's allRoutes() order, wrapping each
// handler with a macro `h!(api::queue::enqueue)` into a boxed-future fn pointer.

// auth.rs (part 18)
pub enum Auth { Supabase(SupabaseAuth), E2e(E2eAuth) }
impl Auth {
    pub async fn verify(&self, token: &str) -> Result<AuthUser, AuthError>;
    pub async fn sign_in(&self, email: &str, password: &str) -> Result<Session, AuthError>;  // E2e only; Supabase answers unavailable
    pub async fn delete_user(&self, user_id: &str) -> Result<(), AuthError>;
}

// db/store.rs (part 20): no trait, two variants, every method on Tx
pub enum Db { Pg(sqlx::PgPool), Fake(std::sync::Arc<tokio::sync::Mutex<fake::FakeData>>) }
pub enum Tx<'a> { Pg(sqlx::Transaction<'a, sqlx::Postgres>), Fake(fake::FakeTx<'a>) }
impl Db {
    pub async fn begin(&self, claim_sub: Option<&str>) -> Result<Tx<'_>, StoreError>;  // Pg: BEGIN; SET LOCAL role service_role; request.jwt.claim.sub (store.ts's rule 2)
}
impl Tx<'_> {
    pub async fn commit(self) -> Result<(), StoreError>;
    // one method per TS Store method, named <substore>_<method> snake_cased, TS's argument order:
    //   profiles_get_by_id, profiles_get_by_user_id, profiles_get_many, profiles_create, profiles_set_status,
    //   profiles_set_glicko, profiles_set_display_name, profiles_set_in_match, profiles_remove,
    //   codes_insert, codes_find_by_hash, codes_claim, codes_log_attempt, …,
    //   matches_create, matches_get, matches_append_actions, matches_actions, matches_set_clocks, matches_finish,
    //   matches_live, matches_mode_of, matches_discard_open, matches_forget_voided, …,
    //   redeem, purge_expired   (the root methods keep their names)
    // each body: match self { Tx::Pg(t) => pg::<name>(t, …).await, Tx::Fake(f) => fake::<name>(f, …) }
}
// TS's store.tx(async (t) => …) becomes: let mut tx = app.db.begin(sub).await?; …; tx.commit().await?;
// TS's ProfileStore.setRating (no caller) is not ported.

// actor/registry.rs (part 19)
pub struct Registry { /* live actors by match id; finished actors stay until the reaper or a restart */ }
impl Registry {
    pub fn new() -> Registry;
    pub async fn start(&self, app: &std::sync::Arc<App>, input: StartMatchInput) -> Result<(), api::http::ApiError>;
    pub async fn attach(&self, app: &std::sync::Arc<App>, match_id: &str, profile_id: &str, socket: actor::ws_server::Socket) -> Result<(), AttachError>;
    pub fn has(&self, match_id: &str) -> bool;
    pub async fn stop(&self, match_id: &str);
    pub fn presence_of(&self, match_id: &str) -> Option<PerPlayer<bool>>;
}
// actor/ws_server.rs (part 19)
pub async fn handle(app: std::sync::Arc<App>, req: axum::extract::Request) -> axum::response::Response;  // the /ws/match upgrade
// background loops app.rs spawns (names fixed):
//   api::queue::run_matchmaker(app) (3 s), api::series::run_sweeper(app) (5 s),
//   api::results::run_reaper(app) (30 s), api::retention::run_purge(app) (boot + hourly),
//   api::ranked::open_season(&app) (once at boot)
```

Tests (`crates/server/tests/`): `support/deps.rs` (part 18) exposes `pub async fn test_app() -> Arc<App>`
(FakeStore, E2eAuth, the E2E fixtures) and `pub async fn call(app, method, path, token, body) ->
(u16, HeaderMap, Value)` (a `tower::ServiceExt::oneshot` over `router`). `support/engine.rs` (part
19) ports `test/fakes/engine.ts`'s test cards (`test-prompt-self`, `test-prompt-enemy`,
`test-lethal`) as real engine scripts installed with the testkit override (§8); the server links
the real engine and has no `Engine` trait. `support/socket.rs` (part 19) is a fake socket over an
mpsc channel. Time: `tokio::time::pause()` and `advance()` replace the TS manual timers.

### 11.3 Fixed decisions

- **No traits** on the server: `Db`, `Tx` and `Auth` are enums with one variant per implementation
  (two each). TS's other ports go: `Timers` → `tokio::time`, `Logger` → `tracing` JSON lines with the
  same `event` names (`api.forwarded_for`, `server.listening`, `match.fold.errors`, …), `Ids` →
  `uuid` + `getrandom`, `Hashes` → functions in `api/crypto.rs`, `EnginePort` → direct calls into
  `jackioh_engine`, `loadStore`/`STORE_EXPORT_CANDIDATES`/`StoreUnavailableError` → nothing.
- **Router**: TS's matcher, in TS's order; a wrong method on a known path answers **404**
  `not_found` "method not allowed for this path"; preflight `OPTIONS` answered by the CORS layer
  first (`api/cors.rs`); bodies over 65,536 bytes are 400; a body that is not a JSON object is 400.
- **WebSocket** at `/ws/match` on the same port. The token comes from `?token=` only (what the browser
  and the e2e Node player send); the unused `Authorization` and `Sec-WebSocket-Protocol` token paths
  are dropped, and `jackioh.v1` is still echoed when offered. A frame over 65,536 bytes closes with
  1009 (one check). `joinRoom` frames are answered `malformed`. The action nonce is read only inside
  `action` (the client's form). The client's handler for a `legal` frame the server never sends is
  deleted (part 21).
- **Migrations**: `db/migrate.rs` keeps the ledger `app.migrations(filename, applied_at, checksum)`,
  the checksum (FNV-1a 32 over UTF-16 code units, 8 hex, the same function as §5.2's), `REWRITTEN =
  {"0013_retention_purge.sql": ["16b93e4d"]}` and `pg_advisory_xact_lock(0x6a61636b)` per file;
  the files are embedded with `include_str!`. `tests/store/migrations_pinned.rs` pins every checksum.
- **The catalog version** is `jackioh_cards::catalog_version()`, compiled in from
  `crates/cards/patches/patches.json`. `CATALOG_VERSION` in the environment is still read and must
  equal it, or the server refuses to boot (one honest check instead of the start-command derivation).
  `GET /api/catalog` keeps `x-deployed-commit` from `RENDER_GIT_COMMIT`.
- **Routes dropped** (unused): `POST /api/auth/signup` (503 in production), `GET /api/catalog/:version`.
  `POST /api/auth/signin` stays under `E2E=1` only. The legacy queue body (`deckIndex`, no `mode`)
  goes; `e2e/cypress/e2e/99-online-smoke.cy.ts` moves to `{ mode: "bo1", deckId }` (part 21).
- **Kept on purpose**: E2E mode (fixture tokens and codes are a contract with `e2e/support/config.ts`),
  rematch offers held in memory, finished actors kept in memory, the `vanilla` portrait default for
  old rows, `matches.mode` fallbacks for old rows, the retired `loadouts` tables.
- **Deploy**: `crates/server/Dockerfile` (multi-stage: `rust:1.97-slim` builds
  `cargo build --release -p jackioh-server`; `debian:bookworm-slim` with `ca-certificates` runs
  `/app/jackioh-server release`), `render.yaml` `runtime: docker`, the same service name, region,
  plan, health check (`/api/catalog`) and env contract minus the Node-only keys (`NODE_VERSION`,
  `CYPRESS_INSTALL_BINARY`). `NODE_ENV` is still read (E2E refusal).

## 12. The `jackioh` CLI

`crates/tools`, binary `jackioh`, run as `cargo jackioh <cmd>` (the alias of §1). `main.rs` is part
1's: a clap `enum Command` with one variant per row below, each holding `<module>::Args` and calling
`<module>::run(args) -> anyhow::Result<()>`; each module (owned by the part named) defines its own
`#[derive(clap::Args)] pub struct Args` and `pub fn run`. Exit code 0 on success, 1 on a failed check,
2 on bad usage. Output to stdout, nothing interactive.

| Command | Module | Part | Replaces | Does |
|---|---|---|---|---|
| `fuzz [--from N] [--seeds N] [--handicap]` | `fuzz.rs` | 22 | `pnpm fuzz`, `fuzz.test.ts`, `fuzz-handicap.test.ts` | random-policy games exactly as `fuzz.test.ts` seeds and deals them; I1–I4 each step; `fold(args, log)` hash equals the live hash; rayon-parallel |
| `replay` | `replay.rs` | 22 | `e2e/support/tasks/replay-runner.ts`'s TS fold | reads `{seed, decks, log, handicaps?, dealt?, lastBoards?, glitchBoards?}` on stdin, prints `{"hash", "errors"}` |
| `trace <matchup> <n>` | `trace.rs` | 22 | `packages/ai/scripts/trace.ts` | prints one gate game turn by turn |
| `catalog check` | `catalog.rs` | 22 | `validate-catalog.ts`, the catalog/registry/references/params/radiant-standard tests | data checks |
| `catalog loc <path>` | `catalog.rs` | 22 | `gen-loc.ts` | §7.5 |
| `catalog-version` | `catalog.rs` | 22 | `scripts/catalog-version.mjs` | prints the newest version in `patches.json` |
| `patches <version> <date> "<title>"`, `patches check`, `patches ship` | `patches.rs` | 22 | `packages/cards/scripts/{patches,patch,patches-io,versions,naming}.ts` | same files, same formats |
| `gate [--full] [--shard k/K] [--out dir]`, `gate merge <dir>` | `gate.rs` | 22 | `pnpm ai:gate`, `gate-*.test.ts`, `gate-merge.ts` | the three matchups against `gateNeeded`, plus the perf gate |
| `sweep [ids…] [--pass2 …] [--report …]` | `sweep.rs` | 22 | `pnpm ai:sweep` | R186's two-pass sweep; prints `SHADOW_BAN` rows |
| `stats [--games N] [--patch v] [--out file]` | `stats.rs` | 22 | `pnpm ai:stats` | R378's development run |
| `golden check`, `golden bless [--seeds N]` | `golden.rs` | 23 | — | §13 |
| `spec check`, `spec index` | `spec.rs` | 28 | `pnpm rulings:coverage`, `rulings.test.ts` | §15 |
| `arena …`, `agent`, `promote …` | `arena.rs`, `agent.rs`, `promote.rs` | 29 | `ladder/`, `duel.ts` | §14 |

## 13. Golden traces

The oracle the rewrite is held to, recorded from the TypeScript engine **before** any Rust runs
(part 23), and replayed by the Rust engine.

### 13.1 Recording (TS, `scripts/golden/record.ts`, run with `pnpm exec tsx`)

For seed `k` in `1..=200`: deal and seed the game **exactly as `packages/cards/test/fuzz.test.ts`
does for fuzz seed `k`** (copy its deck builder, `FUZZ_POOL` and `decksForSeed`; never import the
test file, whose top-level `describe` needs the vitest runner: pool = every catalog id whose def is
not a token (`token !== true` and no `Token` tag) sorted, decks = `createRng("jackioh-fuzz-decks-<k>").shuffle(pool)`
first and next 20, game seed `"jackioh-fuzz-<k>"`), and pick actions with
`subsystems.chooseAction(state, actor, createRng("jackioh-fuzz-policy-<k>"))` (mulligan order from
`createRng("jackioh-fuzz-policy-order-<k>")`), nonce `golden-<n>`, as the fuzz loop does. Seeds
`201..=240`: the same with `fuzz-handicap.test.ts`'s rotating handicap. Stop at game over or at step
3,000.

### 13.2 What is hashed

With §5.2's `canonical` and `fnv1a32_utf16` (TS: copy `replay.ts`'s `canonical` and its FNV loop):

- `s`: `hashState(state)` after the action (the state hash itself);
- `v`: `[fnv(canonical(viewFor(state, "p1"))), fnv(canonical(viewFor(state, "p2")))]`;
- `e`: `fnv(canonical(events))`;
- `l`: `fnv` of the actor's legal actions **as a set**: each action's `canonical` string, sorted,
  joined by `\n`. Order is not pinned (the Rust random policy and AI need not match TS's order);
  membership is.

### 13.3 File format

`crates/engine/tests/golden/games.jsonl`, one game per line, written with keys in this order:

```json
{"v":1,"seed":"jackioh-fuzz-1","args":{"seed":"jackioh-fuzz-1","decks":[[…],[…]],"handicaps":null},
 "begin":{"s":"…","v":["…","…"],"e":"…"},
 "steps":[{"a":{…Action with playerId and nonce…},"l":"…","s":"…","v":["…","…"],"e":"…"}],
 "end":{"winner":"p1","reason":"hero-death","steps":153}}
```

Rust (`crates/engine/tests/golden.rs`, and `cargo jackioh golden check` over the same file): for each
line, `create_game(args)`, `begin_game`, check `begin`; then for each step check `l` for the actor
(`a.playerId`), apply `a` with `reduce`, check `s`, `v`, `e`. On the first mismatch, fail naming seed,
step, which hash, the action, and write the Rust side's canonical JSON to
`target/golden-diff/<seed>-<step>-<which>.json`; `scripts/golden/record.ts --seed k --dump-step n`
writes TS's side next to it (it stays runnable until part 37). `crates/engine/tests/golden/01-hotseat-full-game.json`
(copied from `packages/cards/test/fixtures/`) must fold to `"a798906b"`.

## 14. The training arena and the promotion gate

### 14.1 The agent protocol

`cargo jackioh agent` reads JSON lines on stdin and answers each with one JSON line on stdout:

```
→ {"op":"info"}
← {"generation":7,"lane":"improve","shadowBan":["core-042", …]}
→ {"op":"decide","state":<redact(state, seat) JSON>,"seat":"p1","rngSeed":"…","rngCursor":0}
← {"action":<ActionBody JSON>,"rngCursor":3}
→ {"op":"quit"}
```

The referee holds the true state and sends each agent only `redact(state, seat)` (R185). Both binaries
must share an engine version: an `ai/*` branch differs from `main` only under `crates/ai/` and
`training/history/` (CI enforces it, part 30).

### 14.2 `arena` and `promote`

```
cargo jackioh arena --a self|random|bin:<path> --b self|random|bin:<path> --games N --seed <base> [--out <dir>]
cargo jackioh promote --lane improve|unban --parent-bin <path to main's jackioh> [--dry-run] [--verify]
```

`arena` writes one `GameRecord` JSONL line per game (`packages/shared/src/stats.ts`'s format, R376;
`source: "dev"`, `mode: "random"`, `pilots` naming the two agents) to `--out` (default
`$JACKIOH_TRAINING_OUT/<date>.jsonl`, else stdout).

`promote` plays `TRAINING_GAMES` (100) games against `random` (SPEC §10.7's policy,
`jackioh_ai::random_action`) and `TRAINING_GAMES` against the parent, seats alternating (candidate
`p1` on odd games), seeds `"<lane>:<tree>:<k>"` where `<tree>` is `git rev-parse HEAD:crates/ai/src`
(so committing `generation.json` does not change the seeds), the same seeds against both opponents,
each seat's deck `build_ai_deck` over all three sets minus **its own** AI's shadow-ban list, no
handicap (Easy both). It promotes when:

| Lane | vs random | vs parent | shadow bans |
|---|---|---|---|
| `improve` | ≥ `TRAINING_IMPROVE.vs_random` = 90 | ≥ `TRAINING_IMPROVE.vs_parent` = 85 | — |
| `unban` | ≥ `TRAINING_UNBAN.vs_random` = 90 | ≥ `TRAINING_UNBAN.vs_parent` = 75 | strictly fewer than the parent's |

A draw is not a win. `TRAINING_GAMES`, `TRAINING_IMPROVE` and `TRAINING_UNBAN` are constants in
`crates/engine/src/config.rs` next to `AI_GATE` (CLAUDE.md rule 9; part 1 writes them). Without
`--dry-run` or `--verify`, a pass writes `crates/ai/generation.json` and appends the same object to
`training/history/<lane>.jsonl`:

```json
{"generation":8,"lane":"improve","parent":"<main commit>","tree":"<crates/ai/src tree>","vsRandom":"93/100","vsParent":"87/100","shadowBan":11,"parentShadowBan":11,"date":"2026-11-02"}
```

`--verify` (CI) recomputes and compares with the branch's `generation.json`. Exit 0 on a pass, 1 on
a failed gate.

### 14.3 `crates/ai/generation.json`

The main AI's record. Generation 0 is the port of the last successful TS AI (`packages/ai` at
`91cc43c`: 94/100, 35/50, 47/50), written by part 29 with `"generation": 0, "lane": "port"` and
filled with its first Rust measurement by part 40.

## 15. The spec graph

`SPEC.md` becomes `spec/` (part 28), Obsidian-style: atomic notes that link by `[[id]]`.

```
spec/README.md            the map: one line per section note, the note format, how to add a ruling
spec/01-overview.md … spec/10-engine-guide.md      one note per SPEC § (its ### subsections inside)
spec/11-rulings.md        §11's provenance paragraphs (the prose above the table), verbatim
spec/rulings/R0001.md … spec/rulings/R0762.md      one note per §11 row (4-digit, zero-padded; 561 files)
spec/INDEX.md             generated by `cargo jackioh spec index`: id | title | note | proven in
```

A ruling note:

```markdown
---
id: R195
title: When a card glows yellow (`conditionActive`)
cards: "§10.8, §10.9, #10, #53, #68, #71, #93, C #22, C #36, C #40"
proven_in:
  - crates/engine/tests/rules/condition_active.rs
  - crates/cards/tests/cross/condition_active.rs
---
<the row's ruling cell, verbatim, with every R-number written [[R113]] and every § written [[§10.3]]>
```

`cargo jackioh spec check` replaces `rulings.test.ts`'s index and `rulings-coverage.ts`:

1. every note's `id` matches its file name, and ids are unique;
2. every `proven_in` path exists and holds a proof of the id: a Rust file with a `fn` or `mod`
   whose name has the `_`-delimited token `r<n>` (`fn r195_glows_…`, `fn r90_r81_…`), or a TS file
   (`apps/web/`, `e2e/`: 63 rulings are proven only in web tests) with `it(` or `describe(` whose
   title contains `R<n>` as a word;
3. every `r<n>` test token and every `R<n>` cited in `crates/`, `apps/web/src/` and `e2e/` (comments
   included, recursively) has a note;
4. every `[[…]]` link resolves to a note or section.

It reads ids, file names and test names, never prose (#133).

## 16. Fullsend notes and the fixed assumption keys

Every part writes `.fullsend/notes/part-<NN>.md` (the fullsend builder template: `BUILDS-RUN`,
`FILES`, `SURFACE`, `DEPENDS-ON`, `GAPS`) and `.fullsend/notes/part-<NN>.assumptions`, and commits
both with its last push. `.fullsend/` lives on `staging` until part 37 deletes it.

The keys every part answers. The values below are this file's decisions; copy them, and change one
only if you actually did something else (part 31's diff then finds you):

```
id.instance: String (TS "c<n>")
num.game: i32
num.cursor: u32
map.ordered: IndexMap
bag.data: IndexMap<String, serde_json::Value>
error.style: Result<_, EngineError> with TS's message; panic only on invariants
null.optional: Option + skip_serializing_if
null.nullable: Option, serialised null
case.json: camelCase via serde rename_all
case.rust: snake_case
state.mut: &mut GameState writes, &GameState reads, &mut EngineSink for TS sink mutators
rng.source: Rng::new(&state.seed, state.rng_cursor) and write the cursor back
hook.style: Arc<dyn Fn + Send + Sync> via hook(); no fn pointers, no Box
registry.style: OnceLock set once by register_all; fused scripts built on lookup from state
async.style: tokio in crates/server only; none in pure crates
log.style: tracing JSON to stdout, server only
```

## 17. Amendments from the reconcile (part 31, Wave 2)

Holes the Wave 2 reconcilers found, each settled once and applied to the code (`.fullsend/notes/spec-gaps.md` has the reasoning and `.fullsend/notes/reconcile-decisions.md` the per-crate decisions). Where a section above says otherwise, this section wins.

- **§2, `jsonwebtoken`:** the workspace entry carries `features = ["rust_crypto"]` (without a provider 11.1 panics on the first verify).
- **§4.3, three more TS shapes:** an AI node budget or count (`SearchBudget`, `SearchStats`, `NodeCounter`, `find_lethal`'s limit, `MatchRecord.nodes`) is `usize`, the AI's config counts stay `i32`; a store's counts, caps, positions, sequence numbers and epoch ms are `i64` at the `Tx` boundary; TS's optional nullable `x?: T | null` is `Option<Option<T>>` with `#[serde(default, skip_serializing_if = "Option::is_none", with = "absent_or_null")]`.
- **§5.1 / §10.1, the web's engine constants:** `engineConfig.ts` carries 13, `LIBRARY_CAP` included.
- **§6.1, `view_for`'s clock:** `view_for_with_clock(state: &GameState, player: PlayerId, clock_ms: Option<i32>) -> PlayerView` sits beside `view_for` (TS's third argument, R79; the server's actor passes it).
- **§6.5, `EngineSink`:** two more fields, `frontier: triggers::FrontierSlot<'a>` (which of an action's events are dispatched; TS used object identity) and `owed_behind: Option<IndexSet<String>>` (R117). `reborrow` shares `frontier` and copies the rest.
- **§6.6, closures:** `TriggerWhen = Arc<dyn Fn(&mut EffectContext, &GameEvent) -> bool>` (Classic+ #74 writes while it declines, R99). In effect arguments, a field that builds a list or a query reads `&mut EffectContext` (`ForEachCardArgs.cards`, `CastEachArgs.cards`, `DrawWhileArgs.more`, `DiscoverFromCatalogArgs.query_fn`, `CastNewDef::Read`, `CastRandomQuery::Read`, `CastRandomCount::Read`); a filter reads `&EffectContext` (`ChooseTargetWhereArgs.where_`, `ChooseFromHandArgs.where_`); `forEachCard`/`castEach` answer card ids.
- **§7.1, the prelude:** it names `catalog`'s items one by one and leaves out `register_catalog`, so a card test's two globs (`super::*`, `testkit::*`) do not collide on it.
- **§8, the testkit:** `s.state_mut() -> &mut GameState` (TS assigned `g.state.…`) and `s.card_mut(ref) -> &mut CardInstance` (TS wrote through `s.card(…)`'s live object). Two test seams exist under `#[cfg(feature = "testkit")]` only, beside the registry override: `register_work_handler` (a test-made work hook, TS `registerWorkHandler`) and `mock_brittle_tick` / `mock_animate_at_turn_start` / `mock_return_at_cleanup` (TS `vi.mock` of the three turn stages). In the cards crate, one registering `scenario()` per test binary (`crate::scenario`, `cross::scenario`) and the JSON helpers `js`, `matches_object`, `merged`, `unit_or_blank`.
- **§9, the AI's counter:** `NodeCounter` is a trait with `&self` methods whose tallies live in `Cell` (`clippy.toml` bans `RefCell` only); node-spending functions take `&dyn NodeCounter`.
- **§10.1, the bindings:** each returns `Result<String, JsError>` (`ai_to_act`: `Result<bool, JsError>`), the same JS type, throwing where TS threw. `validator("checkDeckDraft", …)` takes `{ name, cards, deckable: string[], portrait?, portraitKnown?, nameMaxLength }` (its TS argument holds two predicates; the binding rebuilds them).
- **§11.2, handlers:** `pub async fn <name>(app: &Arc<App>, req: Req) -> ApiResult` (a handler that starts a match needs the `Arc` for `Registry::start`); `app::Handler`, `h!` and `api::http::dispatch` follow.
- **§14.2, the arena's records:** `pilots` are both `"ai"` (R376's `Pilot` allows only `human`/`ai`); the two agents are named in the record's `id`, `dev:<patch>:arena:<a>-vs-<b>:<seed>`.
