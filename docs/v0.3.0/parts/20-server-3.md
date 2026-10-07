# v0.3.0 (part 20 of 40): server 3: persistence, migrations, CLIs and the Docker deploy

Part 20 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | part 1 |
| Branch | `staging` (push straight to it; no pull request) |
| Builder | a Claude Code cloud session, Opus |
| Notes | `.fullsend/notes/part-20.md` and `.assumptions` (SURFACE §16) |

Unlabelled on purpose: #306 holds labels until a person says the run is ready. The Plan below is between the night bot's plan markers, so if a person later queues this issue for a bot, it builds from this plan without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port the persistence of `apps/server` to `crates/server`: the `Db`/`Tx` enums with every store method (SURFACE §11.2), `PgStore` over sqlx (every transaction switching to `service_role` with the JWT claim, every `app.*` SQL function call kept), `FakeStore` for unit tests and `E2E=1`, the migration runner (ledger, checksum, the 0013 exception, the advisory lock), the CLIs, the store contract suite against both stores, the SQL invariant suite (copied, unchanged), and the Docker deploy with its rehearsal.

**How you work (fullsend builder rules, in priority order; full text in `.claude/skills/fullsend/agents/builder.md`).**
1. Never run `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or any test runner. Nothing on `staging` compiles until Wave 3; every error you would see is someone else's unwritten file. Count any you ran as `BUILDS-RUN`.
2. Write only the files in your Files to touch table and your two notes files. Read anything in `packages/`, `apps/`, `docs/`, `SPEC.md` (the TypeScript is your spec); do not read other parts' Rust under `crates/`: it is half-written.
3. Match SURFACE.md exactly at every boundary: paths by §4.1, names by §4.2, types by §4.3, the shapes of §5–§14.
4. No stubs: no `todo!()`, `unimplemented!()`, placeholder bodies or `// TODO`. Port the real body; if you truly cannot, leave the function out and list it under GAPS.
5. Duplicate on purpose: a small helper another part probably writes, write your own private copy.
6. Call what you wish existed: another module's function by its TS name snake_cased at its TS module's Rust path.
7. Don't ask questions: decide, and record the decision in your notes.
8. Stop when Done when holds.

**Pushing.** README §3.2: `git fetch origin staging && git checkout -B work origin/staging`, write, commit only your files, `git pull --rebase origin staging && git push origin HEAD:staging` (retry with back-off). Commit often. A session that is cut off leaves its work on `staging`; the next session for this part starts at the first file in the table that is missing or empty.

### 2. Files to touch

| File | new/change/delete | What |
|---|---|---|
| `crates/server/Dockerfile` | new | SURFACE §11.3 |
| `render.yaml` | change | `runtime: docker`, `dockerfilePath: crates/server/Dockerfile`, `dockerContext: .`; drop buildCommand, startCommand's Node derivation, NODE_VERSION, CYPRESS_INSTALL_BINARY; keep name, plan, region, branch, healthCheckPath, CATALOG_VERSION (now checked by the server), TRUSTED_PROXY_HOPS and the `sync: false` secrets; rewrite its comments |
| `crates/server/tests/store/migrations_pinned.rs` | new | pins every migration's checksum (← test/db/migrations-pinned.test.ts); port of `apps/server/test/db/migrations-pinned.test.ts` (75 lines). |
| `package.json` | change | `test:sql`, `test:db`, `test:deploy` point at `crates/server/tests/{sql,db,deploy}/…` |
| `crates/server/.env.example` | copy | `cp` from `apps/server/.env.example` (70 lines); the original stays until part 37 deletes it |
| `crates/server/src/db/fake.rs` | new | port of `apps/server/src/api/e2e-store.ts` (767 lines). one FakeStore for unit tests and E2E=1; port of `apps/server/src/api/memory-stores.ts` (866 lines). one FakeStore for unit tests and E2E=1 |
| `crates/server/src/db/store.rs` | new | port of `apps/server/src/api/ports.ts` (1272 lines). Db/Tx enums, every Store method, row types; Timers/Logger/Ids/Hashes go (SURFACE §11) |
| `crates/server/src/cli/card_stats.rs` | new | port of `apps/server/src/db/card-stats.ts` (138 lines). |
| `crates/server/src/cli/import_dev_records.rs` | new | port of `apps/server/src/db/import-dev-records.ts` (86 lines). |
| `crates/server/src/db/migrate.rs` | new | port of `apps/server/src/db/migrate.ts` (156 lines). checksum and REWRITTEN exactly (SURFACE §11) |
| `crates/server/src/cli/mint_code.rs` | new | port of `apps/server/src/db/mint-code.ts` (113 lines). |
| `crates/server/src/cli/season_start.rs` | new | port of `apps/server/src/db/season-start.ts` (109 lines). |
| `crates/server/src/cli/seed_accounts.rs` | new | port of `apps/server/src/db/seed-accounts.ts` (286 lines). |
| `crates/server/src/cli/seed_catalog.rs` | new | port of `apps/server/src/db/seed-catalog.ts` (187 lines). |
| `crates/server/src/db/pg.rs` | new | port of `apps/server/src/db/store.ts` (2979 lines). sqlx; keep every SET LOCAL role/claim and app.* call |
| `crates/server/tests/db/bootstrap.sql` | new | port of `apps/server/test/db/bootstrap.sql` (57 lines). |
| `crates/server/tests/store/card_stats.rs` | new | port of `apps/server/test/db/card-stats.test.ts` (157 lines). |
| `apps/server/test/db/contract.memory.test.ts` | not ported | TS pool/dynamic-import plumbing, or a one-line runner folded into the contract; leave the TS file in place, part 37 deletes it |
| `apps/server/test/db/contract.postgres.spec.ts` | not ported | TS pool/dynamic-import plumbing, or a one-line runner folded into the contract; leave the TS file in place, part 37 deletes it |
| `crates/server/tests/store/contract.rs` | new | port of `apps/server/test/db/contract.ts` (2251 lines). run against FakeStore always and PgStore when DATABASE_URL is set |
| `crates/server/tests/db/grants.sql` | new | port of `apps/server/test/db/grants.sql` (11 lines). |
| `apps/server/test/db/harness.ts` | not ported | TS pool/dynamic-import plumbing, or a one-line runner folded into the contract; leave the TS file in place, part 37 deletes it |
| `apps/server/test/db/loadstore-boot.ts` | not ported | TS pool/dynamic-import plumbing, or a one-line runner folded into the contract; leave the TS file in place, part 37 deletes it |
| `crates/server/tests/store/mint_code.rs` | new | port of `apps/server/test/db/mint-code.test.ts` (85 lines). |
| `apps/server/test/db/pool-error.test.ts` | not ported | TS pool/dynamic-import plumbing, or a one-line runner folded into the contract; leave the TS file in place, part 37 deletes it |
| `crates/server/tests/store/postgres.rs` | new | port of `apps/server/test/db/postgres.spec.ts` (743 lines). |
| `apps/server/test/db/redeem-race.memory.test.ts` | not ported | TS pool/dynamic-import plumbing, or a one-line runner folded into the contract; leave the TS file in place, part 37 deletes it |
| `apps/server/test/db/redeem-race.postgres.spec.ts` | not ported | TS pool/dynamic-import plumbing, or a one-line runner folded into the contract; leave the TS file in place, part 37 deletes it |
| `crates/server/tests/store/redeem_race.rs` | new | port of `apps/server/test/db/redeem-race.ts` (245 lines). run against FakeStore always and PgStore when DATABASE_URL is set |
| `crates/server/tests/db/run.sh` | new | port of `apps/server/test/db/run.sh` (74 lines). |
| `crates/server/tests/store/season_start.rs` | new | port of `apps/server/test/db/season-start.test.ts` (119 lines). |
| `crates/server/tests/store/seed_accounts.rs` | new | port of `apps/server/test/db/seed-accounts.test.ts` (58 lines). |
| `crates/server/tests/store/seed_catalog.rs` | new | port of `apps/server/test/db/seed-catalog.spec.ts` (106 lines).; port of `apps/server/test/db/seed-catalog.test.ts` (121 lines). |
| `apps/server/test/db/store-guards.test.ts` | not ported | TS pool/dynamic-import plumbing, or a one-line runner folded into the contract; leave the TS file in place, part 37 deletes it |
| `apps/server/test/db/vitest.config.ts` | not ported | TS pool/dynamic-import plumbing, or a one-line runner folded into the contract; leave the TS file in place, part 37 deletes it |
| `crates/server/tests/deploy/rehearse.sh` | new | port of `apps/server/test/deploy/rehearse.sh` (205 lines). rehearses the Docker image and render.yaml |
| `apps/server/test/fakes/store.ts` | not ported | FakeStore is the fake; leave the TS file in place, part 37 deletes it |
| `crates/server/tests/sql/00_supabase_stub.sql` | copy | `cp` from `apps/server/test/sql/00_supabase_stub.sql` (39 lines); unchanged; the original stays until part 37 deletes it |
| `crates/server/tests/sql/01_schema_invariants.sql` | copy | `cp` from `apps/server/test/sql/01_schema_invariants.sql` (932 lines); unchanged; the original stays until part 37 deletes it |
| `crates/server/tests/sql/02_rls_as_client.sql` | copy | `cp` from `apps/server/test/sql/02_rls_as_client.sql` (943 lines); unchanged; the original stays until part 37 deletes it |
| `crates/server/tests/sql/03_match_lifecycle.sql` | copy | `cp` from `apps/server/test/sql/03_match_lifecycle.sql` (1408 lines); unchanged; the original stays until part 37 deletes it |
| `crates/server/tests/sql/03b_legacy_loadout_seed.sql` | copy | `cp` from `apps/server/test/sql/03b_legacy_loadout_seed.sql` (102 lines); unchanged; the original stays until part 37 deletes it |
| `crates/server/tests/sql/04_decks_and_series.sql` | copy | `cp` from `apps/server/test/sql/04_decks_and_series.sql` (802 lines); unchanged; the original stays until part 37 deletes it |
| `crates/server/tests/sql/05_tutorial_progress.sql` | copy | `cp` from `apps/server/test/sql/05_tutorial_progress.sql` (222 lines); unchanged; the original stays until part 37 deletes it |
| `crates/server/tests/sql/06_account_deletion.sql` | copy | `cp` from `apps/server/test/sql/06_account_deletion.sql` (218 lines); unchanged; the original stays until part 37 deletes it |
| `crates/server/tests/sql/07_retention_purge.sql` | copy | `cp` from `apps/server/test/sql/07_retention_purge.sql` (80 lines); unchanged; the original stays until part 37 deletes it |
| `crates/server/tests/sql/08_game_records.sql` | copy | `cp` from `apps/server/test/sql/08_game_records.sql` (85 lines); unchanged; the original stays until part 37 deletes it |
| `crates/server/tests/sql/09_catalog_growth.sql` | copy | `cp` from `apps/server/test/sql/09_catalog_growth.sql` (81 lines); unchanged; the original stays until part 37 deletes it |
| `crates/server/tests/sql/10_last_boards.sql` | copy | `cp` from `apps/server/test/sql/10_last_boards.sql` (215 lines); unchanged; the original stays until part 37 deletes it |
| `crates/server/tests/sql/11_player_settings.sql` | copy | `cp` from `apps/server/test/sql/11_player_settings.sql` (256 lines); unchanged; the original stays until part 37 deletes it |
| `crates/server/tests/sql/12_ranked.sql` | copy | `cp` from `apps/server/test/sql/12_ranked.sql` (475 lines); unchanged; the original stays until part 37 deletes it |
| `crates/server/tests/sql/13_glitch.sql` | copy | `cp` from `apps/server/test/sql/13_glitch.sql` (114 lines); unchanged; the original stays until part 37 deletes it |
| `crates/server/tests/sql/14_patch_retcon.sql` | copy | `cp` from `apps/server/test/sql/14_patch_retcon.sql` (151 lines); unchanged; the original stays until part 37 deletes it |
| `crates/server/tests/sql/run.sh` | copy | `cp` from `apps/server/test/sql/run.sh` (151 lines); then point its migrations path at `crates/server/migrations`; the original stays until part 37 deletes it |

Do **not** touch: any `Cargo.toml`, `lib.rs`, `mod.rs` or `main.rs` (part 1's), other parts' files, `packages/` and `apps/server/` (read only; part 37 deletes them).

### 3. Steps

1. Read SURFACE.md §2, §4 and §11 fully, and `docs/v0.3.0/research/C-server.md` (research C) §2–§4 for the routes, frames and store methods you own.
2. `db/store.rs`: `Db`, `Tx`, `begin`, `commit` and one method per TS Store method (`apps/server/src/api/ports.ts:1086` and the sub-stores, research C §4.1), named `<substore>_<method>` snake_cased (root `redeem`, `purge_expired`), each dispatching to `pg::<name>` or `fake::<name>`. Row types (`MatchRow`, `MatchClocks`, `Ticket`, `Room`, `SeriesRow` with its CAS `version`, …) as serde structs. Errors: `StoreError` with a `Duplicate` variant for TS's `DuplicateResultError` (Postgres 23505 on `results_pkey`).
3. `db/pg.rs`: port `store.ts`'s SQL text verbatim into `sqlx::query(...)` calls with binds (no compile-time `query!` macros: they need a live database at build time). Keep its "KNOWN DIVERGENCES" comment.
4. `db/fake.rs`: `FakeData` (all tables in memory), `FakeTx` (holds the tokio mutex guard and a snapshot cloned at `begin`, restored on drop without `commit`), one free fn per store method, ported from `memory-stores.ts` and `e2e-store.ts` (one fake, not two).
5. `db/migrate.rs`: SURFACE §11.3 exactly; migrations embedded with `include_str!` from `crates/server/migrations/` in lexical order; the checksum is `jackioh_engine::replay::fnv1a32_utf16` over the file text.
6. `cli/*.rs`: `seed-catalog` (upsert `public.cards` from `jackioh_cards::catalog_json()` at `catalog_version()`, then `app.settings.catalog_version`, one transaction), `mint-code`, `seed-accounts` (refuses under production), `season-start`, `stats-cards`, `stats-import` (JSONL of `GameRecord`s). `release` (in main.rs) = migrate, seed-catalog, serve.
7. Tests: `tests/store/contract.rs` ports `test/db/contract.ts` as one function per case taking `&Db`, run against `Db::Fake` always and against `Db::Pg` when `DATABASE_URL` is set (that is `test:db`); same for `redeem_race.rs`; `postgres.rs` (Pg only). Copy `apps/server/test/sql/` to `crates/server/tests/sql/` (`mkdir -p crates/server/tests/sql && cp apps/server/test/sql/* crates/server/tests/sql/`; part 37 deletes the originals) and point the copied `run.sh` at `crates/server/migrations`. Port `test/db/run.sh` and `test/deploy/rehearse.sh` to build and run the Docker image (`docker build -f crates/server/Dockerfile .`) with render.yaml's env, as a non-superuser `migrator`, probing `/api/catalog` for the version and `x-deployed-commit`, under the memory limit, then a second boot that migrates nothing.
8. Push after every 5 files (README §3.2).

### 4. Tests

- The store contract (≈101 cases) against FakeStore in `cargo test`, against Postgres in `test:db`; redeem race; Postgres-only role/claim checks; migrations pinned; CLI tests (seed-catalog, mint-code, seed-accounts, season-start, card-stats).
- `test:sql` unchanged in substance (V17); `test:deploy` rehearses the Docker image (V22).

### 5. Done when

- [ ] Every Store method of `ports.ts` exists on `Tx` with both implementations.
- [ ] `render.yaml` has no Node left; the rehearsal reads it.
- [ ] `.fullsend/notes/part-20.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- A different migration checksum makes every existing database refuse to boot: the pinned test is the guard.
- sqlx's text/JSON type mapping differs from `pg`'s; jsonb columns go through `serde_json::Value`, timestamps through `time::OffsetDateTime`, and epoch-ms fields stay `i64` as TS stored them.

<!-- /jackioh-bot:plan -->
