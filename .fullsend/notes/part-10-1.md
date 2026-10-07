# Slice: part 10 (cards lane 2), chunk 1 of 4 — Core #49–#54 (#398, parent #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
Each the whole port of its TS script (every function in TS order, the header as `//!`, every comment
that states a rule or cites a ruling) with its TS test file as `#[cfg(test)] mod tests` (the TS test
header above it, one `mod` per `describe`, one `#[test]` per `it`: 8, 25, 8, 23, 12, 10, 12).
No `todo!`, `unimplemented!` or `// TODO`.
- `crates/cards/src/scripts/core/c049_snom_bunny_mind_control.rs` — the night bot's port (0180138),
  finished in place: tests moved into the `describe`'s mod, `crate::register_all()` first in each,
  `super::super::{ID, script}` paths.
- `crates/cards/src/scripts/core/c050_k_pop_fanatic.rs` — new.
- `crates/cards/src/scripts/core/c051_1_kys_empty_notebook.rs` — new.
- `crates/cards/src/scripts/core/c051_kys_private_tutor.rs` — new.
- `crates/cards/src/scripts/core/c052_silly_silas.rs` — new.
- `crates/cards/src/scripts/core/c053_reno.rs` — the night bot's port, finished in place: mods renamed
  after the `describe`s (`n53_reno_base`, `n53_reno_radiant`), `crate::register_all()` first in each
  test, the hook reads the state as `&*ctx.state`.
- `crates/cards/src/scripts/core/c054_straaza.rs` — new.

## SURFACE
Every file: `pub const ID: &str` and `pub fn script() -> CardScripts` (SURFACE §7.1); nothing else is
public. `use jackioh_engine::prelude::*;` only; tests `use jackioh_engine::testkit::*;` only.

## DEPENDS-ON / GAPS
No function was left out. Names called in other parts' modules, TS name snake_cased at the TS
module's path, with the shape assumed:

### Effects (parts 6, 7) — every argument built with `json_as(json!(<TS literal>))`, so each args struct must derive `Deserialize` with camelCase keys
- `effects::each::{for_each_card(ForEachCardArgs) -> Effect, ForEachCardArgs { cards, each }}` (#50's
  rider): built as a struct literal, `cards: Arc::new(|now: &mut EffectContext<'_>| -> Vec<String>)`,
  `each: Arc::new(|instance_id: &str| -> Effect)`. **Riskiest call**: `cards` is assumed to take
  `&mut EffectContext` like `Hook` (part 1); part 24.3's notes assumed `&EffectContext`. If part 6.2
  chose `&`, change the one annotation in `radiant_steal_step`.
- `effects::delay::delay({ at: { phase: "start", player: "self" }, step, hook, data, watch, mark: { mark, color } })`.
- `effects::steal::steal({ instanceId } | { target: { of: "chosen" } })`;
  `effects::radiant::set_radiant({ instanceId } | { target: { of: "chosen" } })`.
- `effects::choose::{chosen_options(&EffectContext) -> Vec<String>, choose_mode({ options, step, prompt, data? }),
  discover_from_library({ step, count, player: "self", filter: { type: [CardType…], costRange: { min?, max? } }, prompt })}`.
- `effects::add_to_hand::{add_to_hand({ defId } | { instance: { of: "chosen" } }),
  add_random_from_catalog({ query: <CatalogQueryArgs literal>, count, costOverride, radiant })}`.
- `effects::draw::draw({ count })`, `effects::rotate::rotate({ direction })`, `effects::heal::heal({ target: { of: "selfHero" }, upTo })`.
### Engine reads (parts 2, 3, 4)
- `zones::{slot_of(&GameState, &CardInstance) -> Option<ZoneSlot>, card_at(&GameState, ZoneSlot) ->
  Option<&CardInstance>` (slot by value; part 2.1 takes `impl Into<ZoneSlot>`), `is_unit_token(&GameState,
  &CardInstance) -> bool`, `lock_zone(&mut GameState, ZoneSlot)`, `ZoneSlot { player, row, lane: i32 }`,
  `OffFieldZone::Library}`.
- `query::{zone_cards(&GameState, PlayerId, OffFieldZone)` (owned or borrowed items: read through
  `Borrow<CardInstance>`), `hero_of(&GameState, PlayerId).health}`.
- `catalog::{def_of(Option<&GameState>, &str) -> &CardDef` (part 2.2's shape), `query_cost(&CardDef) -> i32}`.
- `mana::effective_cost(&GameState, &CardInstance, Default::default())` (part 4.1's three arguments).
- `prompts::RESUME_HOOK: &str`.
### Cards crate (part 9)
- `crate::query::{query(&CatalogQueryArgs), pool(&str, &CatalogQueryArgs)}` (the TS `CardQuery` taken by
  reference, as the engine's `catalog::query` is), answers read through `Borrow<CardDef>`.
### Testkit and entry points (parts 5)
- `testkit::Scenario` as part 5.1's notes give it; `unit`/`backrow` answers are `.clone()`d before the next
  step, so `Option<CardInstance>` and `Option<&CardInstance>` both compile. `state_mut()` writes the
  radiant flag (`find_instance_mut`) and the rng cursor; `answer(json!("Spell"))` /
  `answer(json!(instance_id))` / `answer(json!([selection]))`.
- `replay::{fold(&FoldArgs) -> { state, errors }, FoldArgs: Deserialize` (built from TS's `{ seed, decks,
  log }` with `json_as`), `hash_state}`; `reduce(&GameState, &Action)`, `begin_game(&GameState)`,
  `create_game(&CreateGameOptions)`, `view_for(&GameState, PlayerId)`; `Action` built with `json_as`.

## Decisions
- The night bot's #49 and #53 were kept and finished, not rewritten. #53's radiant cases keep the bot's
  setup (`{ def, radiant: true }` in hand, the state TS's post-setup `s.card(x).radiant = true` builds);
  #51.1 and #52 port that TS line literally (`find_instance_mut(s.state_mut(), id).radiant = true`).
- Hooks that TS wrote as `const step: Hook = (ctx) => …` are plain `fn step(ctx: &mut EffectContext<'_>)
  -> Vec<Effect>` wrapped with `hook(step)`; TS factories (`kpopFanatic(step)`, `notebook(count)`,
  `reno(floor)`, `straaza(cost, radiant)`) are private fns returning `Script`. A hook that only reads
  rebinds `let ctx: &EffectContext<'_> = ctx;` once so its closures borrow it shared.
- TS literal unions local to a card became private enums with `as_str` (`TypeOption`, `BracketOption`
  in #51); #51's bracket range is a private `BracketRange` that writes only the bounds it has
  (`{ min: 4 }` has no `max`, as TS). #50's `STEAL_MARK` object is a private struct const with `to_json`.
  #52's direction stays a `&'static str` out of `DIRECTIONS` (TS narrowed with `find`).
- Test assertions on views, prompts, events and declarations compare JSON (`serde_json::to_value`) where
  TS compared object literals or used `"key" in obj`, so they pin the wire shape. `marks_in` answers the
  marks JSON, `Null` for a card on the field with none (TS `undefined`), or `"not on the field"`.
- `Object.keys(base).sort()` (#51) is a private `keys_of` naming every present `Script` field by its TS key.
- TS `expect(a).not.toBe(b)` on two hooks (#50) is `!Arc::ptr_eq`.
- Test names: the title lower-cased, `§` → `s`, `#` → `n`, other runs of punctuation → `_`, ruling tokens kept.
