# Slice: part 9 (cards lane 1: Core #1–#48), chunk 3 of 4 (#397, parent #306): Core #32–#41
BUILDS-RUN: 0

No `cargo`, `rustc`, `rustfmt`, `clippy`, `pnpm`, `tsc` or test runner was run.

## FILES
All ten were absent on `staging` (no bot half-port, no `part-09-bot.md`); all are full ports, the TS
script's header as `//!`, every TS function in TS order with its doc and rule comments, `pub const ID`,
`pub fn script() -> CardScripts`, and the TS test file's header above a `#[cfg(test)] mod tests` with
one `mod` per `describe` and one `#[test]` per `it` (a TS `for (const radiant of [false, true])` loop is
one `#[test]` per face, both calling one shared fn). No `todo!`, `unimplemented!` or `// TODO`.
- `crates/cards/src/scripts/core/c032_prem_panther.rs` (15 tests)
- `crates/cards/src/scripts/core/c033_unstable_clone_machine.rs` (17)
- `crates/cards/src/scripts/core/c034_collateral_damage.rs` (9)
- `crates/cards/src/scripts/core/c035_lunar_eclipse.rs` (10)
- `crates/cards/src/scripts/core/c036_magic_jammed.rs` (7)
- `crates/cards/src/scripts/core/c037_gravedigger.rs` (8)
- `crates/cards/src/scripts/core/c038_quickstriker.rs` (19: 14 + 2 faces × 2 + 1)
- `crates/cards/src/scripts/core/c039_recycling_initiative.rs` (16)
- `crates/cards/src/scripts/core/c040_echoes_of_the_forgotten.rs` (13)
- `crates/cards/src/scripts/core/c041_sheepish.rs` (18: 16 + 2 faces)

## GAPS
No function left out. Names called in other parts' modules (TS name snake_cased at its TS module's
Rust path, rule 6), with the shape assumed:

### Scripts
- Every effect verb takes ONE argument built with `json_as(json!(<the TS literal>))`, so only its serde
  shape (TS's keys, camelCase, `TargetSpec` tagged on `of`, `PlayerSpec` "self"/"enemy") matters, never
  the Rust struct's name: `effects::draw::draw` (`{count}`), `effects::shuffle_into::shuffle_into`
  (`{defId, count, radiant, copyOf}`), `effects::move_::{exile, exile_adjacent_to, bounce}` (`{target}`),
  `effects::library::{exile_random_from_library ({count, player}), exile_bottom_of_library ({player})}`,
  `effects::damage::damage` (`{to, amount}`), `effects::player_mods::add_player_modifier`
  (`{player, mod: {kind: "costDiscount", amount, onlyType, oncePerTurn, expiry: {until, turn}}}`),
  `effects::destroy::destroy`, `effects::counters::lock` (`{zone: {of: "chosen"}}`),
  `effects::steal::steal` (`{target}`), `effects::add_to_hand::{add_to_hand ({defId, player?, radiant?,
  costMod?, costOverride?}), add_random_from_graveyard ({player})}`, `effects::choose::
  discover_from_graveyard` (`{step, prompt}`), `effects::cost::set_cost_mod` (`{target, amount,
  inHandOnly}`), `effects::delay::delay` (`{at: {phase, player}, step, hook, data?}`),
  `effects::transform::transform` (`{instanceId, defId}`). They are imported explicitly
  (`use jackioh_engine::effects::{…}`), so an engine name in a prelude glob cannot make them ambiguous.
- `combat::after_attack_of(&EffectContext) -> Option<AfterAttackFacts>` with `destroyed_ids: Vec<String>`
  (#32; called with the hook's `&mut EffectContext`, which coerces). If the port takes the data bag
  (`&IndexMap<String, Value>`), pass `&ctx.data`.
- `query::cards_played_this_turn(&GameState, PlayerId) -> i32` (#38 preview),
  `query::played_ids_this_turn(&GameState, PlayerId) -> Vec<String>` (#39; any item with `to_string()`
  compiles), `query::zone_count(&GameState, PlayerId, zones::OffFieldZone) -> i32` (#40; a `usize`
  answer needs `as i32`), `query::left_field_since_resolved(&GameState, &GameEvent) -> bool` (#41, part
  2.2's shape), `catalog::def_of(Option<&GameState>, &str) -> &CardDef` (#41, part 2.2's shape; drop the
  `Some(…)` if it takes `&GameState`), `prompts::RESUME_HOOK: &str` (#39), `zones::OffFieldZone::Exile`.
- Frozen names used as written: `TriggerDef::new(..).with_when(..)`, `condition_hook` (the two
  previews), `StaticFlags { quickstriker: Some(FlagOrCount::Flag(true)), .. }`, `PreviewValue`,
  `TargetDecl::target`, `state::find_instance`, `wire::FaceKind`, `GameEvent::CardResolved`.

### Tests
- `testkit` as part 5.1's notes give it: `scenario(Value) -> Scenario`; steps `play(ref, Value)`,
  `attack(ref, ref | "hero")`, `answer(Value)`, `end_turn()`, `start_turn()` returning `&mut Scenario`;
  reads `state()`, `state_mut()`, `events()`, `last_events()`, `view(seat)`, `unit`/`backrow(seat, i32)
  -> Option<CardInstance>`, `hand(seat)`/`pile(seat, &str) -> Vec<CardInstance>`, `card(ref) ->
  &CardInstance`; assertions `expect_in_zone`, `expect_stats`, `expect_events(json!([…]))`,
  `expect_health`, `expect_mana`, `expect_refused_with(|s| …, "text")`. Seats passed as `"p1"`/`"p2"`
  (`&str: SeatRef`, also a non-`'static` `&str` in two helpers); card refs as `&str`, `&String` (an
  instance id, as TS passed `target.id`) or `&CardInstance`.
- `crate::register_all()` before every `scenario` (part 5.1: the engine's testkit cannot call it) and
  `crate::card_def(&str) -> CardDef` (part 1's cards `lib.rs`).
- `mana::effective_cost(&GameState, &CardInstance, CostOptions)` with `Default::default()` (#37, part
  4.1's shape).
- `subsystems::fuse::{fuse(&mut EngineSink, FuseArgs) -> Option<CardInstance>, FuseArgs { ingredients:
  Vec<CardInstance>, target: Option<CardInstance>, ..Default::default() }}` (#38's R281 fused test,
  part 6.2's shape for part 8's module).
- `config::LIBRARY_CAP` compared `as i32` (whatever its integer type).
- `std::sync::Arc::ptr_eq` stands for TS's `expect(radiant).toBe(base)` (#38): the radiant script is
  `base.clone()`, so both faces hold the same preview `Arc`.

## Decisions
- Effect arguments are always the TS literal through `json_as` (SURFACE §6.6), including the two built
  conditionally (#39's `costMod` only when the face discounts, `data` only with a self), which are
  written into the `Value` before `json_as`.
- TS module constants of declarations (`const targets: TargetDecl[]`) are private fns (`targets()`,
  `backrow_target()`): a `Vec` built from JSON cannot be a `const`.
- `TrapTrigger` (TS `traps.ts`: `= TriggerDef`) is written `TriggerDef` (#41), so nothing depends on
  whether `traps.rs` kept the alias. TS's `Extract<GameEvent, { type: "cardResolved" }>` (#33) is a
  private borrowed struct `ResolvedEvent` read off `GameEvent::CardResolved`.
- #40's constant objects `PER_EXILED_CARD`/`FORMULA` are `const`s of a private `PerFace<T>` indexed by
  `wire::FaceKind` (SURFACE §4.2's constant-object rule); TS's `"base" | "radiant"` is `FaceKind`.
- #39's `new Set<string>()` is an `IndexSet<String>` (`indexmap` is a dependency of the cards crate).
- #35 keeps TS's literal arguments `eclipse(3, 1)` / `eclipse(6, 2)` as TS wrote them; #32, #33, #37,
  #39's named numbers stay file `const`s (brief step 4).
- #35's header quotes a two-line TS fix as an indented block; it is fenced ```` ```text ```` so rustdoc
  does not compile it as a doctest. No other header has a 4-space-indented block after a blank line.
- Test names: the TS title snake-cased by §4.2; `§6.1` → `sec6_1`, `#9` → `9`, punctuation dropped,
  ruling tokens kept where TS put them (leading ones lead: `r426_r53_…`). Describe titles name the mods
  (`base`, `radiant`, `r316_what_a_full_library_turns_away`, …).
- TS live-object writes in tests (`s.card(x).radiant = true`, `grantedKeywords.push(…)`) go through
  `find_instance_mut(s.state_mut(), &id)`; a `Keyword` is built with `json_as(json!({ "kind": "Reborn" }))`.
- Reads of views (`s.view(p).you…`, view events, `toMatchObject`, `JSON.stringify(view)`) and of event
  fields TS matched by key (`killerId`, `libraryOverflow`'s `player/defId/outcome`) go through
  `serde_json::to_value`, so the assertions pin the wire JSON; elsewhere events are matched on the
  frozen `GameEvent` variants with `..`. `toBeUndefined` on a view key is `is_null()` on the indexed
  `Value` (absent and null both read as null).
- `_glow.ts`'s `glows`/`handGlows`/`backrowGlows` are private copies in #38 and #41 (rule 5), reading
  the view's JSON and asserting a present `conditionActive` is `true`, as TS did.
- #41's "the paused game is plain JSON" is `GameState` → `Value` → `GameState` and `==` (it derives
  `PartialEq`).
- #34's seeded-repeat test keeps TS's closure (`run`) building the same scenario twice.
