# Damage: engine src (`crates/engine/src/**`, `crates/engine/Cargo.toml`), part 31 Wave 2

Command: `cargo check -p jackioh-engine --lib --features testkit --message-format short`.

Both counts are rustc's type-check phase only: while any body fails to type-check, rustc runs no
borrow check, so the `&mut CardInstance` / `def_of`-borrow / two-`&mut`-of-one-sink errors Wave 3 will
meet (E0499, E0502, E0505, E0507) are not in either number.

- Before: `948e025` (staging when the session started), **187** errors.
- After: staging at this file's commit, **45** errors, every one E0308 and every one local (a caller
  passing `&card`/an owned card where the owner takes `&mut CardInstance`, a `Vec<CardDef>` where the
  owner wants `&[&CardDef]`, an argument by `&` where the owner takes it by value). No unresolved name,
  no wrong arity, no missing field, no missing trait impl is left in the lib.
- Lines in `crates/engine/src/**/*.rs`: before 69,294, after 67,929 (`git diff --shortstat
  85aeec0^ HEAD -- crates/engine/src`: 80 files, 513 insertions, 1,878 deletions).

### BEFORE: 187 errors

| file | errors |
|---|---|
| `crates/engine/src/effects/choose.rs` | 24 |
| `crates/engine/src/triggers.rs` | 14 |
| `crates/engine/src/effects/combat.rs` | 12 |
| `crates/engine/src/effects/transform.rs` | 10 |
| `crates/engine/src/combat.rs` | 10 |
| `crates/engine/src/reduce.rs` | 10 |
| `crates/engine/src/setup.rs` | 6 |
| `crates/engine/src/effects/cast.rs` | 5 |
| `crates/engine/src/effects/add_to_hand.rs` | 4 |
| `crates/engine/src/effects/swap.rs` | 4 |
| `crates/engine/src/replacements.rs` | 4 |
| `crates/engine/src/subsystems/scorer.rs` | 4 |
| `crates/engine/src/work.rs` | 4 |
| `crates/engine/src/effects/summon.rs` | 3 |
| `crates/engine/src/draw.rs` | 3 |
| `crates/engine/src/effects/shuffle_random.rs` | 3 |
| `crates/engine/src/play_steps.rs` | 3 |
| `crates/engine/src/subsystems/fuse.rs` | 3 |
| `crates/engine/src/subsystems/perfect_hand.rs` | 3 |
| `crates/engine/src/traps.rs` | 3 |
| `crates/engine/src/cry_trigger.rs` | 2 |
| `crates/engine/src/testkit/scenario.rs` | 2 |
| `crates/engine/src/effects/fruit.rs` | 2 |
| `crates/engine/src/effects/brittle.rs` | 2 |
| `crates/engine/src/effects/choose_where.rs` | 2 |
| `crates/engine/src/effects/delay.rs` | 2 |
| `crates/engine/src/effects/destroy.rs` | 2 |
| `crates/engine/src/effects/plague.rs` | 2 |
| `crates/engine/src/effects/shuffle_into.rs` | 2 |
| `crates/engine/src/prompts.rs` | 2 |
| `crates/engine/src/subsystems/call_to_chaos_plus.rs` | 2 |
| `crates/engine/src/subsystems/glitch.rs` | 2 |
| `crates/engine/src/subsystems/twice_forward.rs` | 2 |
| `crates/engine/src/targeting.rs` | 2 |
| `crates/engine/src/view_for.rs` | 2 |
| `crates/engine/src/echo.rs` | 1 |
| `crates/engine/src/effects/library_copies.rs` | 1 |
| `crates/engine/src/effects/steal.rs` | 1 |
| `crates/engine/src/effects/tune.rs` | 1 |
| `crates/engine/src/brittle.rs` | 1 |
| `crates/engine/src/effects/animate.rs` | 1 |
| `crates/engine/src/effects/card_scope.rs` | 1 |
| `crates/engine/src/effects/hand_exile.rs` | 1 |
| `crates/engine/src/effects/last_board.rs` | 1 |
| `crates/engine/src/effects/radiant.rs` | 1 |
| `crates/engine/src/effects/random_picks.rs` | 1 |
| `crates/engine/src/effects/rounds.rs` | 1 |
| `crates/engine/src/effects/split.rs` | 1 |
| `crates/engine/src/effects/targets.rs` | 1 |
| `crates/engine/src/instance_view.rs` | 1 |
| `crates/engine/src/ownership.rs` | 1 |
| `crates/engine/src/play_choices.rs` | 1 |
| `crates/engine/src/state_check.rs` | 1 |
| `crates/engine/src/subsystems/ai_policy.rs` | 1 |
| `crates/engine/src/subsystems/board_history.rs` | 1 |
| `crates/engine/src/subsystems/call_to_chaos.rs` | 1 |
| `crates/engine/src/subsystems/copied_text.rs` | 1 |
| `crates/engine/src/subsystems/lethal.rs` | 1 |
| `crates/engine/src/subsystems/rotation.rs` | 1 |
| `crates/engine/src/testkit/invariants.rs` | 1 |

| code | errors |
|---|---|
| E0308 | 100 |
| E0609 | 26 |
| E0061 | 21 |
| E0277 | 12 |
| E0432 | 9 |
| E0560 | 6 |
| E0599 | 5 |
| E0063 | 3 |
| E0422 | 2 |
| E0574 | 1 |
| E0271 | 1 |
| E0631 | 1 |

### AFTER: 45 errors

| file | errors |
|---|---|
| `crates/engine/src/effects/transform.rs` | 6 |
| `crates/engine/src/setup.rs` | 4 |
| `crates/engine/src/play_steps.rs` | 3 |
| `crates/engine/src/replacements.rs` | 3 |
| `crates/engine/src/subsystems/fuse.rs` | 3 |
| `crates/engine/src/effects/add_to_hand.rs` | 2 |
| `crates/engine/src/effects/brittle.rs` | 2 |
| `crates/engine/src/effects/shuffle_into.rs` | 2 |
| `crates/engine/src/effects/swap.rs` | 2 |
| `crates/engine/src/subsystems/glitch.rs` | 2 |
| `crates/engine/src/subsystems/perfect_hand.rs` | 2 |
| `crates/engine/src/cry_trigger.rs` | 1 |
| `crates/engine/src/effects/animate.rs` | 1 |
| `crates/engine/src/effects/fruit.rs` | 1 |
| `crates/engine/src/effects/hand_exile.rs` | 1 |
| `crates/engine/src/effects/random_picks.rs` | 1 |
| `crates/engine/src/effects/split.rs` | 1 |
| `crates/engine/src/ownership.rs` | 1 |
| `crates/engine/src/subsystems/board_history.rs` | 1 |
| `crates/engine/src/subsystems/call_to_chaos_plus.rs` | 1 |
| `crates/engine/src/subsystems/rotation.rs` | 1 |
| `crates/engine/src/subsystems/scorer.rs` | 1 |
| `crates/engine/src/subsystems/twice_forward.rs` | 1 |
| `crates/engine/src/traps.rs` | 1 |
| `crates/engine/src/testkit/scenario.rs` | 1 |

| code | errors |
|---|---|
| E0308 | 45 |


## Collisions (one concept written twice; all resolved, one winner each)

- **"Which of this action's events are dispatched"**: 3.1's `EngineSink.dispatched: Option<usize>`
  (combat.rs `withhold_from_frontier`) and 6.1's `SettleSink { sink, dispatched }` struct
  (effects/combat.rs) vs 3.3's `EngineSink.frontier: triggers::FrontierSlot` (triggers.rs, the owner of
  settle). Frontier kept; the other two gone.
- **TS `scripts.scriptOf(instance)` / `flagsOf` / `textsOf`**: `running_script` (layers, numbers,
  carriers, mana, times_played, query, play_counts, restrictions, subsystems/fuse), `script_of_card`
  (cost_rules, combat, draw, graveyard_play, play_choices, damage, cry_trigger, quests, activate),
  `face_script` (targeting, resolve, prompts, state_check, modifiers, triggers), `with_face`
  (replacements, traps, triggers), `flags_of` (modifiers, query, restrictions), `flags_of_card` (echo,
  combat, damage, play_steps), `texts_of` + `CardText` (damage) vs `scripts::{script_of, flags_of,
  texts_of, TextFlags}`.
- **Ingredient records (R102)**: `IngredientRecord`, `records_from`, `ingredients_of`, `ingredient_paid`,
  `as_ingredient`, `ingredient_record` in subsystems/fuse.rs and `IngredientEntry` in damage.rs vs
  scripts.rs's.
- **Fused-id parse (R179, R468)**: `fused_id_specs_in`, `fused_head_len`, `is_digest_id`, `copy_spec`
  (subsystems/fuse.rs), `fused_id_parts` + marks + `fused_head_len` (params.rs), `fused_head_len`
  (scripts.rs), `self_def_ids` (perfect_hand.rs, glitch.rs) vs catalog.rs's.
- **Part paths and remembered keys**: `part_path_of` (fuse.rs as `i64`, params.rs as `Value`),
  `reroot_remembered`, `memory_of_part`, `remembered_keys` (fuse.rs), `PART_KEY` (params.rs) vs work.rs's.
- **Composed-list runners**: `lazy_part`, `apply_effects` (fuse.rs, call_to_chaos_plus.rs) vs resolve.rs's;
  `active_target_decls`, `stored_declaration_slices` (fuse.rs) vs play_choices.rs's.
- **Zone readers**: `acts_on_field` (replacements, targeting, quests), `slot_in` (replacements, traps),
  `slot_of`/`row_size`/`is_reserved`/`reserve_zone`/`release_zone` (state_check), `card_at`/`row_size`
  (modifiers), `carried_at`/`is_carrier`/`backrow_top` (carriers), `card_at` + `PileZone` (testkit
  scenario, = `zones::OffFieldZone`) vs zones.rs's.
- **Stay marks**: `exit_mark` (resolve, prompts, traps, triggers), `event_mark`/`left_field_after`
  (traps) vs stays.rs's.
- **Work and settle**: `is_paused`/`owe`/`settle` (turn.rs), `is_paused` (traps.rs) vs work.rs's
  `paused`/`owe` and triggers.rs's `settle`.
- **Smaller single-owner copies**: `tuned_count`/`numbered_sum` (numbers.rs → tuning.rs),
  `add_enchantment` (shuffle_random → enchantments), `united_enchantments` (summon → enchantments),
  `count_play` (play_steps → times_played), `credited_killer_id` (damage → kill_credit),
  `mulligan_prompt_for` (testkit invariants → setup), `for_each_card` (hero_power → effects/each),
  the filler deck's catalog order (`is_token`, `index_rank`, `set_rank`, testkit scenario → catalog::query),
  seven `player_or_self` (→ `targets::player_of`), nine `owned` hedges (gone: the owners' return kinds
  are known), `CardScopeSide` + card_scope's `sides_of` (→ `targets::ScopeSide`/`sides_of`).
- **`card_mut`**: 17 private copies in `crates/cards/**` tests vs no testkit method → `Scenario::card_mut`
  added; the cards reconciler deletes the copies.
- **`register_catalog` in a card test**: the prelude's `catalog::*` glob and the testkit's override → the
  prelude lists catalog's names without it.
- **Not collisions (TS had both; kept, rule 3)**: `layers::UnitView`/`wire::UnitView`,
  `query::HeroView`/`wire::HeroView` (different shapes; part 1's `lib.rs` resolves the root);
  `PlayAction` (play_steps already re-exports play_choices'); the effect verbs `draw`, `add_to_hand`,
  `end_turn`, `gain_mana`, `refresh_mana`, `lose_health` beside the engine functions of those names;
  `state::validate_deck`/`validator::validate_deck`; `testkit::{register_catalog, register_scripts}`
  (SURFACE §8's override); TS's own twin helpers `instanceOf` (transform, steal, counters, radiant,
  targets), `sidesOf` (choose, targets), `keyOf`, `labelOf`, `nameOf`, `exileCard`, `standing`,
  `sourceOf`, `resumeFor`, `runOf`/`RUN_KEY`, `zoneFor`, `cardOf`, `contentsOf`/`canAccept`/
  `placeContents`/`bounceHome`, `plural`, `refuseTributes`, `isTrapCard`, `creationNumber`,
  `owedTrapsOf`, `attackTargetOf`, `compareText`, `indexRank`, `isToken`, `plagueOn` (graveyardPlay's
  own), `RUSH_TOKEN_INDEX`/`TOKEN_SET`/`tokenDefId`, `TRAP_TYPES`, `unreadableBy`, `readersOf`.

## Seams (fixed at the callers unless marked)

- Option structs and types the callers guessed: `MoveOptions`/`MovePosition` → `MoveToZoneOptions`/
  `LibraryPosition`; `PlaceOptions` → `PlaceOnFieldOptions`; `MoveToZonePosition::Index(usize)` →
  `LibraryPosition::At(i32)`; `RunHookOptions` → `HookResumableOptions`; `AddToHandResult` →
  `AddToHandOutcome`; `CardsInCardScopeOptions` → `Option<&CardScopeOptions>`; `WorkPlan { resume,
  owner }` → `WorkPlan::new(resume, owner)`; `GlitchOdds { system_plays }` → the state itself;
  `HealArgs { amount, to_full, up_to }` → `HealArgs::Amount { .. }`; `GraveyardRedirect` through JSON →
  its variants.
- Arities and argument kinds: `settle(sink)` (+`SettleOptions`), `switch_position` (+options),
  `play_out_turn` (+`PolicyOptions`, not `None`), `effective_cost` (+`CostOptions`, not `None`),
  `pierces(state, unit)` (→ `Some(unit), None`), `excluding_def_id`/`self_def_ids`/`fused_id_parts`/
  `def_of` (state first, `Option<&GameState>`), `owe(sink, vec![..])` (one item), `resume_self` (an
  `IndexMap`, not an `Option`), `make_context(sink.reborrow(), ..)` (→ `&mut` sink), `cast_card` (`&card`),
  `cards_in_scope`/`adjacent_to` (`&BoardScope`), `park_work` (`&PausedStep`), `apply_resumable`/
  `run_resumable_list` (effects and pause by value), `run_play_steps` (`&PlayAction`, not `&ActionBody`),
  `activate_ability` (`&ActivateAction`), scorer's `play_fields` (the `PlayAction` struct),
  `mark_dispatched(&[usize])`/`mark_dispatched(&events)` (→ `(sink, &[GameEvent])`), `catalog::query(T)`
  (→ `&T`), `zone_contents(..).cloned()` (already owned).
- Refusals returning `Option` (SURFACE §4.4.9): `restrictions::attack_restriction`,
  `zones::why_cannot_carry`, `setup::why_mulligan_refused` → `Result<(), EngineError>` at the owner.
- Missing serde (SURFACE §6.6, built with `json_as`): `ResumeAtArgs`, `CastNewArgs`/`CastNewDef`,
  `CastRandomArgs`/`CastRandomQuery`/`CastRandomCount`, `CastAfterward`, `CastOptions`, `OpenPromptArgs`,
  `HookResumableOptions`, `PlaceOnFieldOptions` → derived at the owner (closure variants
  `#[serde(skip)]`).
- **Left for Wave 3 (local, all E0308)**: the movers' `&mut CardInstance` (zones' `move_to_zone`,
  `place_on_field`, `remove_from_any_zone`, `cease_to_exist`, `replace_in_zone`; draw's `add_to_hand`,
  `shuffle_into_library`; brittle_count's `give/gain/start_*`) at ~25 call sites; `pick_generated`'s
  `&[&CardDef]` (transform ×2, call_to_chaos_plus) and `query`'s `Vec<&'static CardDef>` (scorer);
  `animate_card`'s `AnimateOptions`; `deal_damage` by value (split); `bounce_card(&card)` (swap);
  `add_enchantment(&Enchantment)` (transform); `score_def`'s arguments (perfect_hand); `run_resume`'s
  `&Resume` (cry_trigger).
- Engine-side seams the engine-tests reconciler listed for the test side (`.fullsend/damage/engine-tests.md`):
  the owners' shapes stand (`make_context(&mut sink, ..)`, the movers' `&mut CardInstance`, the trailing
  `Option`s and option structs, `AuditArgs`, `answer_targeting`'s positional arguments,
  `credited_killer_id(&source, &victim)`); the missing serde derives they named are added (above).

## Drift

- None in `crates/engine/Cargo.toml`: no dependency added or wanted (serde, serde_json, indexmap, ts-rs).
- `crates/engine/src/wire/codes.rs` carries ~180 KB of hand-written NFKC tables (part 5.3, R191);
  left for part 37 to cull or replace.
- Every `run_owed_*` handler `work.rs` dispatches to, `replacements::graveyard_redirect_for`,
  `targeting_point::targetable_options`, `play_steps::cast_through_pipeline`, `effects::plague::
  answer_placement`, `effects::fuse::answer_fuse_onto`, `cry_trigger::answer_cry_prompt` were already
  `pub`; `testkit::scenario::{catalog_override, scripts_override}`, `registered_catalog()`/
  `registered_scripts()` returning the override, `Scenario::state_mut`, `view_for_with_clock` and
  `FoldArgs`/`FoldResult` (serde camelCase) were already there.

## Semantic conflicts

- `.assumptions` of parts 1–8: `error.style`, `hook.style`, `null.optional`, `registry.style`,
  `rng.source`, `anon.args` differ only in annotations on one value; no key has two values.
- `EngineSink::reborrow`: `converting` and `dry_running` are copied down (every `converting += 1` is
  undone before its call returns, so the copy is exact inside one sink chain; a nested `reduce` in
  ai_policy starts at 0, where TS's module `let` carried over); `frontier` is shared; `owed_behind`
  (R117) is cloned, as part 3.1 decided (TS's `makeContext` built a fresh object without it).
- The shared frontier: a sink a test has already run a combat on now withholds its next declaration
  too (part 3.3's note); production paths are unchanged.
- `work::part_path_of` drops path entries that are not whole non-negative numbers (TS kept any
  number and `params.param` threw on a bad one); a malformed path no longer panics in `param`.
- A Fuse whose kept card is no ingredient: TS rerooted its memory at index -1 (`key@-1`); Rust skips
  the reroot (`work::reroot_remembered` takes `usize`).
- Scorer dry run (part 8.1): ai_policy's nested `reduce` draws from the match stream, not the dry
  run's side stream; no `reduce_with(sink)` added (no golden trace shows it yet).
