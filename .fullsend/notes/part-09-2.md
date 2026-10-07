# Slice: part 9 (cards lane 1: Core #1–#48), chunk 2 of 4 — Core #19–#31 (#397, parent #306)
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All thirteen were absent on `staging` (no bot port to finish, no `.fullsend/notes/part-09-bot.md`); each is
now the whole port of its TS script and TS test, every script function in TS order with its doc and
rule comments, every TS `it` as one `#[test]` (TS `describe` → nested `mod`), counted against the TS:
- `crates/cards/src/scripts/core/c019_midrange_menace.rs` (9 tests)
- `crates/cards/src/scripts/core/c020_pointmaster.rs` (7)
- `crates/cards/src/scripts/core/c021_hinder.rs` (13, the real-game fold test included)
- `crates/cards/src/scripts/core/c022_carnivorous_cube.rs` (12)
- `crates/cards/src/scripts/core/c023_reoccurring_dream.rs` (8)
- `crates/cards/src/scripts/core/c024_efficiency_dividend.rs` (20)
- `crates/cards/src/scripts/core/c025_4_mana_7_7.rs` (7)
- `crates/cards/src/scripts/core/c026_glowy_jelly_bean.rs` (6)
- `crates/cards/src/scripts/core/c027_blood_ridden_glowy_jelly_bean.rs` (7)
- `crates/cards/src/scripts/core/c028_knockoff_temu_glowy_jelly_bean.rs` (7)
- `crates/cards/src/scripts/core/c029_giga_glowy_jelly_bean.rs` (7)
- `crates/cards/src/scripts/core/c030_archivist.rs` (11)
- `crates/cards/src/scripts/core/c031_kys_math_equation.rs` (16)
No `todo!`, `unimplemented!` or `// TODO`. Every R-id of each TS script and test file is in its Rust file
(as a comment, or as a leading `r<n>` token of a test name).

## SURFACE
Each file: `pub const ID: &str`, `pub fn script() -> CardScripts` (SURFACE §7.1); nothing else public.
Part 1's frozen code wins over SURFACE where they differ: hooks are `hook(|ctx| …)` with `ctx: &mut
EffectContext`; typed hooks take part 1's by-value bundles (`condition_hook(|c: ConditionContext<'_>| …)`
for #31's `preview`).

## DEPENDS-ON
Frozen (part 1): `prelude::*` (`json!`, `json_as`, `Value`, `Arc`), `Script`, `CardScripts`, `StaticFlags`,
`TargetDecl::{target, hand, tribute}`, `ModeDecl`, `PromptKind`, `Selection`, `Zone`, `Row`, `ZoneName`,
`AttackHealth`, `CardType`, `PreviewValue`, `PreviewHook`, `condition_hook`, `EffectContext::live_self`,
`Rng::{chance, lucky}`, `config::{fib, UNIT_ZONES, MAX_MANA}`, `state::{find_instance_mut, new_instance,
CreateGameOptions, create_game, PlayerModifier}`, `Action::new`, `ActionBody::Mulligan`, `ManaView`,
`jackioh_cards::{register_all, card_def}`.

## GAPS
Nothing left out. Names I call that other parts provide (TS name snake_cased at its TS module's path),
with the shape I called them in:

Effects (parts 6–7), each argument built with `json_as(json!(…the TS literal…))`, so only its serde shape
(TS's) matters: `effects::heal::heal` (`{ target, toFull }`, `{ target, amount }`),
`effects::mana::next_turn_mana` (`{ amount, player }`), `effects::move_::{discard_random ({ count }),
bounce ({ target: { of: "self" } })}`, `effects::memory::remember ({ key, value })`,
`effects::destroy::sacrifice ({ target })`, `effects::summon::{summon, fill_board}` (`{ defId, radiant,
statsOverride?, armorOverride? }`), `effects::radiant::{set_radiant ({ target: { of: "chosen", index } } /
{ instanceId }), set_radiant_random ({ zones: "hand" | [..], count })}`, `effects::damage::damage
({ to, amount })`, `effects::draw::draw_from_library ({ instanceId })`, `effects::cost::set_cost_mod
({ amount, inHandOnly })`, `effects::lose_health::lose_health ({ player, amount })`.
Effects by Rust shape:
- `effects::targets::{TargetSpec::Chosen { index: None }, instance_of(&EffectContext, &TargetSpec) ->
  Option<CardInstance>}` (part 7.1's notes).
- `effects::choose::chosen_options(&EffectContext) -> Vec<String>`.
- `effects::each::{for_each_card(ForEachCardArgs) -> Effect, ForEachCardArgs { cards: Arc<dyn Fn(&mut
  EffectContext) -> Vec<String> + Send + Sync>, each: Arc<dyn Fn(&str) -> Effect + Send + Sync> }}` (#30
  radiant). Part 24.1 wrote the `cards` closure over `&mut EffectContext`, part 24.3 over `&EffectContext`;
  I wrote `|at: &mut EffectContext<'_>|` — part 31: if `each.rs` took `&EffectContext`, drop the `mut`.
Engine reads (parts 2 and 4):
- `query::{recalled(&EffectContext, &str) -> Option<Value> (part 2.2's notes), zone_cards(&GameState,
  PlayerId, OffFieldZone) -> Vec<_> (owned or borrowed: only `.iter()`, `.is_empty()` and `card.id` are
  read), was_played_this_turn(&GameState, PlayerId, &CardInstance) -> bool}`.
- `catalog::def_of(Option<&GameState>, &str)` (part 2.2's shape; parts 6–7 assumed `&GameState`):
  `def_of(Some(&*ctx.state), id).type_` in #22.
- `zones::{OffFieldZone::{Hand, Library}, slots_of(PlayerId, Row) -> Vec<ZoneSlot>, card_at(&GameState,
  &ZoneSlot) -> Option<&CardInstance>, active_units_of(&GameState, PlayerId)}` (#29).
- `mana::{effective_cost(&GameState, &CardInstance, CostOptions /* Default::default() */) -> i32,
  printed_cost(&GameState, &CardInstance) -> i32}` (part 4.1's three-argument shape).
- `times_played::times_played_of(&CardInstance) -> i32`.
Testkit (part 5.1's notes): `scenario(Value) -> Scenario`; `play(card, Value)`, `attack(card, card | "hero")`,
`answer(Value)`, `end_turn()`, `start_turn()` returning `&mut Scenario`; `state()`, `state_mut()`, `events()`,
`last_events()`, `view(PlayerId)`, `unit(PlayerId, i32)`/`backrow(..) -> Option<CardInstance>` (owned),
`hand(PlayerId)`/`pile(PlayerId, "graveyard") -> Vec<CardInstance>`, `card(ref) -> &CardInstance`;
`expect_in_zone`, `expect_stats(ref, Value)`, `expect_events(Value)`, `expect_health`, `expect_mana`,
`expect_refused(|s| …)`, `expect_refused_with(|s| …, "text")`. A card ref is `&str` (id, index or catalog
id), `&String` or `&CardInstance`.
Engine entry points (#21's real game): `reduce::{begin_game(&GameState), reduce(&GameState, &Action)} ->
ReduceResult { state, events, error }`, `replay::{fold(&FoldArgs) -> FoldResult { state, errors },
hash_state(&GameState)}`; `FoldArgs` must `Deserialize` from `{ seed, decks, log }` (built with `json_as`).

## Decisions
- Header comments: the TS file's header as `//!`, the TS test file's header as `//` lines right above
  `#[cfg(test)]`, both verbatim (their `engine/src/*.ts` references kept as TS wrote them). #24's
  "A card file exports only `{ def, base, radiant }`" reads `{ ID, script }`, the Rust card contract.
- TS's live `ctx.self` reads of zone, count, cost or flags (#23 and #24's return gate, #31's `blast` and
  `returnToHand`) use `ctx.live_self()`; reads of the id only (#22's "cannot eat itself") use `ctx.self_`.
- #22: `memory.eaten` is a JSON object written in TS's key order (`defId`, `radiant`, then the optional
  `statsOverride`, `armorOverride` only when present); `eaten_of` re-reads it defensively (SURFACE §4.4.10):
  a non-object or a missing `defId` is "nothing eaten", a malformed `statsOverride` is dropped.
- #23 radiant: `ctx.rng.lucky(1, |rng| rng.chance(0.4), |a, b| a || b)` — Rust's `lucky` hands the roll
  the rng, so the draws are TS's two `next()`s in the same order.
- #24: `Math.max(0, Math.trunc(ctx.x))` is `ctx.x.max(0)` (`x` is `i32`); `Math.floor(x / 2)` on that
  non-negative `x` is `x / 2`.
- #28: `ZONES` is a `[&str; 3]` serialised into the effect's `zones`; #29's `make_radiant` takes the ids
  its two readers collect (TS mapped the instances to their ids in the same place).
- #30: `extreme` takes `want: &str` (`HIGHEST` or `LOWEST`, TS's literal union); its tie rule is TS's.
- #31: `FORMULA` is computed in `script()` from `crate::card_def(ID)` (TS computed it at import); TS's
  `/Fib\([^)]*\)/` is matched by hand (`formula_in`, no regex crate), panicking with TS's message.
  `FIB_OFFSET` is a private `Copy` struct const (TS `{ base: 1, radiant: 3 } as const`).
- Tests: every `#[test]` that builds a scenario calls `crate::register_all()` first (part 5.1: the testkit
  cannot name the cards crate). Names: the TS title snake_cased, `§` → `s`, `#` → `n`, ruling tokens
  leading (`r431_r90_…`). TS `toMatchObject` is a private `matches_object` in each file that needs it;
  TS `toEqual` on declarations or events compares `serde_json::to_value`; TS `expect(script).toEqual({})`
  (#25) is a private `is_empty_script` that checks every `Script` field. TS fields the harness cannot seed
  (`statsOverride` in #22, `timesPlayed`/`costOverride` in #31, the Stack top in #28/#29, a player modifier
  in #31) are written through `state_mut()` and `find_instance_mut`, as TS wrote through `s.state`.
- #21's real game: TS's file-level `let nonce` is one counter declared before the seed loop, so the found
  game's nonces are the same `hinder-game-<n>` TS would have given it.
