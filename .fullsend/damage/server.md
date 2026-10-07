# Damage: `crates/server/**` (part 31, server reconciler)

## How it was measured

The real build cannot reach `jackioh-server`: at the first build (`948e025`) `cargo check -p
jackioh-server --all-targets` stopped in `jackioh-engine (lib)` (188 errors); at the end (`2dd342a`,
after part 32's engine commits) it stops in `jackioh-cards (lib)` (28 errors). So the server's counts
are from a **shadow build**: a scratch copy of the workspace outside the repository (never committed)
in which every function body of the server's dependencies is replaced by `loop {}` (signatures,
types, consts, derives and the testkit kept), so the server is checked against the dependencies'
signatures as they stand on `staging`.

- Before: engine, cards and ai stubbed at `948e025`, plus nine engine imports of names the engine had
  not settled yet removed and one missing derive added (`CastAfterward`), none of which the server
  names.
- After: the **real** engine at `f0e8727` (part 32: "the lib type-checks and borrow-checks"), with
  `--features testkit` for the tests; cards and ai stubbed.

## Before (first build, `948e025`)

Real build: 0 server errors reachable; `jackioh-engine (lib)` 188 errors.

Shadow build, `jackioh-server` lib: **115** errors (121 reported, lib and lib-test counting some
twice). The test binary was not reached (the lib failed). A static scan of the tests' imports found
15 more unresolved names (`create_supabase_auth`, `E2eAccount`/`E2eInviteCodes`/`E2eSeedOptions`,
`MintInviteCodeInput`, `ListPublicOptions`, `create_fake_engine`, `FakeEngineOptions`,
`TEST_PATCH_VERSION`, …).

| File | Errors |
|---|---|
| `src/api/series_rules.rs` | 23 |
| `src/db/fake.rs` | 20 |
| `src/db/pg.rs` | 16 |
| `src/db/store.rs` | 15 |
| `src/api/results.rs` | 8 |
| `src/api/collection.rs` | 7 |
| `src/api/decks.rs` | 6 |
| `src/api/queue.rs` | 3 |
| `src/cli/mint_code.rs`, `src/app.rs`, `src/api/http.rs`, `src/api/codes.rs`, `src/actor/ws_server.rs` | 2 each |
| `src/cli/seed_accounts.rs`, `src/api/tutorial.rs`, `src/api/series.rs`, `src/api/rematch.rs`, `src/actor/rooms.rs`, `src/actor/registry.rs`, `src/actor/engine.rs` | 1 each |

| Code | Errors |
|---|---|
| E0308 (mismatched types) | 81 |
| E0609 (no field: `[T; 3]` vs tuple) | 12 |
| E0599 (no method/variant: `FakeData::default`, `StoreError::DuplicateResult`, `as_deref` on three-state) | 6 |
| E0277 | 6 |
| E0061 (arity) | 4 |
| E0432 (unresolved import) | 3 |
| E0425 | 2 |
| E0559 | 1 |

## After (`2dd342a`)

Real build: still blocked, now in `jackioh-cards (lib)` (28 errors).

Shadow build against the real engine: `jackioh-server` lib **0** errors (one deprecation warning,
`time::format_description::parse` in `cli/mint_code.rs`). Test binary (`tests/server.rs`) **21**
errors, all local:

| File | Errors | What |
|---|---|---|
| `tests/actor/match_actor.rs` | 4 | `i64::from(usize)` ×2, `&str`→`String`, one call's arguments |
| `tests/api/collection.rs` | 4 | `IndexMap::new()` needs its value type (E0282/E0283 ×2) |
| `tests/api/decks.rs` | 4 | caps passed as `usize` where the store takes `i64` ×3; `is_portrait_id(&str)` takes `&Value` |
| `tests/api/queue.rs` | 2 | `usize` → the store's `i64` |
| `tests/actor/aim.rs` | 2 | `parse_client_message` answers `Result`; `ClientMessage::Aim(AimMessage)` is a tuple variant |
| `tests/store/contract.rs` | 1 | `usize` → `i64` |
| `tests/api/e2e.rs` | 1 | `usize` → `i64` |
| `tests/actor/recovery.rs`, `tests/actor/last_boards.rs`, `tests/actor/glitch.rs` | 1 each | `&String` → `String` |

| Code | Errors |
|---|---|
| E0308 | 14 |
| E0282 / E0283 | 2 / 2 |
| E0277 | 2 |
| E0769 | 1 |

Lines: `crates/server/src` 27,079 → 26,296 (+596 −1,379); `crates/server/tests` +330 −301.

## Collisions

- **`Socket`**: `actor/contracts.rs` (19.2) and `actor/ws_server.rs` (19.3), with `SocketFrame` twice.
- **The server clock**: `app::now_ms` (18.1), `actor::clock::now_ms` (19.2), private copies in
  `api/queue.rs`, `api/results.rs`, `api/rematch.rs` (19.1) and `cli/mint_code.rs` (20.5).
- **An empty in-memory store**: `db::store::Db::fake()` over a missing `FakeData::default` (20.4),
  `db::fake::create_memory_store` + `MemoryStoreOptions` (20.3), and `FakeData::default()` literals in
  `app.rs` and `tests/support/deps.rs`.
- **`Registry::bind`/`app()`** (19.2) against handlers that take `&Arc<App>` (19.1, 19.3): two ways to
  reach the `Arc`.
- **TS `createFakeEngine`**: `support::engine::install_test_cards` (19.1) against the tests'
  `create_fake_engine`/`FakeEngineOptions` (19.4–19.6).
- **Error helpers** (fullsend rule 5 copies): `api_error` ×8, `bad_request` ×4 (+ `http::bad_request`),
  `internal` ×4, `store_failure` ×3, `hide_internal` ×2 (each re-did `dispatch`'s hiding),
  `caller_profile` ×8 (+ `collection::caller_profile`), `rate_limited` ×2, `ok` ×2.
- **Other helpers**: `new_uuid` ×3, `new_seed` ×3, `lock` ×5, `json_list` ×2, `mode_name` ×2
  (`QueueMode::as_str`), `other` ×2 (`PlayerId::opponent`), `js_number` ×4, `is_integer` ×2,
  `json_text` ×3 + `quoted` ×2, `literal` ×2 (CLI), `is_uuid` ×2, `is_record` ×2, `count_number` ×2,
  `to_public_player_summary` ×2, `fail` ×2, `assert_postgres_url` ×2, `pad_to` ×2,
  `load_patch_versions` ×2, `owned_in` ×2.
- Not collisions (two TS modules had them too, or different concepts): `begin_game`, `seat_of`,
  `initial_clocks`, `json`, `error_message`, `seeds`, `profile_of`, `series_winner_of`, `literal` (pg's
  `Result` form), the 87 store methods in `store.rs`/`pg.rs`/`fake.rs` (the dispatch by design).

## Seams

- Handlers `&App` (SURFACE §11.2) vs `Registry::start(&Arc<App>)`: `h!`/`Handler` built for `&App`
  while `enqueue`, `offer_rematch`, `rooms::join` took `&Arc<App>`.
- `StoreError::{Duplicate(String), Db, Other}` (store.rs) vs `Duplicate { match_id }` (fake.rs) and
  `DuplicateResult` (results.rs).
- `db::Profile`/`db::Db` (SURFACE §11.2) with nothing re-exported from `db/mod.rs`.
- `pg::profiles_create` and `pg::player_stats_list_public` spread TS's anonymous inputs into
  arguments where `store.rs` passes `&ProfileCreateInput` / `&PlayerStatsListOptions`; the fake used
  its own `ListPublicOptions`/`CardCount` names; `fake::begin` answered a `Result` the store did not
  expect.
- `mint_invite_code(MintDeps, MintInput)` (codes.rs) vs `(&Db, &Hashes, MintInviteCodeInput)` (the
  CLI and two tests).
- `rate_ranked_game(&RankedGameInput)`, `advance_series_in_tx(&SeriesGameResult)`,
  `resume_series(Option<&SeriesRow>)`, `start_series(NewSeriesInput)` called with the other reference
  kind; `RatedReason: From<SeriesEnd>` assumed (19.2); `rate_limit_address` called with two arguments.
- `api::loadout_validator::TRIO_DECKS` (no such module; the validator owns it).
- Tests: the actor's sync methods awaited (103 sites), `ClockView` by value (26), `call`'s body as an
  `Option` (17), `add_user` without its fourth argument (8), `SupabaseAuthInput.now`, `Catalog.banned`,
  `TestAppOptions.{e2e, db}`, `FakeData.game_records`, `on_call` as a `Box`, `FakeSocket` held by `&`
  while its methods took `&mut self`, `with_cors` as a layer.

## Drift

- `jsonwebtoken` without a crypto provider (panics on the first verify): `features = ["rust_crypto"]`
  in the root `Cargo.toml` (and `Cargo.lock`).
- `AiDeckOptions.banned: Option<Vec<String>>` (part 17) vs `vec![]` (19.2).
- `TrioSlots`/`FrozenTrio.decks` as `[T; 3]` (store.rs) vs tuples in every caller (SURFACE §4.3 says
  tuples).

## Semantic conflicts

| Key | Values | Parts | Settled |
|---|---|---|---|
| `store.ints` | caps `usize` / `i32`; counts and epoch ms `i64` | 18.2, 20.1, 20.2, 20.3 vs 20.4 | the store's `i64` for every count, cap, position and epoch ms (store.rs owns them; SURFACE §4.3 silent) |
| `null.optional` (`x?: T \| null`) | `Option<T>` (absent and null one) / `Option<Option<T>>` | 20.1, 20.3 vs 20.4 | the store's three states: TS's rows keep `displayName: null` apart from absent |
| `clock.server` / `time.server` | `app::now_ms` / `actor::clock::now_ms` / per-file copies | 18.1, 18.3, 18.5, 19.3 vs 19.2, 19.1 | `app::now_ms` (tokio-anchored) |
| series row widths | `usize` slots and picks (19.1) / `i64` (20.4) | 19.1 vs 20.4 | the store's `i64` in rows; the rules convert at the row |
| `error.style` | `ApiError` built by hand and hidden per handler / hidden by `dispatch` | 18.x, 19.x | `dispatch` hides an internal error once; handlers return `ApiError::internal(message)` |
