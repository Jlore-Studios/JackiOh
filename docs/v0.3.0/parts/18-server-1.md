# v0.3.0 (part 18 of 40): server 1: the HTTP API and auth

Part 18 of 40 of #306 (the v0.3.0 Rust rewrite). The plan, the interface freeze and the port map are in `docs/v0.3.0/` (on `staging` once part 1 has run; until then on the branch `claude/relaxed-archimedes-i5fdah`): [README.md](../README.md) (how the run works), [SURFACE.md](../SURFACE.md) (every boundary), [PORT-MAP.md](../PORT-MAP.md) (every file), and the research inventories written for this plan: research A = [`research/A-engine.md`](../research/A-engine.md), B = [`B-ai.md`](../research/B-ai.md), C = [`C-server.md`](../research/C-server.md), D = [`D-ci-spec.md`](../research/D-ci-spec.md).

| | |
|---|---|
| Wave | Wave 1 (full send) |
| Starts after | nothing (the bot works from `main`) |
| Branch | the night bot's own branch and pull request to `main` (new files only; the orchestrator merges `main` into `staging`) |
| Builder | the night bot, `difficulty:hard` (Opus builds and reviews) |
| Notes | `.fullsend/notes/part-18.md` and `.assumptions` (SURFACE §16) |

Queued for the night bot by #306's orchestrator (`bot:build`, `difficulty:hard`). The Plan below is between the night bot's plan markers: build from it without planning again.

<!-- jackioh-bot:plan -->
## Plan

_Written by Claude (Opus 5.5) for #306's planning session. Edit it here to change it._

### 1. Goal

Port the HTTP half of `apps/server` to `crates/server`: env and config, the app (route table, boot, background loops), TS's own router (order, 404 for a wrong method, auth levels, body cap, rate limiter, client address from X-Forwarded-For), CORS, crypto, the Supabase JWT provider and the E2E fixture auth, and the handlers of routes 1–16, 26–30 and 33–39 of research C §2.2 (auth, catalog, codes, collection, decks and trios, tutorial, ranked, settings, stats), plus the pure ranked math. It also generates `apps/web/src/wire/serverConfig.ts` (SURFACE §5.1).

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
| `crates/server/src/auth.rs` | new | the Auth enum (SURFACE §11.2): SupabaseAuth (JWKS via reqwest + jsonwebtoken, HS256 fallback, GoTrue `/auth/v1/user` liveness cache R194, aal2 R665, admin getUserById/deleteUser) and E2eAuth (tokens `e2e-token-p1`, `e2e-token-p2`, `e2e-token-pending`) |
| `crates/server/src/app.rs` | new | SURFACE §11.2: App, build, router, serve; ROUTES in allRoutes() order; /ws/match → actor::ws_server::handle; port of `apps/server/src/index.ts` (401 lines). router, background loops, boot; `main.rs` is part 1's |
| `crates/server/tests/support/deps.rs` | new | test_app() and call() (SURFACE §11.2); port of `apps/server/test/fakes/deps.ts` (494 lines). |
| `crates/server/tests/export_config.rs` | new | writes apps/web/src/wire/serverConfig.ts: `export const NAME = <JSON>;` for every constant of config.rs that a file under apps/web/src or e2e/ imports from apps/server/src/config.ts today (grep them: `grep -rhoE 'import \{[^}]*\} from "[./]*server/src/config(\.ts)?"' apps/web/src e2e` plus multi-line imports) |
| `crates/server/src/api/auth.rs` | new | port of `apps/server/src/api/auth.ts` (898 lines). provider half → crates/server/src/auth.rs |
| `crates/server/src/api/catalog.rs` | new | port of `apps/server/src/api/catalog.ts` (276 lines). |
| `crates/server/src/api/codes.rs` | new | port of `apps/server/src/api/codes.ts` (424 lines). |
| `crates/server/src/api/collection.rs` | new | port of `apps/server/src/api/collection.ts` (192 lines). |
| `crates/server/src/api/cors.rs` | new | port of `apps/server/src/api/cors.ts` (160 lines). |
| `crates/server/src/api/crypto.rs` | new | port of `apps/server/src/api/crypto.ts` (106 lines). |
| `crates/server/src/api/decks.rs` | new | port of `apps/server/src/api/decks.ts` (674 lines). |
| `apps/server/src/api/deps.ts` | not ported | defaults fold into config.rs; ROOM_CODE_TTL_SECONDS added; leave the TS file in place, part 37 deletes it |
| `crates/server/src/api/e2e.rs` | new | port of `apps/server/src/api/e2e.ts` (301 lines). |
| `crates/server/src/api/http.rs` | new | port of `apps/server/src/api/http.ts` (709 lines). |
| `apps/server/src/api/loadout-validator.ts` | not ported | handlers call jackioh_engine::validator directly; leave the TS file in place, part 37 deletes it |
| `crates/server/src/api/ranked.rs` | new | port of `apps/server/src/api/ranked.ts` (474 lines). |
| `crates/server/src/api/retention.rs` | new | port of `apps/server/src/api/retention.ts` (26 lines). |
| `crates/server/src/api/settings.rs` | new | port of `apps/server/src/api/settings.ts` (151 lines). |
| `crates/server/src/api/stats.rs` | new | port of `apps/server/src/api/stats.ts` (432 lines). |
| `crates/server/src/api/tutorial.rs` | new | port of `apps/server/src/api/tutorial.ts` (121 lines). |
| `crates/server/src/config.rs` | new | port of `apps/server/src/config.ts` (782 lines). |
| `crates/server/src/env.rs` | new | port of `apps/server/src/env.ts` (349 lines). |
| `crates/server/src/ranked/glicko2.rs` | new | port of `apps/server/src/ranked/glicko2.ts` (147 lines). |
| `crates/server/src/ranked/ladder.rs` | new | port of `apps/server/src/ranked/ladder.ts` (286 lines). |
| `crates/server/src/ranked/season.rs` | new | port of `apps/server/src/ranked/season.ts` (106 lines). |
| `crates/server/tests/api/account.rs` | new | port of `apps/server/test/api/account.test.ts` (254 lines). |
| `crates/server/tests/api/auth.rs` | new | port of `apps/server/test/api/auth.test.ts` (984 lines). |
| `crates/server/tests/api/catalog.rs` | new | port of `apps/server/test/api/catalog.test.ts` (421 lines). |
| `crates/server/tests/api/client_address.rs` | new | port of `apps/server/test/api/client-address.test.ts` (637 lines). |
| `crates/server/tests/api/code_input_parity.rs` | new | port of `apps/server/test/api/code-input-parity.test.ts` (268 lines). |
| `crates/server/tests/api/codes.rs` | new | port of `apps/server/test/api/codes.test.ts` (807 lines). |
| `crates/server/tests/api/collection.rs` | new | port of `apps/server/test/api/collection.test.ts` (333 lines). |
| `crates/server/tests/api/cors.rs` | new | port of `apps/server/test/api/cors.test.ts` (250 lines). |
| `crates/server/tests/api/decks.rs` | new | port of `apps/server/test/api/decks.test.ts` (1032 lines). |
| `crates/server/tests/api/e2e.rs` | new | port of `apps/server/test/api/e2e.test.ts` (670 lines). |
| `crates/server/tests/api/ranked.rs` | new | port of `apps/server/test/api/ranked.test.ts` (354 lines). |
| `crates/server/tests/api/rate_limit.rs` | new | port of `apps/server/test/api/rate-limit.test.ts` (237 lines). |
| `crates/server/tests/api/redeem_feedback.rs` | new | port of `apps/server/test/api/redeem-feedback.test.ts` (673 lines). |
| `crates/server/tests/api/retention.rs` | new | port of `apps/server/test/api/retention.test.ts` (69 lines). |
| `crates/server/tests/api/settings.rs` | new | port of `apps/server/test/api/settings.test.ts` (233 lines). |
| `crates/server/tests/api/stats.rs` | new | port of `apps/server/test/api/stats.test.ts` (509 lines). |
| `crates/server/tests/api/tutorial.rs` | new | port of `apps/server/test/api/tutorial.test.ts` (174 lines). |
| `crates/server/tests/api/env_deployed_commit.rs` | new | port of `apps/server/test/env-deployed-commit.test.ts` (41 lines). |
| `crates/server/tests/api/glicko2.rs` | new | port of `apps/server/test/ranked/glicko2.test.ts` (125 lines). |
| `crates/server/tests/api/ladder.rs` | new | port of `apps/server/test/ranked/ladder.test.ts` (239 lines). |
| `crates/server/tests/api/season.rs` | new | port of `apps/server/test/ranked/season.test.ts` (66 lines). |

Do **not** touch: any `Cargo.toml`, `lib.rs`, `mod.rs` or `main.rs` (part 1's), other parts' files, `packages/` and `apps/server/` (read only; part 37 deletes them).

### 3. Steps

1. Read SURFACE.md §2, §4 and §11 fully, and `docs/v0.3.0/research/C-server.md` (research C) §2–§4 for the routes, frames and store methods you own.
2. Port each file in your table. Handlers have exactly the shape of §11.2 (`pub async fn <ts name snake_cased>(app: &App, req: Req) -> ApiResult`); JSON bodies, status codes, headers (`cache-control`, `Retry-After`) and error codes are TS's byte for byte.
3. `env.rs`: `load_env()` reports every problem at once, as `env.ts`; E2E with `NODE_ENV=production` refused; `CATALOG_VERSION` must equal `jackioh_cards::catalog_version()` (SURFACE §11.3).
4. `api/http.rs`: TS's matcher and error table (research C §2.1): `ApiErrorCode` → status, 64 KiB body cap, `resolve_caller` (lazily creates a pending profile at `RATING_START`), the 300/min sliding-window limiter keyed by account or IP-hash bucket (IPv6 /56), `TRUSTED_PROXY_HOPS` from the right, the `api.forwarded_for` log line. `api/deps.ts` folds in here; `ROOM_CODE_TTL_SECONDS` (900) goes to `config.rs`.
5. `api/auth.rs`: routes `/api/auth/me`, `/api/profile`, `DELETE /api/account` and, under `E2E=1` only, `/api/auth/signin`; `/api/auth/signup` is not ported (SURFACE §11.3). `api/catalog.rs`: `GET /api/catalog` only (with `x-deployed-commit`); `/api/catalog/:version` is not ported. `api/decks.rs`: the legacy `deckIndex`/no-`mode` body is gone; validation calls `jackioh_engine::validator` directly (no loadout-validator module).
6. `api/e2e.rs`: the fixture accounts and invite codes and `seed_e2e_fixtures(&App)` (R144: reseeded at boot under `E2E=1`), writing through `Tx` methods (part 20).
7. Background loops and boot in `app.rs` call the names SURFACE §11.2 fixes; `open_season` lives in `api/ranked.rs` (yours).
8. Port the tests in your table to `crates/server/tests/...` (TS behaviour; porting them here does not ratify your code). Use `support::deps::test_app()` and `support::deps::call(...)` (part 18 writes them) and `#[tokio::test]` with `tokio::time::pause()` where TS used manual timers.
9. Commit after every 5 files on your branch.

### 4. Tests

- The ported API tests in the table (auth, account, catalog, client-address, codes, redeem-feedback, collection, cors, decks, e2e, ranked, rate-limit, retention, settings, stats, tutorial, code-input-parity (now against `crates/engine/tests/fixtures/code-input-cases.json`), env-deployed-commit, ranked/*).
- `export_config.rs` writes the TS constants file deterministically (sorted by name).

### 5. Done when

- [ ] Every route of research C §2.2 numbered 1–16, 26–30, 33–39 (minus the two dropped) has its handler; `ROUTES` in app.rs lists all 37 kept routes in TS order, including part 19's.
- [ ] `.fullsend/notes/part-18.md` and `.assumptions` committed; `BUILDS-RUN: 0`.

### 6. Risks

- The router's quirks (404 for a wrong method, registration order) are asserted by tests; keep TS's matcher rather than axum's routing.
- The JWKS cache and liveness cache are behaviour (R194, R665), not optimisations; port their timings from `config.rs`.

<!-- /jackioh-bot:plan -->
