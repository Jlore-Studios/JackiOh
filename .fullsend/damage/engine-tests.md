# Damage: engine tests (`crates/engine/tests/**`), part 31 Wave 2

Command: `cargo check -p jackioh-engine --tests --features testkit --message-format short --keep-going`.

## How the test side was measured

On `staging` the engine **lib** does not compile (187 errors at `948e025`, 45 after the engine-src
reconciler's pushes), so cargo never reaches the `rules` test binary: the real build shows **0** test
errors at both ends. To see the test side, every count below marked *scratch* comes from a copy of the
workspace in the session scratchpad (never committed) where the body of each lib fn holding an error
was replaced by `unimplemented!()` (41–136 bodies, 7 stub types; no signature changed) and
`jackioh-cards` was a two-name stub (`CATALOG`, `register_all`, the only names `tests/golden.rs`
imports). Errors hidden behind a typeck error in the same body (borrowck: E0507, E0502, E0499) are not
counted by rustc until that body typechecks, so Wave 3 will see more of those.

## Before

Real build, `948e025`: **187** errors, all in `crates/engine/src` (lib); test binary not reached.
By code: E0308 100, E0609 26, E0061 21, E0277 12, E0432 9, E0560 6, E0599 5, E0063 3, E0422 2,
E0574 1, E0271 1, E0631 1. Top files: effects/choose.rs 24, triggers.rs 14, effects/combat.rs 12,
effects/transform.rs 10, combat.rs 10, reduce.rs 10, setup.rs 6.

Scratch, first contact: one **syntax error** (`fixtures/copied_text.rs:253`, a missing `)`) stopped
the whole `rules` binary at the parser. With it fixed: **2,775** test-side errors.
By code: E0618 1,344 (a fixture static called as a fn), E0061 879 (794 of them `put` with 3 of its 4
arguments), E0308 371, E0277 95, E0283 41, E0609 11, E0432 10, E0433 8, E0530 4, E0106 2, E0425 2,
E0422 2, E0631 2, E0560 2, E0063 1, E0502 1.
Top files: activate 148, replacements 142, fuse_variants 113, backrow_piles 97, effects_datacenter 81,
effects_tune 81, layers 78, effects_boardwide 75, combat_validation 70, effects_core 62,
call_to_chaos_plus 55, board_history 54, effects_choose 54, animated 50, play_choices_filters 47,
brittle 45, effects_transform 45, core_patches 44, effects_targets 43, effects_combat 42.
`tests/golden.rs` and `tests/export_config.rs` (separate binaries): 0.

## After

Real build, staging after the rebase (`part-31-eng-tests` pushed): **45** errors, all E0308 in
`crates/engine/src` (the engine-src reconciler's; not this scope). Test binary still not reached.

Scratch, same method, over that staging: **384** test-side errors, every one an engine seam (below)
or a local mechanical fix; **0** fixture-shape errors left (no E0618, no `put`/`new_game`/`in_hand`/
`flush`/`vanilla_deck`/`token_def`/`cast_now`/`recorder`/`replayable` arity or type error, no
unresolved fixture name).
By code: E0308 229, E0061 71, E0277 67, E0631 7, E0432 2, E0106 2, E0422 2, E0560 2, E0063 1, E0502 1.
By file (`rules/`): call_to_chaos 34, backrow_piles 27, board_history 18, effects_cost 15, query 15,
call_to_chaos_plus 13, effects_cast 13, effects_choose 13, activate 12, effects_animate 12, papaya 12,
pools 9, params 8, ai_policy 8, replacements 6, animated 6, damage_pipeline 6, effects_core 6,
triggers 6, turn 6, brittle 5, combo_index 5, effects_counters 5, effects_cry 5, effects_datacenter 5,
effects_delay 5, tribute 5, condition_active 4, control_change 4, effects_boardwide 4,
effects_card_scope 4, rotation 4, turn_wiring 3, validator_drafts 3, fixtures/field 3,
control_change_property 3, effects_buff 3, effects_cast_chaos 3, effects_combat 3, effects_enchant 3,
effects_flicker 3, faces 3, kill_credit 3, preview 3, prompts 3, rulings_a 3, audit 2,
fixtures/play_pipeline_b 2, carried_damage 2, combat_validation 2, effects_choose_where 2,
effects_destroy 2, effects_draw_while 2, effects_each 2, recruit_variants 2, restrictions 2,
shuffle_random 2, and 1 each in fixtures/{activate, damage_combat, kill_credit}, after_attack,
backrow_death, draw_complete, effects_after_check, effects_brittle, effects_damage, effects_library,
effects_plague_random_cast, library_copies, own_library, prompt_kinds, replay_scripted, rulings_b,
setup, tribute_zones, trigger_zones, view_marks.

Lines in `crates/engine/tests/**/*.rs`: before 99,006, after 98,968
(`git diff --shortstat 948e025 HEAD -- crates/engine/tests`: 2,054 insertions, 2,092 deletions).

## Collisions

- **Fixture CardDef shape**: `fn x() -> CardDef` guessed by parts 24.1–24.5, 25.x, 26.1, 26.7 against
  the fixtures' `pub static x: LazyLock<CardDef>` (combat, field, instance_data, generation,
  damage_combat, datacenter, activate, call_to_chaos_plus, core_patches, fruit, kill_credit,
  board_history) and, the other way, `x.id` against the fixtures' `pub fn x() -> CardDef`
  (play_pipeline_b, prompts, quests, scripts, turn, twice_forward, ky_test, papaya). Owner wins per
  fixture file: 1,344 static call sites and 10 fn field reads rewritten.
- **Fixture script tables named three ways**: the TS-named static (`TURN_SCRIPTS`), the brief's
  `scripts()`, and a hedge alias (`turn_scripts()`, `twice_forward_scripts()`, `pb_scripts()`,
  `prompt_scripts()`, `fixture_scripts()`, plus `turn_defs()`, `pb_defs()`, `prompt_defs()`,
  `fixture_defs()`). Hedges deleted (9 fns); callers read the TS-named statics. Callers that guessed
  names no fixture has (`activate_scripts()`, `combat_scripts()`, `fruit_scripts()`, `gen_scripts()`,
  `core_patch_scripts()`, `lab_pool()`, `test_mark()`, `ct()`, `rng_child::run`, `harness::PutOptions`)
  now name the owners' (`ACTIVATE_SCRIPTS`, `COMBAT_SCRIPTS`, `FRUIT_SCRIPTS`, `GEN_SCRIPTS`,
  `CORE_PATCH_SCRIPTS`, `LAB_POOL`, `TEST_MARK`, `CT`, `rng_child::rng_child`, a `json!` options bag).
- **`LOG_LANE`**: `i32` in fixtures/activate.rs and damage_combat.rs, `usize` in fixtures/turn.rs;
  every caller passes it to `harness::slot(.., lane: i32)`. turn.rs's is now `i32` (part 1's frozen
  `ZoneRef.lane: i32`).
- Not collisions (left as they are): per-file local sink holders (`Sink`, `Bench`, `with_sink`) beside
  `harness::sink_for`; per-file `act`/`ids`/`json_of` helpers; `turn.rs`'s own inline `turn_scripts`
  (turn.test.ts's own fixtures, not fixtures/turn.ts). No two definitions share one module.

## Seams (engine-side; classified, not fixed: the engine-src reconciler's or Wave 3's)

Counts from the final scratch build; the test side calls the TS shape, the engine owner differs.
- `resolve::make_context` takes `&mut EngineSink`, 52 callers pass `sink_for(..)` by value; 17 more
  pass other wrong arguments.
- `zones::move_to_zone` / `place_on_field` / `remove_from_any_zone`: `&mut CardInstance` vs the
  callers' owned/`&` copies (55), and `PlaceOnFieldOptions` is not `Deserialize` (21: callers build
  `{ stack }` with `json_as`).
- Option-wrapped trailing parameters the callers pass bare: `roll_chaos_effects` (19, `Option<&[..]>`),
  `effects::targets_in_scope` (10, `Option<&TargetScope>`), `catalog::def_of` (10,
  `Option<&GameState>`), `view_for_with_clock` (1), `play_choices::legal_zones_for` (7: callers pass
  `None` where the owner takes `&[String]`).
- Arity: `mana::effective_cost` (19), `triggers::settle` (14), `subsystems::choose_action` (9: a 4th
  `PolicyOptions`), `modifiers::add_modifier` (7), `catalog::excluding_def_id` (5),
  `tribute_cost_of` (4), `work::can_resume` (3), `fused_id_parts`/`self_def_ids` (4, now take state),
  `projected_hero_damage` (2), `answer_targeting` (2) with `AnswerTargetingArgs` missing,
  `modifiers::schedule_delayed` (2).
- Not `Deserialize` though SURFACE §6.6 says effect argument types are data: `CastRandomArgs` (11,
  and no `how` field), `CastNewArgs` (8), `OpenPromptArgs` (6), `RotationArgs` (5, taken by `&`),
  `CastOptions` (2), `HookResumableOptions`, `ResumeAtArgs`, `CardScopeOptions`, `CatalogQueryArgs`.
- Types: `CardOrId<'_>` lacks `From<Option<..>>` (`query::played_earlier`, 10); `papaya::zone_of_point`/
  `point_of_zone` take `&` (12); `ChaosEffectName` enum vs callers' strings (10);
  `HeroPowerName` vs `&str` (1); `activate_ability` takes `&ActivateAction`, `first_target`
  `&ActionBody` (6); `effects::field_spells_doomed` takes `SweepReader` by value (2);
  `kill_credit::credited_killer_id` takes `&CardInstance` (3); `enchantments::has_enchantment` takes
  `EnchantmentKind` (2); `query::zone_cards` takes `OffFieldZone` (2); `replacement_of` takes the
  context (1); closures typed `Fn(&EffectContext)` where the engine wants `&mut` (7, E0631, incl.
  `ForEachCardArgs.cards`, `DrawWhileArgs` has a `player` field the caller's literal lacks), `DeckDraftInput.is_portrait`
  wants `&dyn Fn`, not `Arc` (2).
- Missing names: `subsystems::audit::AuditTargetsArgs`; testkit `mock_brittle_tick`,
  `mock_animate_at_turn_start`, `mock_return_at_cleanup` (turn_wiring.rs, 26.6); a way to run work
  owed under a test-made hook (`registerWorkHandler`, pauses.rs, 25.3): both in `spec-gaps.md`.
- Local mechanical, left for Wave 3: two E0106 lifetimes on test helpers returning
  `EffectContext<'_>`/`DeckDraftInput` (params.rs:71, validator_drafts.rs:66); one E0502 borrow
  (activate.rs:207); the masked borrowck errors noted above.

## Drift

- `crates/engine/tests/export_config.rs` (part 21) is a third test binary beside `rules` and `golden`
  (SURFACE §1 says one binary per crate plus `golden.rs`); it is auto-discovered and compiles. Left.
- `tests/rules/targeting.rs` and `tests/rules/transform_variants.rs` are empty: part 26 never ported
  `targeting.test.ts` (465 lines) or `transform-variants.test.ts` (194). Declared in `rules/mod.rs`, so
  the tree is whole; the tests are missing (for the orchestrator).
- The module tree is complete: every `rules/*.rs` and `rules/fixtures/*.rs` is declared, nothing
  declared is missing; `tests/golden.rs` stands alone (it imports only the engine and `jackioh_cards`).
- The testkit exports no `Arc`; four test files now `use std::sync::Arc;` themselves.

## Semantic conflicts (parts 24–27 `.assumptions`)

- `test.fixture_defs`: fns (26.1, 26.7) vs statics (26.6) vs "as staging's files define them" (26.4)
  → each fixture file's own definition (rule 2).
- `test.default_params`: trailing `Option<T>` (26.1, 26.7, 27.6) vs TS's default passed explicitly
  (26.6, 27.1) → for fixture functions, the owner's: TS defaults passed as plain values (`in_hand(.., 1)`,
  `flush(.., 10)`, `vanilla_deck(20, 1)`, `cast_now(.., P1, false)`, `token_def(name, [Tag::Token])`);
  for engine functions, the engine owner's (seams above).
- `test.literals`: `json_as::<ActionInput>(json!)` vs `json!` → the fixture action helpers take
  `impl Serialize`, so their callers pass `json!` (81 `json_as` wrappers removed); test-local helpers
  that take `ActionInput` keep `json_as`.
- `test.sink_for`: `harness::sink_for` vs local `with_sink`/`Sink` → both stay (no shared module).
- `rng.source` (25.7, 27.2, 27.4: the test sink writes no cursor back) → TS's `sinkFor` did not; the
  harness's `sink_for` does not. Consistent.
- `error.style`, `hook.style`, `registry.style`: same base value with annotations; no conflict.
