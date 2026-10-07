# Slice: part 15 (cards lane 7: Classic+ #19–#49), chunk 4 of 4 — issue #403, parent #306
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All six were absent on `staging` (no night-bot half-port, no `part-15-bot.md`); each is the whole port of
its TS script and TS test, every function in TS order, header and rule-citing comments kept, one
`#[test]` per TS `it` (counts checked: 10, 12, 15, 16, 11, 11), one `mod` per `describe`:
- `crates/cards/src/scripts/classic_plus/c045_complexity_audit.rs` ← `045-complexity-audit.ts` + test
- `crates/cards/src/scripts/classic_plus/c046_1_felinor_flagbearer_prime.rs` ← `046-1-felinor-flagbearer-prime.ts` + test
- `crates/cards/src/scripts/classic_plus/c046_felinor_flagbearer.rs` ← `046-felinor-flagbearer.ts` + test
- `crates/cards/src/scripts/classic_plus/c047_joggs_box.rs` ← `047-joggs-box.ts` + test
- `crates/cards/src/scripts/classic_plus/c048_jlockheeds_lobbyist.rs` ← `048-jlockheeds-lobbyist.ts` + test
- `crates/cards/src/scripts/classic_plus/c049_jay_fungus.rs` ← `049-jay-fungus.ts` + test
No `todo!`, `unimplemented!` or `// TODO`.

## SURFACE
Each file: `pub const ID`, `pub fn script() -> CardScripts`, nothing else public (SURFACE §7.1). Part 1's
frozen shapes where they differ from SURFACE: hooks take `&mut EffectContext`; pure-read hooks take their
`Copy` bundle by value through `aura_hook`/`condition_hook`; `AuraEntry { applies: Box<dyn Fn>, mod_ }`.

## GAPS
Names I call that other parts provide (TS name snake_cased at its TS module's path; the shape I assumed):
- `params::param(&impl ParamContext, &str) -> i32` (part 2.1's notes): `param(&*ctx, …)` on a
  `&mut EffectContext`, `param(&args, …)` on `HookArgs`.
- `catalog::def_of(Option<&GameState>, &str) -> &CardDef` (part 2.2's notes).
- `effects::each::{for_each_card, ForEachCardArgs { cards: Arc<dyn Fn(&EffectContext) -> Vec<String> + Send
  + Sync>, each: Arc<dyn Fn(&str) -> Effect + Send + Sync> }}` (part 6.2; C+ #45). If `cards` takes
  `&mut EffectContext`, change the closure's annotation in `c045`'s `audit`.
- `effects::cast::{cast_random, CastRandomArgs { query: CastRandomQuery::Fixed(CatalogQueryArgs), count:
  Option<CastRandomCount::Fixed(i32)>, radiant: Option<bool>, how: <Default> }}` (part 6.2; C+ #47). Built
  in one private helper, `c047`'s `cast_spells`, so a different field layout is one edit.
- Effects built with `json_as(json!(…))` from the TS literal (their argument types must deserialise it):
  `exile` (`{ target: { of: "instance", instanceId } }`), `summon_copy` (`{ of: { of: "self" } }`),
  `shuffle_into` (`{ defId, count }`), `add_random_from_catalog` (`{ query: { tags }, costOverride, radiant? }`),
  `discount_random_in_hand` (`{ amount }`).
- `effects::choose::chosen_options(&EffectContext) -> Vec<String>`; `effects::card_scope::unreadable_by(&GameState,
  &CardInstance) -> Vec<PlayerId>`.
- `subsystems::audit::{lines_of_code(&GameState, &str) -> i32, audit_targets(&GameState, AuditArgs { controller,
  active, loc, more, enemy_only })}` (part 8.1 names the struct `AuditArgs`; part 25.1 wrote `AuditTargetsArgs`).
  Its answer may be `Vec<CardInstance>` or `Vec<&CardInstance>`: `c045`'s `targets` clones either.
- Tests: `subsystems::fuse::{fuse(&mut EngineSink, FuseArgs) -> Option<CardInstance>, FuseArgs { ingredients,
  target: Option<CardInstance>, .. }: Default}`; `subsystems::call_to_chaos::{chaos_chain_of(Option<&CardInstance>)
  -> i32, cast_random_call_to_chaos() -> Effect}`; `scripts::registered_scripts() -> IndexMap<String,
  CardScripts>`; `zones::lock_zone(&mut GameState, impl Into<ZoneSlot>)`; `params::step_param(&mut CardInstance,
  &str, i32)`; `mana::effective_cost(&GameState, &CardInstance, CostOptions: Default)`;
  `query::cards_played_this_turn(&GameState, PlayerId) -> i32`; `replay::hash_state`; `reduce::reduce(&GameState,
  &Action) -> ReduceResult`; `state::{new_instance, find_instance_mut}` (part 1).
- Testkit (part 5.1's notes): `scenario`, `Scenario::{play, attack, answer, end_turn, switch_position, state,
  state_mut, events, last_events, view, unit, hand, pile, card, stats, expect_in_zone, expect_stats,
  expect_health, expect_refused_with}`, `register_catalog_as(CardDefs, &str)`, `register_scripts(IndexMap<String,
  CardScripts>)`; `layers::UnitView { keywords: Vec<Keyword>, position: Position, .. }`. Both registries are
  named by full path (`jackioh_engine::testkit::…`): a card test's `use super::*` brings the prelude's
  production `catalog::register_catalog`, which would make a bare `register_catalog` ambiguous with the
  testkit's (part 1's notes list six colliding verbs; this is a seventh).
- Wire/engine types assumed to derive `PartialEq` + `Debug` for the assertions: `GameState`, `Zone`,
  `LibraryEntryView`, `Keyword`, `PrintedRarity`, `Rarity` (all part 1's and derive them).

## Decisions
- `export const radiant: Script = base` is `base.clone()`; TS `expect(radiant).toBe(base)` (identity) is
  `Arc::ptr_eq` on the hooks the two faces share (#46.1, #49), and `expect(radiant.cry).toBe(base.cry)` on #47's
  `cry` (`{ ...base, staticFlags }` is `Script { static_flags, ..base.clone() }`).
- #45's `audit(enemyOnly)` takes the predicate as `impl Fn(&EffectContext) -> bool`, held in an `Arc` the hook
  clones into each `forEachCard`; `forEachCard`'s `cards` answers ids (part 6.2), so the card maps its
  instances to `card.id`. `run.self?.defId ?? run.defId ?? def.id` is `self_` → `def_id` → `ID`.
- #45's test `EQUAL_PAIR` (a module-load throw in TS) is a function that panics with TS's message; `fuseEqual`
  builds its own `EngineSink` over `s.state_mut()` with a side `Rng` whose cursor is never written back, as TS.
- #47's test: `box` is a Rust keyword, so the helper is `box_` with a `BoxOpts` (`seeded(seed)` + struct
  update). TS's `try/finally` restores are a small drop guard (`Finally`). The R593 test's shared `runs` array
  is an `mpsc` channel (the hook must be `Fn + Send + Sync`, and `Mutex`/`RefCell` are banned by clippy.toml),
  drained after each seed's play (TS's `runs.length = 0`). `Played` is a struct of the four `cardPlayed` fields
  the tests read. `CARDS[id].base.targets` is `crate::scripts_of()`, built once before the loop.
- `toEqual` on engine values against TS literals is compared as JSON (`serde_json::to_value`); `toMatchObject`
  on an event is a `matches!` with a guard. Test names follow part 11's convention (`§` → `s`, `#` → `n`,
  a leading digit gets `t_`, ruling tokens lead).
