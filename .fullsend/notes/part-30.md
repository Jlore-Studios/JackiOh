# Slice: part 30, CI: essential checks per pull request, the daily super run
BUILDS-RUN: 0 cargo (nothing Rust compiles yet). Ran: `pnpm install --frozen-lockfile` (with `CYPRESS_INSTALL_BINARY=0`; the Cypress binary download fails in this sandbox), the two script tests about 8 times while writing them (`pnpm exec vitest run apps/web/src/net/ci-scope.test.ts apps/web/src/net/vercel-ignore.test.ts`, green, 15 tests), actionlint 1.7.12 with shellcheck 0.11.0 over every workflow (0 errors), shellcheck over both scope scripts, `node e2e/scripts/shard-specs.mjs` by hand, and the training gate's claim step against a scratch repo (7 cases, all as expected).

## REQUIRED CHECKS (for part 38)
Branch protection on `main`, `.harness/config.json` and `.squishy/config.json` `required_checks` become exactly these five, the job `name:`s:
- `rust` (ci.yml)
- `web` (ci.yml)
- `e2e smoke` (ci.yml)
- `db` (ci.yml)
- `bot selftest` (bot-selftest.yml, unchanged)

Remove the seven old ones: `lint, typecheck, unit, fuzz, coverage`, `ai quality gates (full)`, `sql invariants (real Postgres)`, `store contract (real Postgres)`, `e2e (chrome)`, `e2e (electron)` (and keep `bot selftest`). The training gate needs no name of its own: it is a piece of `rust` (below). A cheap local mirror for the bots' `gates`: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts`, `cargo jackioh catalog check`, `cargo jackioh patches check`, `cargo jackioh spec check`, `pnpm exec vitest run --project web --changed origin/main --exclude '**/src/audio/**' --exclude '**/src/fx/**'`.

## FILES
.github/workflows/ci.yml — rewritten: `changes`; `rust` over rust (fmt, clippy), rust (test), rust (release build), rust (checks, fuzz 200), rust (training gate); `web` over web (wasm, client build), web (typecheck, eslint), web (unit k/3); `e2e smoke` over e2e smoke (k/2); `db` over db (sql invariants | store contract | deploy rehearsal); `workflow_dispatch` added; Electron, AI gates, coverage, full fuzz and full e2e gone
.github/workflows/super.yml — new: cron `17 7 * * *` + dispatch; release build, wasm and client build, fuzz (k/4: 10,000 seeds plain and `--handicap`), ai gates (k/4) + ai gates (full) via `gate merge`, golden check, coverage (cargo llvm-cov, engine+cards+ai, floor from .github/coverage-floor.txt), e2e (k/8) all specs on Chrome, e2e component (chrome), web (audio, fx and asset tests), db (3 suites), report (issue `CI: daily super run failed`: open or comment on red with one linked row per failed job, close on green)
.github/coverage-floor.txt — `90`
.github/actions/setup/action.yml — inputs `node` and `rust` (both default true): dtolnay/rust-toolchain@1.97.0 (rustfmt, clippy, wasm32) + Swatinem/rust-cache@v2 (saves only on main and staging; caches .cache/bin for wasm-bindgen)
.github/actions/e2e-shard/action.yml — Chrome only; input `specs: all|smoke`; downloads the run's `web-build` and `rust-release` artifacts, serves apps/web/dist with `vite preview`, boots `target/release/jackioh-server` with `E2E=1` and `CATALOG_VERSION=$(target/release/jackioh catalog-version)`
.github/workflows/ci-duration.yml — super.yml stays off its list by design (comment); `rust (training gate)` exempt; issue text names the new split points
.github/workflows/patches-ship.yml — `cargo jackioh patches ship`; paths `crates/cards/patches/pending/**`; Node off; timeout 20
scripts/ci-scope.sh + apps/web/src/net/ci-scope.test.ts — skip list adds `training/*.md`, `training/loop.sh`; prints a second line `db=true|false`; `--no-renames`
scripts/vercel-ignore.sh + apps/web/src/net/vercel-ignore.test.ts — the Rust layout: crates/{engine,cards,ai,wasm} (minus tests/), every Cargo.toml, Cargo.lock, rust-toolchain.toml, .cargo/, scripts/build-wasm.sh build; crates/server, crates/tools (minus their manifests), spec/, training/, render.yaml skip; packages/ and apps/server entries dropped; `--no-renames`
e2e/scripts/shard-specs.mjs — `export const SMOKE = ["01", "06", "10", "13", "19"]`, `smokeSpecs()`, CLI `--smoke`; algorithm unchanged

## SURFACE
`node e2e/scripts/shard-specs.mjs [--smoke] <k> <K>`; `sh scripts/ci-scope.sh <base> <head>` prints `full=…` then `db=…`. Artifacts within a run: `rust-release` (`rust-release.tgz`: target/release/jackioh, target/release/jackioh-server), `web-build` (`web-build.tgz`: apps/web/src/wasm/pkg, apps/web/dist). Setup action inputs `node`, `rust`. e2e-shard inputs `specs`, `shard`, `shards`.

## DEPENDS-ON
- Part 21: `sh scripts/build-wasm.sh` writes apps/web/src/wasm/pkg; `pnpm build:e2e` writes apps/web/dist; `apps/web/src/test/setup.ts` loads the module from pkg on disk; `e2e/support/tasks/replay-runner.ts` spawns `target/release/jackioh replay`; `pnpm --dir apps/web typecheck`, `pnpm --dir e2e typecheck`, `pnpm --dir e2e typecheck:component` and `pnpm exec eslint apps/web e2e` work; the web's audio and fx tests stay under `apps/web/src/audio/` and `apps/web/src/fx/`.
- Part 22: `jackioh catalog check | patches check | catalog-version | fuzz --from N --seeds N [--handicap] | gate --full --shard k/K --out <dir> | gate merge <dir> | patches ship` (ship still prints `patches ship: shipped <version>` lines, which patches-ship.yml's PR title reads).
- Part 23: `jackioh golden check`, writing `target/golden-diff/` on a mismatch. Part 28: `jackioh spec check` (exists).
- Part 29: `jackioh promote --lane <lane> --parent-bin <path> --verify`; `crates/ai/generation.json` has `generation` and `lane`; a promotion appends the same object as the last line of `training/history/<lane>.jsonl`.
- Part 5 / part 18: `cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts` writes apps/web/src/wire/generated/, engineConfig.ts and serverConfig.ts (ts-rs export tests and the two export_config tests).
- Part 18: `E2E=1 target/release/jackioh-server` boots on :8787 with no other env than `CATALOG_VERSION` (equal to the compiled one).
- Part 20: `crates/server/tests/sql/run.sh` and `crates/server/tests/deploy/rehearse.sh` need only Docker (and the runner's own node/jq if they parse JSON); `crates/server/tests/db/run.sh` needs Docker and cargo. CI calls them by path, not through `pnpm test:*`, so part 37's package.json trim cannot break them.

## GAPS
- Nothing here has run. The first real run of ci.yml is the cutover pull request (part 38); a person can try it earlier with `gh workflow run ci.yml --ref staging` (ci.yml is known to GitHub from main, and the dispatch reads staging's copy, which has `workflow_dispatch`).
- super.yml exists only on staging, so GitHub may refuse `gh workflow run super.yml --ref staging` (404) until a workflow of that name has been seen on the default branch; then V25's check (a dispatched run opens the issue while red and closes it once green) waits for the cutover. Its schedule fires only from main, i.e. after cutover.
- Step 5 / V24 (every per-PR job under 5 minutes) was unmeasured when this was written; it is measured and met now, see `## V24` below. Likeliest to run long: `rust (test)` (debug build of the whole workspace plus 240 golden games in debug), `rust (release build)` (`lto = "thin"`), `web (wasm, client build)`. Fixes in order: a cargo test matrix by crate; `CARGO_PROFILE_RELEASE_LTO=false` in CI's env for the e2e build only; more `web (unit k/K)` shards. `db (deploy rehearsal)` builds the Docker image from scratch each time and may pass five minutes on the pull requests that run it; if so, rehearse.sh could take a prebuilt image or buildx's GHA cache.
- Deviations from the brief, each deliberate: (1) the e2e-shard action no longer compiles: `rust (release build)` and `web (wasm, client build)` build once per run in the brief's order (build-wasm.sh before build:e2e; `cargo build --release -p jackioh-server -p jackioh-tools`) and every shard downloads the result, or each of 2 smoke and 8 daily shards would compile the workspace twice; (2) the ts-rs/config export diff (V20) runs in `rust (test)`, where the engine and server are compiled anyway, not in `web`; (3) the db gate also fires on its own test files, `crates/server/Cargo.toml` and `Cargo.lock`, and lives in ci-scope.sh (tested) instead of inline YAML; (4) ci-duration.yml keeps watching `bot selftest` and `deploy watch` (only super is kept off); (5) the stale workflow-level `CATALOG_VERSION: v0.2.0` is gone, since the Rust server refuses a version other than its compiled one; (6) super.yml also runs the web's audio, fx and asset tests, which no other job would; (7) `--no-renames` in both scope scripts: a move onto a skip list used to hide its old path (pre-existing, now tested).
- The audio, fx and asset web tests no longer gate a pull request. If super keeps catching regressions there, give them a ci-scope output like `db` (`apps/web/src/audio/`, `apps/web/src/fx/`, `apps/web/public/audio/`) and run them per pull request when touched.
- Left stale, not in my table: CLAUDE.md's Commands and CI paragraphs (seven checks, Electron, `pnpm fuzz`, `pnpm ai:gate`, `test:coverage`), e2e/README.md's Electron and sharding notes, bot-night.yml / squishy-run.yml toolchain setup (part 38), BUILD.md's CI wording.
- promote-production.yml is untouched; its deploy job uses the setup action, which now installs Rust by default, so its `pnpm --filter @jackioh/web build` (prebuild runs build-wasm.sh) keeps working.

## V24
Every per-pull-request job of ci.yml finishes under five minutes on a warm cache. Final warm run: workflow_dispatch run 37632766405 on staging at 425011a, all green; the slowest job is 3:46.

| job | total | main step |
|---|---|---|
| e2e smoke (2/2) | 3:46 | e2e-shard 3:08 |
| rust (test engine) | 3:44 | cargo test 3:16 |
| rust (test engine rules) | 3:36 | cargo test 3:08 |
| rust (release build) | 3:06 | cargo build --release 2:37 |
| db (store contract) | 2:59 | run.sh 2:24 |
| rust (test cards lib) | 2:56 | cargo test 2:33 |
| web (unit 3/3) | 2:56 | vitest 2:11 |
| rust (test ai) | 2:54 | cargo test 2:34 |
| rust (test tools, wasm) | 2:46 | cargo test 2:17 |
| rust (test cards) | 2:37 | cargo test 2:12 |
| e2e smoke (1/2) | 2:33 | e2e-shard 2:01 |
| rust (test server) | 2:19 | cargo test 1:49 |
| web (wasm, client build) | 2:06 | build-wasm.sh 1:13 |
| web (unit 1/3), (unit 2/3) | 1:58, 1:56 | |
| rust (fmt, clippy) | 1:34 | clippy 1:01 |
| web (typecheck, eslint) | 1:17 | |
| rust (checks, fuzz 200) | 0:35 | |
| db (deploy rehearsal) | 0:32 | the image from buildx's cache |
| db (sql invariants) | 0:28 | |

Before (run 37620287774): rust (test) 14:36, rust (release build) 5:51, db (deploy rehearsal) 5:25, db (store contract) 4:44.

What changed:
- `rust (test)` is a matrix, `rust (test <group>)`: engine rules (`--test rules`, testkit only), engine (`--lib --test golden --test export_config`, testkit and ts, then `--doc`), cards lib (`--lib`), cards (`--test cards`, then `--doc`), server, ai, and tools, wasm (`--workspace --exclude` the other four, so a new crate lands there). Measured with `--timings` on GitHub's 4 vCPUs: the engine and the cards at opt-level 2 cost about 80 s in every group; `rules` alone takes 118 s to compile, the engine's unit tests 115 s, the cards' unit tests 166 s, `tests/cards.rs` 58 s. A guard step checks the split crates' test binaries against `cargo metadata`, so a new `tests/*.rs` cannot go unrun. Every group runs the V20 diff. Each group has its own rust-cache entry (the setup action's new `cache-key` input; a cache key may not hold a comma).
- `CARGO_PROFILE_TEST_DEBUG=0` workflow-wide, and `CARGO_PROFILE_RELEASE_LTO=false` in `rust (release build)` only (e2e and the checks; Render's image keeps thin LTO). Cargo.toml is untouched.
- The setup action uninstalls every rustup toolchain but the pinned one before rust-cache keys itself. rust-cache hashes every installed toolchain into its key, and the runner image's own `stable` differed between runners of one run (1.98.1 on one, 1.99.0 on the next) during an image rollout, so about half the jobs missed their cache and compiled every dependency. That is why the brief's warm and cold runs looked alike.
- `crates/server/tests/db/run.sh` builds once, in the test profile with the tests, and runs that binary for `migrate`. Before, `cargo build` (dev), `cargo run` (dev, without the dev-dependency features) and `cargo test` (test) compiled the workspace up to three times.
- The deploy rehearsal builds its image with `docker buildx build --load` and the GitHub Actions cache (rehearse.sh's `REHEARSAL_CACHE_FROM` and `REHEARSAL_CACHE_TO`, set by ci.yml; written only from main and staging, `mode=min`, about 45 MB). Same Dockerfile and context; without the variables rehearse.sh runs the plain `docker build` it did.
- `workflow_dispatch` input `e2e: smoke | all`. `all` runs every spec in eight shards (`e2e all (k/8)`) and `e2e component (chrome)`, as super.yml does, and its summary is named `e2e all`, so it never reports under the required `e2e smoke`. Pull requests and pushes are unchanged. ci-duration exempts `e2e all (…)` and the component job. All-specs runs: 37628675555 (05 and 20 sent the legacy `{ deckIndex }` room body; fixed in e7f6aac), then 37630374228, every shard and the component specs green.

Still over five minutes: `db (deploy rehearsal)` when the image's inputs changed. The Dockerfile copies all of `crates/` and compiles in one `RUN`, so any change under `crates/` (every pull request that runs the db suites has one, except a render.yaml-only change) recompiles every dependency and the server with thin LTO, about 270 s of build: 5:31 in run 37631639361 and 5:45 in run 37628675555, under the seven-minute alert. Getting it under five needs crates/server/Dockerfile, which part 30 does not own: a dependency layer before `COPY crates` (cargo-chef, or the manifests with stub sources), so only the workspace crates rebuild, or an `ARG` the rehearsal sets to skip LTO (default `thin`, so Render's build is unchanged), or both.
