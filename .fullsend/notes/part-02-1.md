# Slice: part 2 (engine 1: the model and the board), chunk 1 of 2 (#390)
BUILDS-RUN: 0

## FILES
All new, each the whole TS file (every function in TS order, its doc comments and every comment that
states a rule or cites a ruling): `crates/engine/src/{announce, book_swap, brittle_count,
cast_on_draw_now, draw_complete, faces, marks, params, plague, scripts, stays, tuning, zones}.rs`.
Nothing left.

## SURFACE
Matched §4–§6 with part 1's frozen types (`EventStay`, `AnnounceRecord`, `MarkRecord`, `FieldExits`,
`UncoveredNote`, `HomeZone`, `EngineSink`, the hook argument bundles). `scripts::script_of(state,
def_id) -> CardScripts` and `register_scripts(IndexMap<String, CardScripts>)` as §6.6 names them.

## DEPENDS-ON / GAPS

### Not ported (by SURFACE §6.6)
- `zones.ts`: `registerGraveyardRedirect`, `GraveyardRedirectCheck` and the module-level
  `let graveyardRedirect`: `move_to_zone` calls `crate::replacements::graveyard_redirect_for` directly.
- `scripts.ts`: TS replaced the registry on every `registerScripts`; the `OnceLock` takes the first
  call and ignores the rest (tests use the testkit override).

### Names called in other parts' modules (signature assumed)
- Part 2 chunk 2: `catalog::def_of(Option<&GameState>, &str) -> &'a CardDef` — a reference tied to the
  state's lifetime (`faces::running_face` returns `&'a CardFace` borrowed from it). If it returns an
  owned `CardDef`, `running_face`/`running_face_of` must return owned faces. `own_library::show_to_owner(&mut CardInstance)` (part 1's).
- Part 3: `replacements::graveyard_redirect_for(&GameState, &CardInstance) -> Option<zones::GraveyardRedirect>`
  — TS's module-private `graveyardRedirectFor`, so it must be at least `pub(crate)`; its two answers are
  `zones::GraveyardRedirect::{Exile, LibraryBottom}` (defined here, TS's `{to:"exile"} | {to:"library",
  position:"bottom"}`). The card it is handed still names the zone it left (TS read `card.zone.z`).
- Part 5: `testkit::scenario::scripts_override() -> Option<&'static IndexMap<String, CardScripts>>`
  (SURFACE §8's thread-local; `'static` like chunk 2's `catalog_override`, e.g. a leaked box per
  registration). Called only under `#[cfg(feature = "testkit")]`; while it is `Some`, it is the whole
  registry (TS's wholesale replace), fused composition still applying to ids it lacks.
- Part 7: `effects::transform::swap_book(<args deserialisable from { instanceId, from }>) -> Effect`
  (built with `json_as`, so the arg type's name does not matter); `effects::transform::book_swap_source_of(&CardInstance)`
  returning `Option<String>` or `Option<&str>` (only `.is_none()` and `.map(|s| s.to_string())` are used);
  `effects::tune::TuneDirection::{Degrade, Upgrade}` (`Copy + PartialEq`; `params::steppable_params` takes one).
- Part 8: `subsystems::fuse::compose_fused_scripts(&GameState, &CardDef) -> CardScripts` (SURFACE §6.6),
  called by `scripts::scripts_for` only for an id the registry lacks that names `FUSE_MIN_INGREDIENTS`
  or more ingredients (a readable `t-<n>:<a>+<b>` id parsed as `catalog.fusedIdSpecs` did, or a digest
  id's `ingredients` off its definition) and whose definition is in `state.transient_defs`. A readable
  fused id with no transient definition gets the empty scripts (TS rebuilt it from the id alone) — part
  8's composition of a nested fused ingredient must find that ingredient's definition in the state.

### Signatures other parts call that differ from TS's (part 31: fix call sites, not these)
- `scripts::script_of(state, key)`: `key` is a `&str`/`&String` (→ `CardScripts`, SURFACE §6.6, TS
  `scriptsFor`) or a `&CardInstance`/`&mut CardInstance` (→ the running face's `Script` with the Vanilla
  guard, TS `scriptOf`), through the `ScriptKey` trait. `scripts_for(state, def_id)`,
  `flags_of(state, &card)`, `texts_of(state, &card)`, `zones::is_carrier(state, &card)` and
  `plague::plague_multiplier_of(state, &card)` take the state (fused scripts come from it).
  `registered_scripts()` returns an owned copy.
- `params::param(&impl ParamContext, key)`: implemented for `EffectContext`, `HookArgs`,
  `ConditionContext`, `TargetCheckArgs`, `ReplacementContext`, `WouldCounterArgs`, `CostArgs`. A caller
  holding a hook's by-value args passes `&args`; `&mut EffectContext` coerces. `param_value(state,
  Option<&CardInstance>, key, ParamValueOptions { def_id, radiant })` (`Default`).
  `steppable_params -> Vec<SteppableParam { param, steps, delta }>`; `params_of -> Vec<Param>`,
  `param_decl_of -> Option<Param>` (owned).
- `tuning::tuned_count(&card, key, printed)` (3 args, floor `TUNED_FLOOR`) and
  `tuned_count_with_min(&card, key, printed, min)` for TS's optional `min` (no TS caller passes it).
  `add_step(Option<&IndexMap<String, i32>>, key, delta) -> IndexMap`, `sum_tunings(&[Option<Tuning>])`,
  `copy_tuning(Option<&Tuning>)`, `tuning_of(&mut CardInstance) -> &mut Tuning`.
- `plague::permanents_on_field(state, first)` and `plague_on_field(state, player)` take
  `impl Into<Option<PlayerId>>` (a `PlayerId` or `None` for TS's omitted argument).
  `place_plague_on`/`remove_plague(&mut EngineSink, &CardInstance, i32) -> i32` write the counter on the
  card under that id in the state and read its count there.
- `faces::{running_face, card_type_of}(state, &CardInstance)`; TS's `{ defId, radiant }` literal
  callers (summon.ts, restrictions.ts) use `running_face_of(state, def_id, radiant)` /
  `card_type_of_face(state, def_id, radiant)`.
- `brittle_count::{start_brittle_on_field, give_brittle_count, gain_brittle_count}(&GameState, &mut
  CardInstance, …)`: a card in the state is cloned out, changed and written back by its caller.
- `book_swap::book_swap_trigger() -> TriggerDef` in place of the `BOOK_SWAP_TRIGGER` constant (closures
  cannot be a `const`; no new static, SURFACE §3). `granted_hand_triggers(&card, &[TriggerDef]) -> Vec<TriggerDef>`.
- `marks::{mark_delayed(sink, &DelayedEffect, &CardMark), sync_marks(sink, delayed_id, &CardMark,
  &[String]), sweep_marks(sink)}` take `&mut EngineSink` (TS `MarkSink = { state, events }`).
- `stays::left_field_since(&[GameEvent], from: usize, id)`; `moves_in(impl IntoIterator<Item =
  &GameEvent>, Option<&GameState>) -> LaterMoves { moved: IndexSet, controller_before: IndexMap }`;
  `note_uncovered(state, removed_id, resumed: Option<&str>)`; `stays::EventStay` re-exports `state::EventStay`.
- `announce::end_announce -> Option<AnnounceRecord>` (owned); `open_announces -> &[AnnounceRecord]`.
- `zones`:
  - `ZoneSlot = wire::ZoneRef`; every slot argument is `impl Into<ZoneSlot>` (a slot or `&slot`; zones.rs
    implements `From<&ZoneRef> for ZoneRef`).
  - Readers return references: `card_at`, `pile_at`, `carried_at -> Option<&_>`, `beneath_at -> &[CardInstance]`,
    `active_units_of`, `dormant_units_of`, `carried_units_of`, `dormant_backrow_of -> Vec<&CardInstance>`,
    `home_of -> Option<&HomeZone>`; `card_at_mut`, `pile_at_mut` added. `zone_contents -> Vec<CardInstance>`
    (copies, for a mover to set down). `why_cannot_carry -> Option<&'static str>`; `stacked_onto -> Option<&str>`.
  - Options structs (all `Default`, fields `Option<bool>` by SURFACE §4.3): `PlaceOnFieldOptions { stack }`,
    `RemoveFromFieldOptions { with_pile }`, `AcceptsStackCardOptions { move_ }`, `MoveToZoneOptions {
    position: Option<LibraryPosition>, keep_state }` with `LibraryPosition::{Top, Bottom, At(i32)}`.
    `step_into_backrow(state, &card, slot, stack: bool)` takes a bool (chunk 2's call).
  - `OffFieldZone::{Hand, Library, Graveyard, Exile}` (`zone_name()`, `zone_for(player)`, `ALL`);
    `MoveResult::{Moved, Vanished, Replaced}`; `report_graveyard_landing(&mut EngineSink, &CardInstance, MoveResult)`.
  - Ownership (zones.rs header): `move_to_zone`, `remove_from_any_zone`, `cease_to_exist` take `&mut
    CardInstance`, refresh it from the card under its id in the state first, and leave it as it landed
    (zone included, for `report_graveyard_landing`); `place_on_field`, `replace_in_zone`'s replacement,
    `fresh_face_down_id`, `reset_instance` take `&mut CardInstance` as handed and the state stores a copy;
    `step_into_unit_zone`, `step_into_backrow`, `flicker_in_place` take `&CardInstance` and work on the
    card under that id (`flicker_in_place` no longer hands the reset card back: re-read it by id).

## Decisions
- Rust ownership of TS's live objects as above; `remove_from_field`, `is_buried`, `acts_on_field`,
  `slot_of` and the other readers take `&CardInstance` and use its id and zone as handed.
- `OffFieldZone` and `MoveResult` are written by hand (serde camelCase, `as_str`), not with
  `wire::string_union!`, which would export them to `apps/web/src/wire/generated/` (V20 diff).
- `scripts`: one `OnceLock<IndexMap<String, CardScripts>>`; fused composition on lookup (above). The fused
  id parse (`t-<n>:` head, top-level `+`, `(…)` groups, a trailing `*`) is a private copy in `scripts.rs`
  and in `params.rs` (rule 5), reading a digest id's list off `state.transient_defs[id].ingredients`.
- `params`: `part_path_of` is a private copy of `work.partPathOf` over the literal `"__part"` (work.rs's
  `PART_KEY`); a path entry that is not a usable index panics with TS's message. `param_step`'s
  `Math.round` is `(x + 0.5).floor()` (SURFACE §4.4.4); `param_max` `None` is `+∞`. A missing declaration
  panics with TS's text (TS threw).
- `IngredientRecord` derives serde (camelCase, `parts` skipped when absent) so a Fuse can write it to
  memory; `as_ingredient` sets `embiggened: Some(record.embiggened)` (TS wrote the boolean, `false` too).
- `stays::cards_named_by` reads the event's JSON (`serde_json::to_value`) for TS's scan of named fields;
  every other event reader matches `GameEvent` variants. `note_moved`/`note_reported` leave an emptied
  `uncovered` as `Some({})`, as TS left `{}` (hash fidelity); `note_field_exit` inserts into `last`
  keeping an existing key's place (JS object order).
- `tuning`: TS's `delete record[key]` is `shift_remove`; `tidy` drops `Some(empty)` maps/vecs and zeros,
  and the record once every field is `None` (TS `Object.keys(tuning).length === 0`).
- `zones`: an out-of-range lane is guarded (lane < 1 → no zone; a row grows to fit a lane past its end,
  as a JS array assignment would); `lock_zone`/`unlock_zone` grow the flags row likewise.
  `place_on_field` decides where the card goes before changing anything, then updates the card and
  inserts the copy — the same end state as TS's insert-then-mutate of one object. `ring_neighbor` and
  `pile_at` panic with TS's messages.
- `marks::sync_marks`/`sweep_marks` compare records by value (TS by identity; equal records are only
  ever removed together, so the result is the same).
- No unit tests (parts 24–27 port them).
