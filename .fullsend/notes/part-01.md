# Slice: part 1, bootstrap: the Cargo workspace, the module tree and the type freeze
BUILDS-RUN: part 1 may build (brief §1). Two sessions; the first was cut off by a container restart
after `de242d2` and the work-in-progress commit `6633d86`, the second finished. Runs of the second:
`cargo check -p jackioh-engine` and `cargo check --workspace` (about 8), the damage run, and in a
scratch copy of the workspace with one-line stubs for the 16 names other parts write (never
committed): `cargo test -p jackioh-engine --lib` (17 pass), `cargo test -p jackioh-cards --lib`
(2 pass, plus a scratch differential test: `fill_params`, `param_placeholders`, `keyword_key` and
`armor_of` equal TS's on all 318 catalog cards), `cargo test -p jackioh-engine --features ts
export_bindings` (148 files) compared with the TS originals by `tsc --strict` (below), `cargo test
-p jackioh-engine --features testkit --test rules` (part 28's 29 config asserts pass), `cargo
clippy` (nothing on part 1's files after two `allow`s), `cargo check --workspace --all-targets
--keep-going`, `cargo fmt`. Node: `node --experimental-strip-types` on `packages/engine/src/rng.ts`
(the pinned draws) and on `catalog-types.ts` (the differential test); `tsc` 6.0.2 for the type
comparison. No `pnpm install`.

## FILES
Root: `Cargo.toml`, `Cargo.lock`, `rust-toolchain.toml`, `rustfmt.toml`, `.cargo/config.toml`,
`.gitignore` (target/, .cache/, apps/web/src/wasm/pkg/).
Manifests and purity: `crates/{engine,cards,ai,wasm,server,tools}/Cargo.toml`,
`crates/{engine,cards,ai}/clippy.toml` (SURFACE §3 verbatim).
Module roots (written once, SURFACE §1): `crates/engine/src/{lib.rs, prelude.rs, wire/mod.rs,
effects/mod.rs, subsystems/mod.rs, testkit/mod.rs}`, `crates/engine/tests/{rules.rs, rules/mod.rs,
rules/fixtures/mod.rs, golden.rs}`, `crates/cards/{build.rs, src/lib.rs, tests/cards.rs,
tests/cross/mod.rs}`, `crates/ai/{src/lib.rs, tests/ai.rs, tests/ai/mod.rs}`, `crates/wasm/src/lib.rs`
(empty), `crates/server/src/{main.rs, lib.rs, api/mod.rs, actor/mod.rs, ranked/mod.rs, db/mod.rs,
cli/mod.rs}`, `crates/server/tests/{server.rs, api/mod.rs, actor/mod.rs, store/mod.rs, support/mod.rs}`,
`crates/tools/src/main.rs`.
The freeze: `crates/engine/src/{state.rs, script.rs, config.rs, rng.rs}`,
`crates/engine/src/wire/{actions.rs, catalog_types.rs, events.rs, view.rs}`.
Placeholders: an empty file for every other `.rs` destination of port-map.tsv under `crates/` (554
destinations checked: every one exists and is declared by its parent; card scripts excepted), plus
`crates/server/src/auth.rs` (SURFACE §11.1) and `crates/tools/src/{agent,golden,promote,replay}.rs`
(SURFACE §12).
Data, byte for byte (`cmp`): `crates/cards/{catalog.json, flavour.json, patches/**}` (25 files),
`crates/server/migrations/*.sql` (26 files).
Run scratch: `.fullsend/README.md`, `.fullsend/{notes,damage}/.gitkeep`, `.fullsend/waves.md`,
`.fullsend/damage/part-01.txt`, this file and `part-01.assumptions`.

## SURFACE
Matched as written, apart from the decisions below. Every name other parts copy is in the files
above; this section lists what SURFACE.md did not decide.

### Manifests
- `reqwest` takes feature `rustls`, not `rustls-tls`: reqwest 0.13 has no `rustls-tls` (its
  `default-tls` is `["rustls"]`). SURFACE §2's row is wrong.
- `ts-rs` adds features `indexmap-impl` and `serde-json-impl` (an `IndexMap` or `Value` field cannot
  derive `TS` without them).
- `[workspace.package]`: version 0.3.0, edition 2024, rust-version 1.97.0, publish false; the three
  path crates are in `[workspace.dependencies]` too.
- `jackioh-tools` and `jackioh-server` depend on `indexmap` (§11.2's `Req` holds `IndexMap`s; §2's
  rows do not list it).
- `crates/engine`: `[[test]] rules` has `required-features = ["testkit"]`; `[[test]] golden`.
  `tests/rules.rs` and `tests/ai.rs` declare `#[path = "rules/mod.rs"] mod rules;` (resp. `ai`),
  because `tests/rules.rs` and `tests/rules/mod.rs` would both answer `mod rules;`.
- `jackioh-cards` has a build-dependency on `serde_json`.

### Root re-exports (`lib.rs`, `prelude.rs`, `testkit/mod.rs`, the barrels)
- `lib.rs` globs EVERY top-level engine module (`pub use reduce::*;` …), not only the 47 that
  `index.ts` re-exported, and `wire::*`, so `crate::<name>` and the testkit's `pub use crate::*`
  name every engine item (SURFACE §8). TS kept names distinct across the index modules; across all
  engine modules the TS sources have exactly one collision (`PlayAction`, in `playChoices` and
  `playSteps`), and two with the wire (`UnitView` in `layers`, `HeroView` in `query`). `lib.rs`
  resolves the three explicitly in `@jackioh/engine`'s favour: `crate::UnitView` is `layers`',
  `crate::HeroView` is `query`'s, `crate::PlayAction` is `play_choices`'. The view's are
  `wire::UnitView` and `wire::HeroView`. A new pub name that two engine modules both define is an
  `ambiguous_glob_reexports` warning and an error only where someone names it through the root.
- `validator` stays a module, never globbed (TS's own package; its `validateDeck` is not
  `state.ts`'s): `jackioh_engine::validator::validate_deck`.
- `subsystems::glitch` is declared, not globbed (TS's barrel left it out); the root re-exports
  `seat_played_by`, `seats_swapped` and `last_board_for` (SURFACE §6.1).
- `prelude.rs` globs the modules card files import from (research A §5.3 measured by import):
  animated, book_swap, cast_on_draw_now, catalog, combat, config, cost_rules, faces, graveyard_play,
  kill_credit, layers, mana, numbers, params, plague, prompts (`RESUME_HOOK`, `summoned_so_far`),
  query, replacements, restrictions, rng, script, state, times_played, traps, tuning, zones,
  `effects::*`, `wire::*`; `subsystems` as a module; `json!`, `Value`, `IndexMap`, `Arc`, `json_as`.
  Left out because their names collide with effect verbs a card means: `draw` (its `draw`,
  `add_to_hand`), `turn` (`end_turn`), `damage` (`lose_health`). `mana` stays in and its
  `gain_mana`/`refresh_mana` are resolved to the effects' explicitly. `json_as` panics with the
  type name and the JSON.
- `testkit/mod.rs`: `pub use crate::*;` plus `scenario::*`, `glow::*`, `invariants::*`, then
  `pub use scenario::{register_catalog, register_scripts};` so the thread-local override wins over
  the production `catalog::register_catalog`/`scripts::register_scripts` that `crate::*` also
  brings (part 5 writes both in `scenario.rs`). Also `json!`, `Value`, `serde_json`, `IndexMap`,
  `IndexSet`, `json_as`. The effects library is NOT globbed into the testkit (its verbs collide with
  engine functions); a test imports the verbs it uses. A card file's `mod tests` that does
  `use super::*; use jackioh_engine::testkit::*;` gets an ambiguity error only if it names one of
  the six colliding verbs bare (`draw`, `add_to_hand`, `end_turn`, `gain_mana`, `refresh_mana`,
  `lose_health`).
- `effects/mod.rs` globs every module (TS's barrel was selective; no two effects modules share a
  name), and also re-exports what TS's barrel took from elsewhere: `LAST_BOARD_CARD_COST`,
  `LAST_BOARD_DISCOVER_OPTIONS` (now in config) and `subsystems::{perfect_hand::
  replace_hand_with_perfect, board_history::roll_back, glitch::glitch}`.
- `subsystems/mod.rs` re-exports from `crate::config` every number its modules stated in TS
  (`FUSE_MIN_INGREDIENTS`, `CHAOS_*`, `SCORER_*`, the Heroic Power numbers, `FIRST_GRADE`, `GRADE_*`,
  `AI_PLAYOUT_STEP_CAP`, `CRAFTED_CARD_COST`), so TS's `subsystems.X` path still resolves.

### Types placed outside their TS module (owners: use these, never redefine them)
- `state.rs`: `EngineError` (SURFACE §6.2: `new`, `Display`, `Error`, `From<String>`, `From<&str>`),
  `CostRule` (TS costRules.ts, part 4), `EventStay` (TS stays.ts, part 2).
- `script.rs`: `EngineSink` (TS resolve.ts, part 3; `EngineSink::new(state, events, rng)`,
  `reborrow()`), `CostAura`, `CostAuraWhose`, `CostAuraArgs` (costRules.ts, part 4),
  `GraveyardPlayPermission` (graveyardPlay.ts, part 4), `QuestGoal`, `QuestDef`, `QuestRewardDef`,
  `QuestBook` (subsystems/quests.ts, part 8), `ReplacementDef`, `ReplacementMoment`,
  `ReplacementWhere`, `ReplacementContext`, `ReplacedEvent`, `TargetedReplacement` (= `ReplacementDef`),
  `ReplacementInstead`, `ReplacementWhen`, `ReplacementBy`, `HealedRef`, `DyingUnit`, `TargetedWhat`,
  `Instead*` (replacements.ts, part 3).
- `wire`: `Phase`, `Position`, `Winner`, `GameResult` (state.rs re-exports them), `PerPlayer`,
  `PerPlayerOpt` (catalog_types.rs). A module that wants them by TS path writes `crate::state::Phase`
  and it resolves.

### Names for TS's anonymous types (other parts use these names)
state.rs: `Exertion`, `KnownAs`, `BrittleCounter`, `ModifierExpiry`, `ModifierKind` (the union half
of `PlayerModifier`, `#[serde(flatten)]` beside `id`/`expiry`), `DelayedAt`, `TurnLog`, `HeroState`,
`ManaState`, `DrawOfferState`, `DrawCount`, `GameCounters`, `AppliedAction`, `RowSlots` (`row_of`'s
answer), `NextId` (trait), `CreateGameArgs` (= `CreateGameOptions`). wire: `AttackHealth`, `Counters`,
`RowFlags`, `ZoneName` (`Zone["z"]`), `FaceKind` ("base" | "radiant"), `OneOrMany<T>`, `CostRange`,
`FilterOf`, `FilterSide`, `TargetAim`, `ParamBetter`, `ParamTunedOn`, `ParamPlaceholder`, `CounterKind`,
`CounteredTo`, `PlayedFrom`, `RedirectWhat`, `RotationDirection`, `SwapWhat`, `GlitchOutcome`,
`AnimatedView`, `HandView`, `ManaView`, `DrawOfferView`, `QuestItemView`, `QuestOpenView`,
`BackrowCounters`, `PublicBackrowView`, `FaceDownBackrowView`, `PendingPromptView`,
`PendingElsewhereView`, `ActionType`, `ActionInput`, `GameEventType`. script.rs: `HookArgs`,
`CostArgs`, `SetStat`, `AuraEntry`, `FlagOrCount`, `ActivationUses`, `ActivationCost`, `DrawLimitPlayer`,
`TargetCheckArgs`, `WouldCounterArgs`, `HeroGuard`, `ConditionZone`, `EffectApply`, `EffectExpand`,
`Memo`, `TriggerWhen`, `TriggerRun` and the hook aliases (`CostHook`, `SetStatHook`, `AuraHook`,
`ConditionHook`, `PreviewHook`, `CostAuraHook`, `GraveyardPlayHook`, `TargetingDiscardsHook`,
`RecordsPlayAsHook`, `DrawLimitHook`, `HeroGuardHook`, `ConditionalKeywordsHook`,
`PlagueMultiplierHook`, `TributeWhenHook`, `WouldCounterHook`, `HasHook`, `TargetCheck`). config.rs:
`ByFace` (`ANTI_ONESHOT_CAP`, `QUICKSTRIKER_COMBO_MULTIPLE`; `.on(radiant)`), `HeroArmor`,
`HeroArmorPrice`, `AiDifficulty` (`Index<Difficulty>`), `ParamDefaultStep`, `ParamStepBand`,
`MarkSpec`, `IntRange`, `KyTestDifficulty`, `KyTestReward`, `KyTestRewardPool`, `KyTestRewardCount`,
`KyTestRewards` (`.of(difficulty)`), `GrapeOdds`, `ScorerWeights`, `TrainingGate`.

### Hooks and the context (SURFACE §6.6)
- `Hook = Arc<dyn Fn(&mut EffectContext<'_>) -> Vec<Effect> + Send + Sync>` — `&mut`, where SURFACE
  writes `&EffectContext`: a dozen TS card hooks draw from `ctx.rng` while they build their list
  (Classic #10, #65, C+ #7, #8, #19.2, #19.3, #25, #37, #53, Core #23, #83). A card writing
  `hook(|ctx| …)` is unaffected; a helper taking `&EffectContext` still accepts `ctx`.
- `EffectContext<'a> { sink: EngineSink<'a>, events_from, exits_from, chosen_from, event_stay,
  summoned, self_resolving, controller, self_: Option<CardInstance>, def_id, radiant, targets, modes,
  x, embiggened, data, mana_before_play }`; it `Deref`s to its `EngineSink`, so `ctx.state`,
  `ctx.rng`, `ctx.events` read as in TS and a `&mut EffectContext` passes where `&mut EngineSink` is
  taken. `self_` is the instance as the context was built (TS held the live object); `live_self()`
  reads the card as it stands now. `EffectContext::new(sink, controller)` builds one at rest
  (`resolve::make_context` is the engine's constructor, part 3).
- `Effect::new(kind, apply)`, `Effect::with_expand(kind, apply, expand)`; `EffectPart { effects,
  memo }`; `Memo = Option<Value>`.
- `TriggerDef { id, on: Vec<GameEventType>, when: Option<TriggerWhen>, run: TriggerRun }`; TS's
  `ctx & { event }` is two arguments `(ctx, &GameEvent)`; `TriggerDef::new(id, &[types], run)
  .with_when(…)`.
- Every pure-read hook takes a `Copy` argument bundle by value: `HookArgs<'a> { state, self_,
  radiant }` (aura, setStat, costAura, graveyardPlay, targetingDiscards, recordsPlayAs, drawLimit,
  heroGuard, conditionalKeywords, plagueMultiplier, tributeWhen, ActivationDecl.has),
  `ConditionContext`, `CostArgs { state, instance }`, `TargetCheckArgs`, `WouldCounterArgs`,
  `ReplacementContext`. Builders: `read_hook`, `aura_hook`, `condition_hook`, `cost_hook`,
  `target_check`, `would_counter_hook`, `replacement_when`. `AuraEntry { applies: Box<dyn
  Fn(&CardInstance) -> bool + 'a>, mod_: StatMod }` (the one `Box`: `applies` borrows the hook's
  arguments).
- `Script` derives `Default`; TS optional arrays are `Vec`s (empty = absent), TS optional hooks
  `Option`s, `resume` and `target_checks` are `IndexMap<&'static str, _>`, `static_flags:
  Option<StaticFlags>` (`flags()` is `?? {}`), `hook_named(name)` maps TS's string hook names.
  `EMPTY_SCRIPT` is `empty_script()`. `Script.activate` and `StaticFlags.deftDuelist` not ported
  (SURFACE §7.2).

### state.rs functions
- `create_game(&CreateGameOptions) -> GameState` panics with TS's message on a bad handicap or deck
  (TS threw; SURFACE §6.1 fixes the return type); `validate_handicap` and `validate_deck(deck,
  catalog, label, size)` return `Result<(), EngineError>` with TS's text. `create_game` with
  `catalog: Some(..)` validates against it and registers nothing (the registry is a `OnceLock`;
  tests use the testkit override). `create_game_for_reset(options, next_id, resets)`.
- `new_instance(&mut impl NextId, def_id, owner, zone)`: TS's `Pick<GameState, "nextId">` is the
  trait `NextId`, implemented for `GameState` and `u32`.
- Added beside TS's functions: `find_instance_mut` (TS handed back the live object its callers
  wrote through) and `side_snapshot_instances_mut`. Shapes: `starting_hero_health(Option<&Handicap>)`,
  `row_of -> RowSlots`, `numbering_order<T: Clone>(&GameState, &[T])`, `clone_state` = `clone()`,
  `is_turn_of(&GameState, PlayerId)`, `find_instance(&GameState, &str) -> Option<&CardInstance>`.
- Calls other parts must match: `crate::catalog::registered_catalog() -> &'static CardDefs` (part 2),
  `crate::own_library::show_to_owner(&mut CardInstance)` (part 2),
  `crate::subsystems::last_boards::freeze_last_boards(Option<&LastBoardInput>, &CardDefs) ->
  Option<PerPlayerOpt<Vec<LastBoardEntry>>>` (part 8). `LastBoardInput` is a tuple `(Vec, Vec)`.

### config.rs
- Integer types: game quantities `i32`; bounds on Rust loops and collection lengths `usize`
  (`TIMEOUT_ANSWER_CAP`, `MAX_CHOICE_COMBINATIONS`, `NONCE_HISTORY`, `FUSED_ID_CAP`,
  `VIEW_EVENT_LIMIT`, `STATE_CHECK_PASS_CAP`, `MAX_WORK_STEPS`, `MAX_PROMPT_ANSWERS`,
  `MAX_MULLIGAN_SUBSETS`, `FUSED_MIN_PARTS`, `SCORER_DRY_RUN_PLAYS`, `AI_PLAYOUT_STEP_CAP`,
  `FUSE_MIN_INGREDIENTS`); `DAMAGE_REDIRECT_CAP: u32` (it bounds `EngineSink::converting`).
- The "decide" string switches (`ROTATION_RING`, `GENN_GREED_EXILES`, `FIENDER_STATS_MODE`,
  `MULLIGAN_ORDER`) are `&str`, not enums. `FATIGUE_DAMAGE: fn(i32) -> i32`. `RANDOM_KEYWORD_POOL`
  holds keyword keys (`&[&str]`); `TUNE_HARMFUL_KEYWORDS: &[KeywordKind]`; `POOL_TOKEN_TAGS`,
  `LAST_FACE_UP_SKIPPED_TAGS: &[Tag]`; `GLITCH_OUTCOMES: &[GlitchOutcome]`; `fib(i32) -> i32`.
- Moved in (research A §4, brief step 5), TS names kept: `VIEW_EVENT_LIMIT`,
  `RADIANT_SHEEP_TRIBUTE_VALUE`, `SHEEP_TRIBUTE_VALUE`, `LAST_BOARD_DISCOVER_OPTIONS`,
  `LAST_BOARD_CARD_COST`, `STATE_CHECK_PASS_CAP`, `MAX_WORK_STEPS`, `MAX_PROMPT_ANSWERS`,
  `MAX_MULLIGAN_SUBSETS`, `FUSED_MIN_PARTS`, `CHAOS_UNIT_COUNT`, `CHAOS_UNIT_COST`, `CHAOS_HEAL`,
  `CHAOS_MANA`, `CHAOS_ADDED_CARDS`, `CHAOS_RUSH_TOKENS`, `CHAOS_COST_DISCOUNT`, `CHAOS_BACKROW_CARDS`,
  `SCORER_WEIGHTS` (f64 fields), `SCORER_LOW_HEALTH`, `SCORER_DRY_RUN_PLAYS`, `LIFE_TAP_DAMAGE`,
  `STEADY_SHOT_RAISE`, `PING_DAMAGE`, `ARMOR_UP_ARMOR`, `TANK_UP_ARMOR`, `DIE_INSECT_DAMAGE`,
  `DIE_INSECT_LUCKY`, `BRAINSTORM_DISCOUNT`, `PLUCK_COST`, `STITCHING_INGREDIENTS`,
  `STITCHING_MAX_COST`, `AI_PLAYOUT_STEP_CAP`, `FIRST_GRADE`, `GRADE_D_CARDS`, `GRADE_D_DISCOUNT`,
  `GRADE_A_DAMAGE`, `FUSE_MIN_INGREDIENTS`, `CRAFTED_CARD_COST`. Not moved: `STEADY_SHOT_PARAM` (a
  string key), `LAST_GRADE` (= `GRADES.length`, derived in comboIndex), viewFor's `HIDDEN_*` (-1
  sentinels). Plus `TRAINING_GAMES`, `TRAINING_IMPROVE`, `TRAINING_UNBAN` (`TrainingGate { vs_random,
  vs_parent }`). All 96 `config.ts` exports are present with TS's values (checked by script).

### rng.rs
`create_rng(seed, cursor)` exists beside `Rng::new` (TS's name). `Rng` derives `Clone, Debug,
PartialEq, Eq`. `lucky`'s `roll` is handed `&mut Rng`. The pinned draws (`golden`,
`jackioh-fuzz-1`, plus `int`, `shuffle`, a UTF-16 seed and `lucky`) are TS's, printed by running
`rng.ts` under Node at this commit.

### wire
- `string_union!` (wire/mod.rs, `pub(crate)`): a TS literal union as a unit enum with `as_str`,
  `Display`, `FromStr`, `ALL` (TS order) and the `TS` derive. `GAME_EVENT_TYPES` is
  `GameEventType::ALL` (same 65, same order as TS); `KEYWORD_KINDS` is `KeywordKind::ALL`.
- `PerPlayer<T>`: `new`, `iter`, `map`, `Index`/`IndexMut<PlayerId>`. `PerPlayerOpt<T>`: `get`,
  `get_mut`, `slot` (`delete record[p]` is `*slot = None`), `is_empty`, `Default` without `T:
  Default`.
- `CardCost` (number | "X" | {base, embiggen}) and `ActivationUses` (number | "unlimited") serialise
  by hand; `Keyword` is tagged on `kind` (`Armor { n }`, `Lucky { n }` …). `BackrowView` and
  `PendingView` are boolean-discriminated (`faceDown`, `forYou`): two structs carrying the boolean,
  serialised untagged, deserialised by reading the boolean first. TS's `BackrowView` included
  `null`; Rust's does not, and an empty zone is `Option::<BackrowView>::None`.
- `Action { #[serde(flatten)] body: ActionBody, player_id, nonce }`; `ActionInput` is the same
  without the nonce (`with_nonce`); `ActionBody::action_type() -> ActionType`.
- `PARAM_PLACEHOLDER` is kept as the TS regex's text; `param_placeholders`/`fill_params` match it by
  hand (no regex crate in pure crates). Equal to TS on every catalog card (scratch test above).

### The client's generated TypeScript (SURFACE §5.1)
- `export_to = "../../../apps/web/src/wire/generated/"`, not SURFACE's `"../../…"`: ts-rs resolves
  `export_to` against `TS_RS_EXPORT_DIR`, default `./bindings` in the package directory, so `../../`
  would land in `crates/apps/`. The export creates an empty `crates/engine/bindings/` on the way.
- Every `skip_serializing_if = "Option::is_none"` field of a `TS` type has `ts(optional)`, so ts-rs
  writes TS's `x?: T`, not `x?: T | null`; every TS `x?: true` (and `autoEndTurn?: false`) has
  `ts(optional, type = "true")`; `TargetDecl.kind`, `ModeDecl.kind`, `DelayedAt.phase`,
  `costDiscount.onlyType`, `stolen.zone` and `crumbled.zone` carry TS's narrower literal unions.
- Checked: the 148 generated files against `packages/shared/src/{catalog-types,actions,events,view}.ts`,
  `state.ts` and `config.ts` with `tsc --strict` (mutual assignability, readonly stripped): all 87
  types that exist on both sides are equal except `BackrowView` (the `null` above). Not generated:
  type aliases (`CardDefs`, `Pile`, `LastBoardInput`), `DistributiveOmit`,
  `GameEventTypesAreExhaustive`, and the script types (not wire).

### Cards (`build.rs`, `src/lib.rs`, SURFACE §7.4)
- `build.rs` reads `src/scripts/<set>/<file>.rs` only (`<set>` ∈ core, classic, classic_plus); a
  `.rs` file anywhere else under `src/scripts/` or a file name that is not an identifier fails. The
  "catalog id with no file" check is a single `cargo:warning` (count and first five ids) while the
  files are fewer than the catalog's ids (318 today, so no number is written down), an error once
  they are as many. Every other problem fails the build listing all of them. `REGISTRY` is sorted
  by `ID`; modules are `scripts::<set>::<file>` with absolute `#[path]`s. The catalog version is the
  last `patches.json` entry's `version`, emitted as `JACKIOH_CATALOG_VERSION`. `build.rs` carries
  `#![allow(clippy::disallowed_methods)]` (clippy.toml bans `fs::read_to_string`/`env::var`, which a
  build script needs).
- `src/lib.rs`: `CATALOG: LazyLock<CardDefs>` and `CATALOG_IDS: LazyLock<Vec<String>>` (TS's
  constant names), `CATALOG_VERSION` const and `catalog_version()`, `catalog_json()`,
  `flavour_json()`, `card_def(id)` and `card_def_by_index(set, index)` (panic as TS threw),
  `scripts_of() -> IndexMap<String, CardScripts>`, `register_all()` guarded by a local
  `OnceLock<()>` and calling `register_catalog(CATALOG.clone(), CATALOG_VERSION)` and
  `register_scripts(scripts_of())` once per process. `pub mod query; pub mod ky_test_bank; pub use
  query::*;`. Not ported: `buildRegistry`, `CARDS`, `CardModule` (build.rs), flavour.ts (web).
  Unit tests: catalog.json round-trips through `CardDefs` unchanged; `catalog_version()` is the
  newest patch.

### Server and tools mains (SURFACE §11.1, §12)
- Server: `serve` (default), `release` (= `db::migrate::run(vec![])`, `cli::seed_catalog::run(vec![])`,
  serve), and one subcommand per TS `main()` handed its raw arguments: `migrate`, `seed-catalog`,
  `mint-code`, `seed-accounts`, `season-start`, `stats-cards`, `stats-import`. Each is `pub async
  fn run(args: Vec<String>) -> anyhow::Result<()>` in `db::migrate` or `cli::<x>` (each keeps TS's
  own argument parser). Serving reads the environment through `app::load_server_env(&IndexMap<String,
  String>) -> anyhow::Result<env::Env>` (TS's `loadServerEnv` lives in index.ts, so in `app.rs`)
  then `app::serve(env)`. An error prints on stderr and exits 1; clap's usage error exits 2.
- Tools: `catalog-version` is `catalog::VersionArgs` + `catalog::run_version` (a top-level
  subcommand, §12's row, inside part 22's `catalog.rs`); every other row `<module>::Args` +
  `<module>::run`. `Err` prints with `{:#}` and exits 1.

## DEPENDS-ON
The 16 names in `.fullsend/damage/part-01.txt`'s header, with the signatures above:
`catalog::{registered_catalog, register_catalog(CardDefs, &str)}`, `scripts::register_scripts(
IndexMap<String, CardScripts>)`, `own_library::show_to_owner`, `layers::UnitView`, `query::HeroView`
(part 2); `play_choices::PlayAction` (part 4); `testkit::scenario::{register_catalog,
register_scripts}` (part 5); `effects::mana::{gain_mana, refresh_mana}` (part 7);
`subsystems::{last_boards::{freeze_last_boards, last_board_for}, glitch::{seat_played_by,
seats_swapped, glitch}, board_history::roll_back, perfect_hand::replace_hand_with_perfect}` (part 8);
the server's `app::{serve, load_server_env}` (part 18), `db::migrate::run`, `cli::*::run` (part 20);
the tools' `<module>::{Args, run}` (parts 22, 23, 29).

## GAPS
- SURFACE §2: `reqwest`'s `rustls-tls` does not exist in 0.13 (used `rustls`); `indexmap` missing
  from the server and tools rows.
- SURFACE §5.1: `export_to` should be `"../../../apps/web/src/wire/generated/"` (or set
  `TS_RS_EXPORT_DIR`); ts-rs needs `ts(optional)` per field to write `x?: T` — SURFACE says nothing,
  and without it every optional field is `T | null` in TS.
- SURFACE §6.6: hooks take `&mut EffectContext`, not `&EffectContext` (card hooks draw from the rng).
- SURFACE §6.4/§6.5: names the types but not where the cross-module ones live; see "Types placed
  outside their TS module". A Wave 1 builder that redefines one of them (`CostRule` in
  `cost_rules.rs`, `EngineSink` in `resolve.rs`, `ReplacementDef` in `replacements.rs`,
  `QuestBook` in `subsystems/quests.rs`, `EventStay` in `stays.rs`, …) makes the root glob ambiguous:
  part 31 deletes the duplicate.
- SURFACE §6.1/§8: does not say that `layers::UnitView`/`query::HeroView` collide with the wire's,
  nor that `play_choices`/`play_steps` both define `PlayAction`; resolved in `lib.rs` as above.
- SURFACE §3: "the only statics are the catalog and the script registry": the cards crate also holds
  `CATALOG_IDS` (derived from the catalog), the `register_all` guard `OnceLock<()>`, and the
  generated `REGISTRY` (an immutable slice of fn pointers). None is mutable.
- SURFACE §7.4: "fails the build for a catalog id with no file" is a warning until the scripts are
  as many as the catalog's ids (brief step 9).
- SURFACE §10.4 (part 21): the hand-kept `apps/web/src/wire/index.ts` must add `type CardDefs =
  Record<string, CardDef>` (ts-rs exports no aliases) and widen `BackrowView` with `| null`, or the
  web's imports of those two names break.
- `cargo fmt --check` reports every empty placeholder file (rustfmt writes a newline into an empty
  file); it passes once the parts have filled them.
- Not done: git tags (the proxy refuses them); `.fullsend/waves.md` records the two commits instead.
