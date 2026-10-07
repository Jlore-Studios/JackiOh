# Cull report (fullsend Phase 6, part 37)

Staging `11518b0` → `36bad47`. Method: `.claude/skills/fullsend/agents/culler.md`. Candidates came from a
token count of every `pub` item in `crates/*/src` across all Rust files (comments stripped) plus the
web's sources for wasm-bindgen names, from rustc's own dead-code list with `patches.rs`'s
`#![allow(dead_code)]` lifted, and from `.fullsend/damage.md`'s collision list. Each crate was
deleted from, then `cargo clippy --workspace --all-targets -- -D warnings` (with and without the
engine's `testkit,ts`) and `cargo test --workspace --features jackioh-engine/testkit,jackioh-engine/ts`
ran green; `apps/web/src/wire/` came out unchanged (V20). Stopped where the next candidates save
under ~20 lines each.

## Deleted

- `crates/server/src/config.rs` `SERVER_CONFIG` — TS's frozen boot-log snapshot; nothing logs it, the named constants stay — 205
- `crates/engine/src/turn.rs` `trigger_order` + `has_hook` — the turn loop calls `triggers::run_hooks_in_trigger_order`; card comments repointed — 49
- `crates/server/src/env.rs` `server_env` + `CACHED_ENV` — `main.rs` and the CLIs call `load_env` themselves — 12
- `crates/server/src/env.rs` `js_trim`'s own `is_js_space` — damage.md collision; calls the engine's — 17
- `crates/engine/src/prompts.rs` `default_work_handler` — `work.rs` calls `run_resume` itself — 22
- `crates/engine/src/instance_view.rs` `InstanceData::apply_to` — unreferenced — 21
- `crates/engine/src/validator.rs` `is_js_space` + `JS_SPACE` — damage.md collision; uses `wire::codes::is_js_space` — 20
- `crates/engine/src/modifiers.rs` `due_start_of_turn_effects` — unreferenced — 15
- `crates/engine/src/wire/stats.rs` `CardStats::tally_mut` — unreferenced — 14
- `crates/engine/src/zones.rs` `dormant_backrow_of` — unreferenced — 10
- `crates/engine/src/cost_rules.rs` `cost_rule_modifier_label` — unreferenced — 10
- `crates/server/src/api/catalog.rs` `load_current_patch` — unreferenced — 17
- `crates/server/src/actor/protocol.rs` `CLIENT_MESSAGE_TYPES`, `SERVER_MESSAGE_TYPES` — TS lists nothing reads — 16
- `crates/server/src/actor/ws_server.rs` `close_all` + `CLOSE_GOING_AWAY` — unreferenced — 15
- `crates/server/src/api/crypto.rs` `safe_equal` — unreferenced — 13
- `crates/server/src/api/e2e.rs` `session_of`, then `auth_user_of` (dead once `session_of` went) — unreferenced — 27
- `crates/server/src/api/http.rs` `deck_list` — unreferenced — 9
- `crates/server/src/api/settings.rs` `utf16_len` — damage.md collision; uses the engine's — 5
- `crates/server/src/actor/match_actor.rs` `panic_message`'s payload reader — damage.md collision; calls the AI's — 8
- `crates/tools/src/patches.rs` `order_entry` + `ENTRY_KEYS`, `rewrite_index`, `FRAGMENT_VERSION`, `Json::as_bool` — dead in the binary and no test calls them — 50
- `crates/tools/src/patches.rs` `js::is_js_space` — damage.md collision; uses the engine's — 8
- `crates/tools/src/agent.rs` `panic_message` — damage.md collision; re-exports the AI's — 9
- `crates/ai/src/match_.rs` `message_of` — a second copy inside the AI; calls `simulate::panic_message` — 10

Lines removed per crate (the four cull commits, `git diff --numstat`): engine −175 +12, server −354 +6,
tools −74 +7, ai −15 +6, cards −7 +8 (two comments repointed from `trigger_order`). Net −586.

## Restored after test failure

None. Every deletion compiled once its cascade (imports and helpers left unused, which clippy named)
went with it, and no test went red.

Considered and left:

- `crates/server/src/db/pg.rs` `create_postgres_store` (23 lines) — unreferenced, but it is the pool
  with the idle timeout TS needed against Supavisor (`POOL_IDLE_TIMEOUT_MS`), and `app.rs` builds a
  bare `PgPoolOptions::new().connect_lazy(..)` instead: a wiring gap, not dead code. Reported, kept.
- `crates/tools/src/patches.rs` `naming` and `versions_at_sites` — no caller in the binary, but their
  own tests call them; a culler does not delete tests. `#![allow(dead_code)]` stays for them.
- `crates/server/src/env.rs` `PUBLIC_ENV_VARS` — unreferenced in Rust, but `apps/web/src/net/env-production.test.ts` reads env.rs for its names.
- `crates/wasm` `ai_decide`, `engine_tables` — the web calls them.
- `crates/ai` `NodeCounter` — the one single-implementation trait (`CountingNodeCounter`); inlining saves under 20 lines and touches every search signature.
- damage.md: `MAX_SAFE_INTEGER` and `PERCENT` (one line each, of different types per crate),
  `whole_number` (engine `Option<i64>`, server `Option<f64>`), the server test's `panic_text` (a
  test helper, under the bar), `crates/cards/tests/cross/registry.rs`'s copy of tools' `naming`
  (needs its tests moved, which a culler may not do), the arena's referee loop beside
  `jackioh_ai::play_match` (needs an external-agent seat in the AI: a design change), the NFKC
  tables, `patches::js::Json` and the SHA-1 (crates exist, but V2 forbids them in pure crates and
  SURFACE §2 names none for the others).
- Under the bar (≤ 7 lines each): `ai::{AiEval, GreedyMulligan, run_gate}`,
  `engine::{SetRadiantArgs, PromptAnswerer, play_still_announced, unit_for_action, active_units,
  all_zones_empty, chaos_effect_by_name, open_quests_of, completed_quests_of, clear_overrides,
  catalog_version_override, FLAG_NUMBERED_KEYS, run_work, TransientHolder}`,
  `server::{CreateMatchClock, all_routes, AuthSession, ServerEnv, clear_rematch_offers,
  rematch_offer_count, start_series_sweeper}`.

## Behaviors with no test

docs/v0.3.0/README.md §6, after part 37's coverage commit (`36bad47`):

- New tests: V2 (`crates/tools/tests/acceptance.rs`: the pure crates' linked dependencies), V23 (same
  file: part 28's grep for code reading a `.md` file, as a test), V12 (`crates/ai/tests/ai/shadow_ban.rs`:
  generation 0 holds TS's eleven entries).
- Proven already, now cited: V1, V5, V6, V7, V8, V9 (ci.yml, super.yml), V9 and V15
  (`crates/server/tests/actor/recovery.rs`), V11 (`crates/ai/tests/ai/observe.rs`), V13
  (`crates/server/tests/api/mod.rs`), V14 (`crates/server/tests/actor/mod.rs`, e2e 05 and 06), V18
  (`crates/server/tests/api/catalog.rs`), V19 (e2e 01 and 13), V26 (`crates/tools/src/promote.rs`).
- Cited before: V3, V4, V10, V16, V17, V20, V21, V22, V24, V25.
- No test: V27 (a training lane survives a reboot: part 39's check on the box, a person's). V28 (no
  TypeScript under `packages/` or `apps/server/`, no `ladder/`): the TypeScript deletion is not on
  staging yet (see the part 37 report), so a test of it would be red; it belongs with that commit.

## Lines

Rust under `crates/`: before 446,351; after the cull 445,765; after the coverage tests 446,036.
