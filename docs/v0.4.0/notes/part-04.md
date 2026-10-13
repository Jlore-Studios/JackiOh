# v0.4.0 Part 4: The Luau Host and Pilot Cards

Notes for Issue #446 (v0.4.0 part 4 of 29): `crates/luau`, the sandbox, integer numbers, a fresh module per call, `crates/cards` integration via `build.rs`, and three pilot cards.

## 1. Luau Host & Dependencies

- **mlua**: Pinned exactly to `=0.12.2` with `default-features = false, features = ["luau", "serde"]`.
- **Luau version**: Built by `luau0-src 0.22.0+luau740`, corresponding to Luau `0.740`.
- **Crate edges**: `jackioh-luau` is a dependency and build-dependency of `jackioh-cards`.
- **Compiler / Sandbox**:
  - Sandboxed environment with safe standard libraries (`string`, `table`, `math`, `bit32`, `utf8`).
  - Dangerous builtins (`print`, `math.random`, `os`, `debug`, etc.) and non-deterministic iterators (`pairs`, `next`) disabled or linted.
  - Optimization level set to `LUAU_OPTIMIZATION_LEVEL = 1` with `sandbox::DISABLED_BUILTINS` to prevent fastcall bypasses.
  - Boundary enforced via `numbers::to_json`: integers strictly bounded within `i32` range (L6).

## 2. Card Registry & Build Script

- `crates/cards/build.rs` discovers cards under `src/scripts/<set>/`:
  - Each card must have a companion `.rs` file declaring `pub const ID: &str = "...";` and its unit tests.
  - Exactly one script source is enforced: either a `.luau` file or a Rust `pub fn script()` definition.
  - For Luau cards (`.luau`), `build.rs` lints and compiles the source to bytecode at build time, writing `$OUT_DIR/<set>_<module>.bin`.
  - In `$OUT_DIR/registry.rs`, helper functions `luau_script_<set>_<module>()` load bytecode via `jackioh_luau::load_card(ID, BYTECODE)`.
  - `REGISTRY` table contains references to either the Rust script or the Luau loader function.

## 3. Pilot Cards Ported to Luau

1. **Core #1 Big D-Fender (`core-001`)**:
   - Source: `crates/cards/src/scripts/core/c001_big_d_fender.luau`
   - Features: `aura` hook returning `defense_armor_aura` targeting Defense-position friendly units.
   - Reads `armor` parameter dynamically via `ctx:param("armor")`.
   - Radiant face clones base behavior via `radiant = "base"`.
   - Tests: All 6 unit tests in `c001_big_d_fender.rs` pass unchanged against Luau script.

2. **Core #2 Bigot (`core-002`)**:
   - Source: `crates/cards/src/scripts/core/c002_bigot.luau`
   - Features: `targets` declaration with `TargetDecl` filtering (`enemy`, `unit`, `notTags = { "Human" }`).
   - Base face has `cry` hook calling `J.destroy({ target = { of = "chosen" } })`.
   - Radiant face has `cry` hook calling `J.destroy_all({ side = "enemy", rows = { "units" }, notTags = { "Human" } })`.
   - Tests: All 5 unit tests in `c002_bigot.rs` pass unchanged.

3. **Core #3 Right-House Defender (`core-003`)**:
   - Source: `crates/cards/src/scripts/core/c003_right_house_defender.luau`
   - Features: Base face has printed keywords only (`base = {}`).
   - Radiant face has `death` hook calling `J.summon({ defId = "core-003", player = "self" })`.
   - Tests: All 7 unit tests in `c003_right_house_defender.rs` pass unchanged.

## 4. Measurements & Verifications

- **Fuzz testing**:
  - Command: `cargo jackioh fuzz --seeds 200`
  - Result: 200 passed, 0 failed, 0 panics, 0 replay mismatches, 0 invariant violations.
  - Wall time: 21 seconds.
- **AI Gate**:
  - Command: `cargo jackioh gate`
  - `ai-vs-random`: 20 / 20 wins (22.6s)
  - `ai-vs-greedy`: 16 / 20 wins (36.1s)
  - `hard-vs-easy`: 18 / 20 wins (76.9s)
  - Slowest decision: `{"game":1,"turn":9,"ms":104.58397730970944,"rawMs":250.329486,"nodes":312,"reason":"search"}`
  - Wide-hard search: `267.75ms` (`nodes: 447`)
  - Wide-easy search: `166.80ms` (`nodes: 356`)
- **Golden traces**:
  - Command: `cargo jackioh golden check`
  - Result: 240 of 240 games replayed identically (22,713 steps), 0 diverged.
- **Spec check**:
  - Command: `cargo jackioh spec check`
  - Result: 805 rulings, 1710 proofs, 27607 citations in 1905 files, 7568 links: ok.
- **Vercel deployment ignore script**:
  - `scripts/vercel-ignore.sh` and `apps/web/src/net/vercel-ignore.test.ts`:
  - Luau files (`.luau`), `crates/luau/src`, and `crates/luau/Cargo.toml` trigger build (exit code 1).
  - Integration tests in `crates/luau/tests/*` skip build (exit code 0).
  - Vitest verification: 9 of 9 tests passed.
- **Docker image**:
  - `crates/server/Dockerfile`: added `g++` to chef build stage for native Luau compilation.
