# Slice: part 35, green 4: the server and the database
BUILDS-RUN: many. `cargo check`/`clippy -D warnings`/`test -p jackioh-server` (fake store) to green; `sh crates/server/tests/sql/run.sh` (test:sql), `sh crates/server/tests/db/run.sh` (test:db) against postgres:16; the deploy rehearsal (test:deploy) with a sandbox image (below); `cargo run -p jackioh-tools -- spec check`.

## COUNTS
- `cargo check -p jackioh-server --all-targets`: 21 errors (all in the test binary) to 0.
- `cargo clippy -p jackioh-server --no-deps --all-targets -- -D warnings`: 137 lib lints (and the tests') to 0.
- `cargo test -p jackioh-server`: first run 893 passed, 53 failed (of 946 in `tests/server.rs`); now 951 passed (the 946 and the 5 restored), 0 failed, plus export_config's 3; doctests 0 failed (two indented doc blocks fenced as text).
- test:sql: exit 0, 125 OK checks, no psql error (two `\echo` quotes fixed).
- test:db: 261/24 to 285 passed, 0 failed (`max_uses`/`uses`/`quantity` are `int` columns read as i64).
- spec check: the server's R108, R145, R149 problems gone; R301 (engine test comment) is the only one left.

## SEAMS (each "Not in SPEC, and no R-row")
- `actor::rooms::script_room_codes(&[..]) -> ScriptedRoomCodes`: the calling thread's room codes, in order, the last repeating (TS `roomIds(codes)`). R149's two tests.
- `db::fake::FakeData.before_call: Option<BeforeCall>` (`Fn(&str, &mut FakeTables) -> CallOutcome`): runs before each method with the tables in hand; may change them (a rival writer) or make `series.update` lose (`CallOutcome::Lose`). R263's three compare-and-set tests.
- `auth::SupabaseAuthInput.now: Option<ProviderClock>` (TS `now?`): the provider caches' clock; the R159/R194/R665 cache tests drive it and run unpaused.
- `tests/support/deps.rs`: `TestAppOptions.e2e` (default true; false = `env.e2e` off, no fixtures) and `empty_test_app()` (TS `createTestDeps()`: no R144 fixtures, whose `profile-1..3` ids collided with the tests' own); the store stamps on `app::now_ms`; `set_log_default` registers a process-wide no-op subscriber before a test's recorder (tracing cached callsite interest from threads with no subscriber).

## IMPLEMENTATION FIXES (src)
- `app::now_ms`: anchored on a std instant, read from the calling runtime's tokio instant either side of it, floored on both sides (a paused runtime's instant froze or skewed every other runtime's clock).
- `actor::match_actor`: the clock is synced when the actor is created (TS's arm ran before `actorFor` resolved).
- `api::game_records`: a summary the engine refuses (its panic, TS's `createGame` throw) is logged `game.record.failed`, as TS's try/catch did.
- `db::pg`: int4 columns decoded as i32. `ServerMessage::View` boxes its view, `FakeTx` its snapshot (clippy).
- `crates/server/Dockerfile`: the build stage is `rust:1.97-slim-bookworm`. Plain `rust:1.97-slim` is trixie (glibc 2.41), and its binary needs GLIBC_2.38, which the `debian:bookworm-slim` runtime (2.36) lacks: the rehearsal's first boot exited on it. SURFACE §11.3 still writes `rust:1.97-slim`.
- `crates/server/Cargo.toml`: serde_json `float_roundtrip` (JSON numbers parse to the nearest double, as `JSON.parse`).

## TEST:DEPLOY IN THIS SANDBOX
`deb.debian.org` is denied by the sandbox's egress policy (403 on CONNECT), so the Dockerfile's runtime `apt-get install ca-certificates tini` cannot run here, and containers need the agent proxy's CA for crates.io. The rehearsal ran on an image built from a scratch copy of `crates/server/Dockerfile` that adds the CA to the build stage, copies `/etc/ssl/certs` from the build stage instead of the apt step, and starts the binary without tini; the rest of `rehearse.sh` ran unchanged and passed (first boot: 26 migrations and the reseed of 318 cards at v0.2.11; second boot: nothing to migrate, 6 MB; a stale CATALOG_VERSION refused with exit 1, the database still stamped v0.2.11). Docker Hub answered 429 to the second build, so `debian:bookworm-slim` came from `mirror.gcr.io/library/debian:bookworm-slim`. The committed Dockerfile carries none of this; CI's `db (deploy rehearsal)` builds the real one, whose apt step is the only part not exercised here.

## DEPENDS-ON
- Part 32's engine (no engine change made; no engine suspect filed).
- Part 36's `apps/web/src/wire/serverConfig.ts` (export_config test) and `pnpm-lock.yaml`.

## GAPS
- e2e specs 05, 06, 10, 19, 20 not run: the Cypress binary cannot be downloaded here (part 38's cutover pull request runs them in CI).
- `actor::rooms::CODE_ATTEMPTS = 8` is a number outside `config.rs` (CLAUDE.md rule 9), as part 19 left it.
