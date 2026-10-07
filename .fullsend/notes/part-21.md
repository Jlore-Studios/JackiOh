# Slice: part 21, the WASM bindings and the web client on them (#409)
BUILDS-RUN: 0 (no cargo, rustc, rustfmt, clippy, pnpm, tsc, vitest or Cypress. Python and grep only, to copy files and rewrite import lines.)

## FILES
Rust and scripts:
- `crates/wasm/src/lib.rs` — SURFACE §10.1, every binding (init, catalog, catalog_version, create_game,
  begin_game, reduce, legal_actions, view_for, seat_to_act, hash_state, fold, last_board_for,
  seat_played_by, ai_to_act, ai_decide, build_ai_deck, validator, constants, find_instance,
  choose_action, engine_tables).
- `scripts/build-wasm.sh` — SURFACE §10.2 (idempotent; wasm-bindgen-cli 0.2.129 tarball into .cache/bin).
- `crates/engine/tests/export_config.rs` — writes `apps/web/src/wire/engineConfig.ts`.
Web, new:
- `apps/web/src/wasm/index.ts` — SURFACE §10.3 (loadWasm, loadWasmSync, wasmLoaded, whenWasmLoaded, one
  typed function per binding under its TS name).
- `apps/web/src/wire/{index,catalog,codes,emotes,aim,stats,rng,engine,engineConfig,validator,cards,ai}.ts`,
  `wire/generated/.gitkeep`, `wire/codes.test.ts` (the shared code-input fixture, R191).
- `apps/web/src/practice/personas.ts`, `personas.test.ts` (copies; the import-isolation describe dropped).
Web, changed: `game/engine.ts`, `game/engine.real.ts` (deleted), `practice/{core,practice.worker,host}.ts`,
`tutorial/lessons.test.ts`, `cards/flavour.ts`, `main.tsx`, `game/net.ts`, `test/setup.ts`,
`vite.config.ts`, `vitest.config.ts`, `tsconfig.json`, `package.json`, and the 49 web files that imported
`apps/server/src/config.ts` (now `@jackioh/server-config`).
e2e, changed: `support/tasks/replay-runner.ts`, `support/tasks/lessons-runner.ts`, `tsconfig.json`,
`cypress/component/tsconfig.json`, `cypress/component/{radiant-marks,card-faces,keyword-visuals,deckbuilder-layout,landing-and-code-field}.cy.tsx`,
`cypress/e2e/99-online-smoke.cy.ts`, the 8 e2e specs that imported the server config.
Deploy: `vercel.json` (`'wasm-unsafe-eval'`, rustup in `installCommand`), `apps/web/public/_headers`.
Outside the table, each a direct consequence of a table row (Decisions below): `game/net.test.ts`,
`routes/match.test.tsx` (the legal-frame cases), `net/deploy-routes.test.ts` (the CSP regex),
`game/hotseat.test.ts`, `audio/test/realGame.ts` (`enginePort` moved), `patches/source.ts`,
`patches/source.test.ts`, `audio/{voiceData,music-assets,voice-lines,gen-voice,voice-assets}.test.ts`,
`routes/landingFan.test.ts`, `e2e/support/commands.ts`, `e2e/support/tasks/replay.ts` (data read from
the byte-identical `crates/cards/` copies), `e2e/cypress.config.ts` (the component tests' aliases).

## SURFACE
Matched as written (§10.1–§10.4), with these choices where SURFACE was silent:
- Every JSON binding returns `Result<String, JsError>` (`ai_to_act` `Result<bool, JsError>`): the JS
  signature is SURFACE's; bad JSON, a bad seat, an unknown validator call, and a setup TS's
  `createGame`/`fold` threw on (`validate_handicap`/`validate_deck`, checked first because Rust's
  `create_game` panics) throw an `Error` with the engine's message.
- `seat_to_act` answers `""` for `None`; the TS wrapper `seatToAct` returns `PlayerId | null`.
- `checkDeckDraft` crosses as `{ name, cards, deckable: string[], portrait?, portraitKnown?, nameMaxLength }`:
  the wrapper answers TS's two predicates for exactly the ids/portrait the rules ask about; the binding
  rebuilds them as closures. `validateDeck`/`validateLoadout` send only the snapshot's defs the decks hold.
- `choose_action` is `jackioh_ai::random_action` (§9: §10.7's policy; TS `chooseAction`'s default skip set).
- `engine_tables`' `heroPowerNames` is `HERO_POWERS.map(name)`, as TS defined it.
- `constants`' `SHADOW_BAN_IDS` is `SHADOW_BAN`'s ids sorted (TS `Object.keys(SHADOW_BAN).sort()`).
- `engineConfig.ts`: 13 constants (SURFACE's 12 + `LIBRARY_CAP`, which `game/animations.window.test.ts`
  imports), each `export const NAME<: TS's annotation> = <JSON>;` (`DIFFICULTIES: readonly Difficulty[]`,
  `AI_DIFFICULTY: Readonly<Record<Difficulty, Handicap>>`, `AI_TUTORIAL`/`HUMAN_HANDICAP: Handicap`), the two
  types re-exported from `./generated/{Difficulty,Handicap}.ts`. Committed now, as the test would write it.
- `wire/index.ts` re-exports each generated type by name (`./generated/<Name>.ts`, the 51 TS shared types
  that are Rust types), never `export *` (part 5.3/5.4's TS2308 warning); `CardDefs`, `BackrowView | null`
  (part 1's GAP), `DistributiveOmit`, `GAME_EVENT_TYPES` + its exhaustiveness type by hand; catalog-types'
  10 runtime values by name from `./catalog.ts`; `export *` of the hand-kept codes/emotes/aim/stats.
- `wire/engine.ts` also exports `seatToAct`, `findInstance`, `fold`'s types (and `ReplayInput`/`ReplayResult`
  aliases), `CardInstance`; `subsystems` is an object with lazy getters plus `chooseAction(state, seat, rng)`.
- `wire/ai.ts` exports SURFACE's list plus the personas' whole surface (`AI_PERSONAS`, `EMOTE_TRIGGERS`,
  `EMOTE_REPLY_KEYS`, `rollForTrigger`, `rollForReply`, `replyKeyOf` and their types) and the AI types
  (`SearchBudget`, `AiOptions`, `Decision`, `DecisionReason`, `SearchStats`, `AiDeckOptions`).
- Aliases: `jackiohAliases` in `apps/web/vite.config.ts`, exact-match regexes; `@jackioh/ai/config` and
  `@jackioh/ai` both → `wire/ai.ts`; `@jackioh/cards/{catalog,flavour}.json` → `crates/cards/`.

## DEPENDS-ON
Engine (root re-exports): `create_game(&CreateGameArgs)`, `begin_game(&GameState)`, `reduce(&GameState,
&Action)` → `ReduceResult: Serialize`; `legal_actions`, `view_for`, `seat_to_act -> Option<PlayerId>`,
`hash_state`, `fold(&FoldArgs) -> FoldResult` (`FoldArgs: Deserialize`, unknown keys ignored; `FoldResult:
Serialize`), `last_board_for`, `seat_played_by`, `find_instance`, `registered_catalog`, `validate_handicap`,
`validate_deck` (state.rs), `PerPlayerOpt::get`, `PLAYER_IDS`, `DECK_SIZE`, `Rng::new`/`cursor`.
`subsystems::{call_to_chaos::CHAOS_EFFECTS, call_to_chaos_plus::CHAOS_PLUS_EFFECTS}` (`.label`),
`subsystems::hero_power::HERO_POWERS` (`.name: Serialize`, `.x`, `.title`, `.radiant_title`, `.label`,
`.radiant_label`).
Validator (part 5.3): `validator::{validate_deck(&DeckInput), validate_loadout(&LoadoutInput),
check_trio_draft(&TrioDraftInput), check_import_room(&ImportRoomInput), trio_conflicts(&[LoadoutDeck]),
normalize_name(&str) -> String, check_deck_draft(&DeckDraftInput)}`, inputs `Deserialize`, answers
`Serialize`; `DeckDraftInput { name: String, cards: Vec<String>, is_deckable: &dyn Fn(&str) -> bool,
portrait: Option<String>, is_portrait: Option<&dyn Fn(&str) -> bool>, name_max_length: <any int> }`.
AI (part 17, SURFACE §9): `decide(&GameState, PlayerId, &mut AiOptions { rng, budget, should_stop:
Option<&dyn Fn() -> bool> }) -> Option<Decision>` with `Decision: Serialize` (TS's JSON: `reason`
"draw-offer" etc.), `SearchBudget: Serialize + Deserialize` (camelCase), `AI_BUDGET`, `AI_GATE_BUDGET`
(consts), `AiDeckOptions: Deserialize` (camelCase; `theme` absent vs null kept by its own serde),
`build_ai_deck(&mut Rng, i32, &AiDeckOptions) -> Vec<String>`, `ai_to_act`, `random_action`,
`SHADOW_BAN: &[(&str, &str)]`.
Cards: `jackioh_cards::{register_all, catalog_version}`.
Server (part 18): `apps/web/src/wire/serverConfig.ts` must export all 50 names the web and e2e import:
API_REQUEST_TIMEOUT_SECONDS AUTH_EMAIL_RESEND_COOLDOWN_SECONDS AUTH_PASSWORD_MAX_LENGTH AUTH_PASSWORD_MIN_LENGTH
AUTH_PENDING_ADDRESS_TTL_SECONDS AUTH_PROVIDER_TIMEOUT_SECONDS AUTH_SESSION_REFRESH_MARGIN_SECONDS
AUTH_SESSION_RENEWAL_FLOOR_SECONDS AUTH_SIGN_OUT_WAIT_SECONDS CATALOG_NUMBER_SET_OFFSETS CODE_ALPHABET
CODE_ATTEMPTS_PER_PROFILE_PER_HOUR CODE_ATTEMPT_RETENTION_DAYS CODE_ATTEMPT_WINDOW_SECONDS
CODE_STATUS_RECHECK_FLOOR_SECONDS DECK_AUTOSAVE_DEBOUNCE_MS DECK_AUTOSAVE_RETRY_SECONDS
DECK_CODE_CORE_ONLY_VERSION DECK_CODE_MAX_INPUT_LENGTH DECK_CODE_VERSION DECK_NAME_MAX_LENGTH
DISCONNECT_GRACE_MS DISCONNECT_GRACE_SECONDS GATE_SLOW_NOTICE_SECONDS INVITE_CODE_FORMAT
INVITE_CODE_GROUP_SIZE INVITE_CODE_LENGTH INVITE_CODE_SEPARATOR MATCHMAKER_SWEEP_INTERVAL_SECONDS
MATCH_ACTION_RETENTION_DAYS MATCH_CEILING_MS MATCH_FOUND_NAV_DELAY_MS MAX_SAVED_DECKS MAX_SAVED_TRIOS
MULLIGAN_CLOCK_MS PROMPT_CLOCK_MS PROMPT_CLOCK_SECONDS RATING_WINDOW_UNCAPPED_AFTER_SECONDS
REDEMPTION_IDENTICAL_ERROR REDEMPTION_RESPONSE_FLOOR_MS ROOM_CODE_LENGTH SERIES_MAX_GAMES SERIES_PICK_SECONDS
SERIES_POLL_SECONDS SERIES_WINS_NEEDED TRIO_CODE_CORE_ONLY_VERSION TRIO_CODE_MAX_INPUT_LENGTH TRIO_CODE_VERSION
TURN_CLOCK_MS TURN_CLOCK_SECONDS (part 18.1's notes say 42; the test files account for the rest).
Tools (part 22): `target/release/jackioh replay` reads `{seed, decks, log, handicaps?, dealt?}` on stdin,
prints `{"hash", "errors"}` as its last stdout line, exit 0.

## GAPS
- The Done-when grep is not empty: 77 lines in 44 files outside the table still name `packages/` or
  `apps/server/src`. 74 are comments or message strings (e.g. `e2e/support/types.ts`, `wsPlayer.ts`,
  `game/actions.ts`, `routes/dev/hotseat.tsx`'s stale "packages/engine re-exports six modules"); a
  mechanical sweep to `crates/` paths (port-map.tsv) is part 36/37's. Three read TS source and need
  rework, not a path: `net/env-production.test.ts:45` (reads `apps/server/src/env.ts`),
  `game/v020-real.test.tsx:13` (imports the TS `packages/cards/test/_harness` and runs the TS engine),
  `game/deckbuilder/messages.test.ts:23` (reads `packages/validator/src/index.ts` for its sentences).
- `pnpm-lock.yaml` still links `@jackioh/{ai,cards,engine,shared,validator}` into `apps/web`:
  `pnpm install --frozen-lockfile` (Vercel's installCommand, CI) fails until `pnpm install` rewrites it.
- The `pre*` hooks need pnpm to run pre/post scripts (pnpm ≥ 8's default `enablePrePostScripts: true`;
  if pnpm 11 differs, set it in pnpm-workspace.yaml). The root `pnpm test`/`typecheck` do not run
  `apps/web`'s hooks: CI must run `sh scripts/build-wasm.sh`, the ts-rs export and both `export_config`
  tests before `tsc -p apps/web` and vitest (part 30's notes already order them).
- `e2e/support/component.tsx` (or each spec) must `await loadWasm()` for any component spec that calls
  the validator or the engine; done in `deckbuilder-layout.cy.tsx` only.
- `promote-production.yml`'s Cloudflare build needs Rust too (part 30 says its setup action installs it).
- `practice/config.ts`'s `PRACTICE_AI_CLOCK_MS` doc still says `AiOptions.shouldStop` (now `deadlineMs`).
- Practice saves on devices made by the TS engine resume only if `hash_state` equals TS's (SURFACE §5.2);
  the catalog check refuses them only on a version bump.
- A panic in WASM traps; the wrapper rethrows it as "the engine panicked", but the instance is not
  re-created, so later calls may misbehave (wasm-bindgen's glue cannot re-init).
- `crates/wasm/src/lib.rs` is unbuilt: every DEPENDS-ON shape above is a guess where no note confirmed it
  (AiOptions' fields, Decision/SearchBudget/AiDeckOptions serde, DeckDraftInput's field types).

## Decisions
- The practice core keeps calling `decide`/`reduce` by their `@jackioh/ai`/`@jackioh/engine` names (now
  `wire/ai.ts`, `wire/engine.ts` over `../wasm`), not `../wasm` directly as the table says: the unchanged
  `core-fallback.test.ts` and `core-glitch.test.ts` `vi.mock` those two specifiers. `wire/ai.ts`'s `decide`
  is the brief's mechanism: `wasm.decide(state, seat, {rngSeed, rngCursor, budget}, deadlineMs)`, then the
  caller's `Rng` drawn forward to the returned cursor (`advanceTo`), so `rng.cursor` (the R668 save) holds.
- The clock: `AiOptions.shouldStop` (a callback) became `deadlineMs`; `PracticeCoreEnv.now` is `Date.now`
  (worker and in-thread host), and a `now()` of 0 is no clock, so the tests' frozen `() => 0` still never
  stops a search.
- The in-thread host awaits `loadWasm()` (a no-op under jsdom after setup's `loadWasmSync`), not
  `loadWasmSync` itself: it has no bytes, and it also serves browsers without module workers.
- `AI_BUDGET`, `AI_GATE_BUDGET`, `SHADOW_BAN_IDS` are live `export let` bindings filled on load, so no module
  touches WASM while being imported (the worker imports core before its first message awaits the load).
- `CATALOG_VERSION` (wire/cards.ts) is the newest entry of `crates/cards/patches/patches.json`, the same
  source `jackioh_cards::catalog_version()` compiles in; JSON, so it is a constant at import time.
- `LoadoutDeck`, `LOADOUT_DECKS = 3`, the validator's types are a TS copy in `wire/validator.ts`.
- `main.tsx` renders after `loadWasm()` settles, and also when it fails (logged): landing and sign-in need
  no engine.
- The worker answers every request `failed` when the module cannot load.
- `net.ts`: the `legal` frame type, parser case and handler are gone, `LegalSource` is `"none" | "view"`;
  a view without `legal` still keeps the previous list. Its tests: "accepts a separate legal frame"
  deleted, "does not blank" now sets the list with a view; `match.test.tsx`'s "comes alive on a legal
  frame" now sends a view carrying the list. Comments name `crates/server/src/actor/*.rs`.
- e2e specs import `../../../apps/web/src/wire/serverConfig.ts` by path (Cypress bundles e2e specs without
  the client's aliases); component specs use the aliases, which `e2e/cypress.config.ts` now imports from
  `apps/web/vite.config.ts` (SURFACE §10.4 names "the component-test config").
- `replay-runner.ts` spawns the CLI for the fold and hashes the browser's state with a TS copy of §5.2's
  canonical + FNV (hashing decides no rule); it no longer reaches outside e2e/, so e2e/tsconfig checks it.
- `99-online-smoke.cy.ts`: all five `{ deckIndex: 1 }` bodies (the brief named four) send
  `{ mode: "bo1", deckId }`, `deckId` = `GET /api/decks`'s `decks[1].id` (legacy index 1, oldest first).
- Web files that read `packages/cards/catalog.json` or `packages/cards/patches/` read the byte-identical
  `crates/cards/` copies (checked with cmp), so part 37's delete breaks nothing there.
- `CodeField.test.tsx` reads `crates/engine/tests/fixtures/code-input-cases.json` (typed locally).
- The copied shared helpers keep their code byte for byte; only import paths, a provenance header and the
  comments that named `packages/`/`apps/server/src` paths changed.
- vercel.json installCommand: rustup (minimal, `--default-toolchain none`), `rustup toolchain install`
  (rust-toolchain.toml), the wasm target, then `pnpm install --frozen-lockfile` (244 characters).
