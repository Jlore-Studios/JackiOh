The code audit ([this run](<!-- run -->), every three days while none is open) asks for one pull request
that removes dead code, merges duplicated logic and maps the states the code holds without naming them,
checking each behaves as expected, and proves every change. Its first three parts repeat #900's audit
(its findings and their proofs are on that issue), whose pull request was #953. A person reviews and
merges it (`person merges`), so make each change easy to check. Dead files and the agent instructions
have the weekly hygiene run's own issues: leave them to those.

## 1. Dead code

Find every function, type, constant, export and dependency that nothing references. The methods that
worked in #900:
- **Rust:** rename each low-hit `pub` item in a scratch copy, then run `cargo check --workspace
  --all-targets` with every feature on (`testkit`, `ts`, `wasm/preview`), and put back each
  rename the compiler names. Blank the `#[allow(dead_code)]`s and check again. An item only its own
  tests use is dead with them.
- **TypeScript:** grep each `export` of `apps/web/src` and `e2e` with
  `git grep -nw <name> -- apps e2e crates`. Rule out `import.meta.glob`, the barrels and test globs.
- **Python:** grep each `def` and module-level name of `analysis/`, `training/` and `scripts/`. Rule out
  `getattr` dispatch, `unittest` discovery and subprocess entry points.

Those that look dead but are not (#900):
- the 30 `#[no_mangle]` WASM exports, called from `apps/web/src/wasm/index.ts`;
- the server constants `tests/export_config.rs` writes into `serverConfig.ts`;
- the engine's "decide" switches, which `rulings_config.rs` and CLAUDE.md rule 9 pin;
- `PUBLIC_ENV_VARS`, read as text by `env-production.test.ts`;
- the generated wire types: the fix for an unused one is in Rust (its `ts(export)` or its
  `export_config.rs` row), never a TypeScript delete;
- test seams a test drives live machinery through, such as the `*ForTests` hooks.

## 2. Duplicates

Find logic written more than once: rules, cost, validation, dates, API calls, auth, RNG, and JSON or
storage helpers. Read each pair side by side, and merge one into a shared function only when the same
inputs give the same outputs, edge cases included; then update every caller. The per-card scripts are
parallel on purpose.

## 3. Zombie routes

List what calls each HTTP and WebSocket route (`crates/server/src/app.rs` `ROUTES`), each page
(`apps/web/src/main.tsx`), each WASM export and each WebSocket message, tests counted apart. Flag what
nothing calls; delete none of them (CLAUDE.md rule 13).

## 4. Implicit states

Find the states nothing names: a mode the code holds as fields read together (`Option`s, booleans,
counters, a "pending" or "last sent" field beside what it guards), not as one enum. Map each, then check
it does what it should.
- **Map.** Each state's fields, every combination of their values, and what moves it between them: a
  message, a timer, a reconnect, a restart from a checkpoint, a JSON round trip, a second tab. Mark each
  combination reachable, with the shortest sequence that reaches it, or unreachable, with why. Places to
  start: a match actor's seat (`crates/server/src/actor/match_actor.rs` `Core`: its socket,
  `hello_pending` and `socket_generation`; the same three in `crates/server/src/duos/actor.rs`), the
  engine's paused action (`state.pending`, `state.work` and `work_cursor`; read `work.rs`'s header), the
  dungeon run (`crates/run`), the web's session and live game (`apps/web/src/net/session.ts`,
  `liveGame.ts`), and `Game.tsx`'s layers (a drag, an animation and a prompt at once).
- **Expected.** What each reachable combination should do, and its source: a spec section, a ruling, or
  the README that is the code's contract. Where none says, write "unspecified" and name it in the pull
  request for a person; decide nothing here (a ruling is a pull request of its own, CLAUDE.md rule 3).
- **Check.** Reach each combination through the real machinery (the testkit's `scenario()`, the server
  suite's actor, vitest on the real module), never by setting its fields by hand: a combination only
  hand-set fields reach is unreachable. Keep a test for each combination no test reached before.
- **Mismatch.** Outside the engine, the cards and the AI, fix it in a commit of its own, its test failing
  before the fix and passing after. In how a game plays (the engine, the cards, the AI) change nothing:
  that is a ruling and a patch. Name it in the pull request with its sequence and what each source says.
  Do the same for a combination you would make unrepresentable when its type is serialized (the game
  state, the wire types, checkpoints, replays): name it, change no shape.

The map is `docs/implicit-states.md`, one table per state: each combination, reachable or not and how,
what it should do and the source, what it does and the test that shows it, and the commit it was checked
at. The first audit makes it. Each audit maps at least three states it lacks, and checks again every
mapped state whose files changed since its commit (`git log <commit>.. -- <files>`).

Those that look implicit but are not:
- a field that is `None` or a default for a record older than it (a match that predates portraits or
  loadouts, an older replay, checkpoint or shipped catalog a replay folds under): that is a version, not
  a state. Check the old record still reads, and remove nothing.
- the engine's two thread-locals (CLAUDE.md rule 4), each set for one call and put back after it.

## Never

- Never delete anything called from outside this codebase (CLAUDE.md rule 13): a webhook, a workflow's
  `schedule:`, an auth callback, a route a deploy probes or a person calls (`/readyz` is the site
  Worker's daily wake-up, `/healthz` its check of the running image, and the deploy jobs poll
  `/api/catalog`), or a `cargo jackioh`, server or harness subcommand a
  workflow, a deploy or a person runs. Name it in the pull request for a person instead.
- Never delete a test or a function a ruling's `proven_in` needs: `cargo jackioh spec check` passes.
- Never delete a named constant nothing reads before asking whether it should be read (CLAUDE.md rule 9).
- Never change `.github/`, `bot/`, `.harness/`, `.squishy/` or `.claude/`. Name what should go there in
  the pull request, for a person.

## Done when

The pull request body has a table of every removal, merge and fix: what, where, and the evidence (the
commands you ran and what they printed), one commit per batch. It lists the states it mapped or checked
again, and each mismatch and "unspecified" combination it names for a person. The fast checks pass:
`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, the tests of every crate or
package you touched, `cargo jackioh spec check`, and the web's typecheck and eslint. Nothing to remove,
no state left to map and none changed since its commit? Make no change and say so in your answer, with
your evidence.

---
_Opened by `.github/workflows/code-audit.yml`._
