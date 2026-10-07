# Slice: part 14 (cards lane 6), chunk 3 of 4 (#402, parent #306): Classic #76–#88
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All thirteen were absent on `staging` (no night-bot half-port, no `part-14-bot.md` notes); each is the
full port of its TS script and its TS test file: the script's header as `//!`, every TS function in TS
order with its comments, `pub const ID`, `pub fn script() -> CardScripts`, then the test file's header
above `#[cfg(test)] mod tests`, one `#[test]` per TS `it` (counts checked against the TS: 13, 17, 25, 16,
10, 15, 16, 10, 13, 8, 3, 25, 18) and one `mod` per inner `describe`. No `todo!`, `unimplemented!` or
`// TODO`.
- `crates/cards/src/scripts/classic/c076_plague_bringer.rs` ← `076-plague-bringer.ts` + test
- `crates/cards/src/scripts/classic/c077_anti_magic_monkey.rs` ← `077-anti-magic-monkey.ts` + test
- `crates/cards/src/scripts/classic/c078_mutate_spell.rs` ← `078-mutate-spell.ts` + test
- `crates/cards/src/scripts/classic/c079_risky_die.rs` ← `079-risky-die.ts` + test
- `crates/cards/src/scripts/classic/c080_boom_big_max.rs` ← `080-boom-big-max.ts` + test
- `crates/cards/src/scripts/classic/c081_the_power_to_thrive.rs` ← `081-the-power-to-thrive.ts` + test
- `crates/cards/src/scripts/classic/c082_sheeople.rs` ← `082-sheeople.ts` + test
- `crates/cards/src/scripts/classic/c083_flame_lance.rs` ← `083-flame-lance.ts` + test
- `crates/cards/src/scripts/classic/c084_lockdown.rs` ← `084-lockdown.ts` + test
- `crates/cards/src/scripts/classic/c085_king_wagtoggle.rs` ← `085-king-wagtoggle.ts` + test
- `crates/cards/src/scripts/classic/c086_genn.rs` ← `086-genn.ts` + test
- `crates/cards/src/scripts/classic/c087_plague_chalice.rs` ← `087-plague-chalice.ts` + test
- `crates/cards/src/scripts/classic/c088_siphon_squad.rs` ← `088-siphon-squad.ts` + test

## SURFACE
SURFACE §7 (one file per card, `use jackioh_engine::prelude::*;`, effect verbs imported by name from
`jackioh_engine::effects`, effect arguments built from TS's object literals with `json_as(json!(…))`) and
§8 (the testkit verbs as part 5.1's notes fix them: steps return `&mut Scenario`, `unit`/`backrow`/
`hand`/`pile` return owned copies, `card()` returns `&CardInstance`, card refs `&str`/`&String`/
`&CardInstance`, seats `PlayerId`, `expect_refused(_with)` closures `|s| s.step(…)`). Part 1's frozen
code wins over SURFACE where they differ: hooks take `&mut EffectContext`, `TriggerDef::new(id, &[..],
run).with_when(..)` with `(ctx, &GameEvent)`, `read_hook`/`aura_hook`/`condition_hook`/
`would_counter_hook` builders, `crate::card_def(id) -> CardDef` (owned).

## DEPENDS-ON
- Effects (parts 6, 7), each built with `json_as` from TS's literal so only the serde shape matters:
  `place_plague_tokens({count})`, `draw({count})`, `consume_plague({target})`, `exile({target})`,
  `forced_attack_random({attacker, times})`, `fuse_onto_your_card({target})`, `remember({key, value})`,
  `set_cost_mod({target, amount})`, `heal({target: {of: "selfHero"}, amount})`, `gain_mana({amount})`,
  `damage({to, amount, trample})`, `lock_played_zone({event})` (a `GameEvent` read back from JSON),
  `recruit({count})`, `counter_play({target})`, `place_plague({target: {of: "self"}, amount})`;
  `swap_library()` (no argument); `reveal(Default::default())`.
- `effects::targets::{TargetSpec::Chosen { index: None } (Serialize: it goes into `json!` too),
  instance_of(&EffectContext, &TargetSpec) -> Option<CardInstance>}`.
- Engine reads (TS name snake_cased at its TS module): `params::param(&impl ParamContext, &str) -> i32`
  for `EffectContext` (`param(&*ctx, …)`) and `HookArgs` (`param(&args, …)`) as part 2.1 wrote it;
  `query::{recalled(&EffectContext, &str) -> Option<Value> (part 2.2's notes), zone_cards(&GameState,
  PlayerId, OffFieldZone)}` (owned or borrowed elements both compile: `CardInstance::clone(card)`);
  `zones::{OffFieldZone::{Hand, Library}, active_units_of(&GameState, PlayerId) (anything with .len()),
  is_locked(&GameState, &ZoneSlot), ZoneSlot { player, row, lane: i32 }}`; `plague::{plague_on(&CardInstance)
  -> i32, permanents_on_field(&GameState, impl Into<Option<PlayerId>>)}`; `mana::effective_cost(&GameState,
  &CardInstance, Default::default()) -> i32`.
- Tests: `params::step_param(&mut CardInstance, &str, i32)`, `tuning::{tuning_of(&mut CardInstance) ->
  &mut Tuning, add_step(Option<&IndexMap<String, i32>>, &str, i32) -> IndexMap<String, i32>}`,
  `catalog::fused_id_parts(&str)` (any `Serialize` answer: compared as JSON), `scripts::registered_scripts()
  -> IndexMap<String, CardScripts>`, `subsystems::ACTIVATIONS_MEMORY_KEY` (`.to_string()`), `reduce(&GameState,
  &Action) -> ReduceResult`, `legal_actions`, `hash_state`, `GameState: PartialEq + Deserialize`,
  `layers::UnitView { attack, keywords, position }` (`position` serialises as "ATK"), the testkit's
  `register_scripts` override, `jackioh_cards::register_all()`.

## GAPS
- No function was left out of any file.
- `effects::each::ForEachCardArgs` (C #79): built by struct literal, since its fields are functions —
  `cards: Arc<dyn Fn(&EffectContext<'_>) -> Vec<String> + Send + Sync>` and `each: Arc<dyn Fn(&str) ->
  Effect + Send + Sync>`. Part 24.1's notes guessed `&mut EffectContext` for `cards`, part 24.3's `&EffectContext`;
  I chose `&EffectContext` (a pure read, SURFACE §6.5). If part 6.2 wrote `&mut`, the two closure
  annotations in `c079_risky_die.rs` change from `&EffectContext<'_>` to `&mut EffectContext<'_>`.
- `effects::cast::CastNewArgs` (C #84's test fixture only): part 6.2's notes say it derives `Clone` only
  (no serde), so it is a struct literal `CastNewArgs { def: VANILLA.into(), radiant: None, how:
  Default::default() }` — the field names `def`/`radiant`/`how` and `CastNewDef: From<&str>` are assumed.
- `lock_played_zone`'s argument must deserialize `{ "event": <GameEvent JSON> }` (TS `{ event?: GameEvent }`).
- `reveal`'s argument type must be `Default` (TS `args = {}`).

## Decisions
- TS `expect(radiant).toBe(base)` (one script object on both faces) is `Arc::ptr_eq` on a hook both faces
  hold after `base.clone()` (`cry`, `cost_aura`, `death`, the activation's `run`); Genn's `toEqual({})` and
  `toBe(base)` are a field-by-field emptiness check of both faces (`is_empty_script`).
- Each test module shadows `scenario` with a wrapper that calls `crate::register_all()` first (TS's harness
  registered on import); TS `def` is a local `def()` over `crate::card_def(ID)`.
- Views, legal actions and some events are asserted as JSON (`serde_json::to_value`) with TS's own keys
  (`you.hand[].cost`, `pending.forYou`, `counteredOnPlay`, `x`, `tributes`, `modes`, `targets`), so the tests
  pin the wire shape, not Rust field names; `toMatchObject` is a private `matches_object`.
- C #87: TS read the live `ctx.self`, so `matches` reads `ctx.live_self()` — the count on the card now.
- C #82: TS's module-level `WORTH` (from `def.params`, throwing when absent) is `worth()` over
  `crate::card_def(ID)` at `script()` time, panicking with TS's message.
- C #88: TS's `Read = { state, self, radiant }` is `HookArgs` (`type Read`); the preview's
  `ConditionContext` is narrowed to it by `read_of`, so `param` reads through `HookArgs`.
- C #88's factory `siphon(mod, preview)` takes its two TS arrow functions as plain `fn` pointers (they
  capture nothing); every stored hook is still an `Arc` from part 1's builders (`hook.style`), and the
  aura's `applies` is part 1's `Box` (`AuraEntry`).
- C #78: `CHOSEN` is a `const TargetSpec`; the effect literals put it into `json!` as TS's `{ of: "chosen" }`.
- C #80/#83/#84/#85/#86 `SideSetup` spreads (`{ ...x, ...SPARE }`, `{ ...SPARE, ...x }`) are small
  key-merge helpers in the test module (`with_spare`, `spare_with`, `p2_with`).
- TS default parameters are explicit (`board(radiant, library)`, `standing(tokens, radiant, active)`,
  `set_against(enemies, radiant, mine)`); helpers that took optional bags take a `json!` bag (`fusing`,
  `standing` in #81).
- `stepParam(s.card(x), …)` and other writes through a live card go through `find_instance_mut(s.state_mut(), id)`.
- Test names: the TS title snake_cased (`§` → `s`, `#` → `n`), R-ids leading as in TS; the exact title is
  the doc comment. #87's second top-level `describe` is `mod r667_the_warning_on_the_viewer_s_hand_patch_v0_2_7`.
