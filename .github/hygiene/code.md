The code audit ([this run](<!-- run -->), every three days while none is open) asks for one pull request
that removes dead code and merges duplicated logic, and proves every change. It repeats #900's audit
(its findings and their proofs are on that issue), whose pull request was #953. A person reviews and
merges it (`person merges`), so make each change easy to check. Dead files and the agent instructions
have the weekly hygiene run's own issues: leave them to those.

## 1. Dead code

Find every function, type, constant, export and dependency that nothing references. The methods that
worked in #900:
- **Rust:** rename each low-hit `pub` item in a scratch copy, then run `cargo check --workspace
  --all-targets` with every feature on (`testkit`, `ts`, `luau/ts`, `wasm/preview`), and put back each
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
- `cards → jackioh-luau`, which the generated registry calls;
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

The pull request body has a table of every removal and merge: what, where, and the evidence (the
commands you ran and what they printed), one commit per batch. The fast checks pass: `cargo fmt
--check`, `cargo clippy --workspace --all-targets -- -D warnings`, the tests of every crate or package you
touched, `cargo jackioh spec check`, and the web's typecheck and eslint. Nothing to remove? Make no
change and say so in your answer, with your evidence.

---
_Opened by `.github/workflows/code-audit.yml`._
