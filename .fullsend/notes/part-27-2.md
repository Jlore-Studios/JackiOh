# Slice: part 27, chunk 2 of 8 (engine tests 4: play pipeline and cross-card rules), #415 under #306
BUILDS-RUN: 0

## FILES
All nine ported whole (every `describe` as a `mod`, every `it` as a `#[test]`, in TS order, header
and rule comments kept; TS local helpers are private fns in their file). `#[test]` counts equal the
TS `it` counts file by file, 54 in all: `crates/cards/tests/cross/{deaths_and_reborn (10),
echo_and_exile (6), forced_attacks (4), fuse_registry (1), fused_hooks (18), fused_nested_resume (1),
fused_target_checks (2), game_over (4), hand_returns (8)}.rs`. Notes: this file,
`part-27-2.assumptions`, `spec-gaps-part-27-2.md` (no gaps). Nothing outside these paths was written;
no rebase conflicted. Nothing under `crates/*/src/` other than part 1's frozen files was opened.

## GAPS
No test was left unported (`spec-gaps-part-27-2.md` lists none).

Names called that other parts provide (the signature each call assumes; part 31 reconciles):

**Testkit, part 5** (`jackioh_engine::testkit`, SURFACE §8):
- `scenario(Value) -> Scenario`. Every file wraps it in a local `scenario` that calls
  `jackioh_cards::register_all()` first: the engine's testkit cannot (cards depends on engine), and
  TS's harness registered the real cards on import.
- `Scenario`: `play(&mut self, card: &str, opts: Value)`, `attack(&mut self, &str, &str)` (`"hero"`
  or a card ref), `answer(&mut self, Value)` (a string, a list of strings or of selections, as TS),
  `end_turn(&mut self)`, `start_turn(&mut self)` — return values never used, so `()` or `&mut Self`
  both fit; `state(&self) -> &GameState`; **`state_mut(&mut self) -> &mut GameState`** (not in
  SURFACE §8's table: TS tests write through `s.state` — transient defs, `rngCursor`, a library
  unshift, a granted keyword — and pools_and_randomness.rs uses the same name); `events(&self)` and
  `last_events(&self)` as `&[GameEvent]` (or `&Vec`); `view(&self, PlayerId) -> PlayerView`;
  `unit(&self, PlayerId, i32) -> Option<&CardInstance>` and `backrow` the same (lanes are `i32`);
  `hand(&self, PlayerId)` a list (`.len()`, `.iter()`); `pile(&self, PlayerId, &str)` with TS's
  pile name (`"graveyard"`); `card(&self, &str) -> &CardInstance`, a ref being a catalog id, an
  index, a name or an instance id (every TS `CardInstance` ref is passed as its `id`);
  `stats(&self, &str) -> UnitView` (`layers::UnitView`: `attack`, `max_health`, `health`,
  `keywords: Vec<Keyword>`, `armor`); `expect_in_zone(&self, &str, &str)`, `expect_stats(&self, &str,
  Value)`, `expect_events(&self, Value)` (a JSON array of type strings, TS's rest arguments),
  `expect_health(&self, PlayerId, i32)`, `expect_mana(&self, PlayerId, i32)` — taken by `&self`
  (fused_nested_resume.rs holds its finished scenario in a plain `let`).
- The thread-local `register_scripts(IndexMap<String, CardScripts>)` (SURFACE §8) and
  `scripts::registered_scripts()` (owned or `&'static`; the tests `.clone()` it and insert).

**Engine** (through `jackioh_engine::testkit::*` unless a path is given):
- `zones` (part 2): `ZoneSlot { player, row, lane: i32 }` built as a literal;
  `place_on_field(&mut GameState, &mut CardInstance, &ZoneSlot, Default::default()) -> bool` (the
  form part 24's `fixtures/harness.rs` uses; part 26 passed the slot by value).
- `catalog` (part 2): `def_of(&GameState, &str)` returning a `CardDef` or `&CardDef` (`.base.keywords`
  read); `query(&CatalogQueryArgs) -> Vec<CardDef>`, the argument built with `json_as(json!({ "type":
  [...] }))` so its type name is inferred.
- `damage` (part 3): `hero_armor_of(&GameState, PlayerId) -> i32`.
- `scripts` (part 2): `INGREDIENTS_KEY: &str`; `ingredients_of(&CardInstance) ->
  Option<Vec<IngredientRecord>>` with `embiggened: bool`.
- `reduce`, `legal_actions`, `view_for` (SURFACE §6.1); `ReduceResult.error: Option<String>`.
- `subsystems::fuse` (part 8): `fuse(&mut EngineSink, FuseArgs) -> Option<CardInstance>` and
  `FuseArgs { ingredients: Vec<CardInstance>, target: Option<CardInstance>, to_hand: Option<PlayerId>,
  … }` **deriving `Default`** (built as `FuseArgs { …, ..Default::default() }`; every TS field but
  `ingredients` is optional).
- `subsystems::hero_power` (part 8): `POWER_KEY: &str`, `power_of(&CardInstance) -> Option<_>`,
  `power_ability_of(&GameState, &CardInstance) -> Option<ActivationDecl>` (or `Option<&…>`; `.id`
  read).
- `subsystems::activate` (part 8): `why_cannot_activate_ability(&GameState, PlayerId, &str,
  Option<&str>) -> Result<(), EngineError>` (TS's `string | null`; part 26 assumes the same).
- `subsystems::call_to_chaos` (part 8): `CHAOS_EFFECTS` a slice of effects with a `name` comparable
  to `&str`; `roll_chaos_effects(&mut Rng, bool) -> Vec<_>` of the same; `CHAOS_CHAIN_KEY: &str`
  (pools_and_randomness.rs calls `roll_chaos_effects` the same way).
- `effects` (parts 6–7): `damage`, `destroy`, `draw`, `exile_matching`, `bounce`, each taking one
  `Deserialize` argument built with `json_as(json!(…))` from TS's literal (`{ "to": { "of":
  "instance", "instanceId": … }, "amount": 5 }`, `{ "target": { "of": "chosen" } }`, `{ "count": 2 }`,
  `{ "zones": ["graveyard"] }`, `{ "target": { "of": "self" } }`).

Frozen names used as part 1 wrote them: `GameEvent` variants and fields (`Destroyed`, `Damage`,
`Summoned`, `Drawn`, `Fused`, `Burned`, `Bounced`, `AddedToHand`, `CostChanged`, `ControlChanged`,
`AttackDeclared`, `TrapFired`, `CardPlayed`, `ChaosRolled`), `GameEvent::event_type`,
`GameEventType`, `ModifierKind::{EchoNextSpell, ComboDraw}`, `PendingView::ForYou`, `Selection`,
`ActionBody`, `Action::new`, `TargetDecl::target`, `TriggerDef::new`, `hook`, `Script`,
`CardScripts`, `StaticFlags` (`json_as`), `EngineSink::new`, `Rng::new`, `new_instance`,
`find_instance_mut`, `PLAYER_IDS`, `Zone`, `ZoneName`, `GameResult`, `Winner`, `GameOverReason`,
`PromptKind`, `Tag`, `Row`, `Position`, `Keyword`, `KeywordKind`, `AttackHealth`, `CardDef`
(`Deserialize`), `CALL_TO_CHAOS_CHAIN_CAP`.

## Decisions
- **Live objects.** TS holds live `CardInstance`s and writes through them (`g.card(x).radiant =
  true`, `kept.memory[k] = v`). Here a test holds an owned copy for its id, reads the card back with
  `g.card(&id)` after every step, and writes through a local `card_mut` over `find_instance_mut`.
  Where TS read a field off a stale object right after taking it (`vanilla.defId`), the copy reads
  the same. TS's optional-chained write (`units[0]?.[0]?.grantedKeywords.push`) is an `if let`.
- **`sinkFor(s)`** (fused-hooks, fused-nested-resume, fused-target-checks): a sink over
  `s.state_mut()` with an event list and an rng of its own at the state's cursor, which nothing
  writes back, as TS's did not.
- **`JSON.parse(JSON.stringify(state))`** in fuse-registry is a `serde_json` round trip, not
  `clone()`: the test's point is that only the match's JSON is kept (§9.3).
- **Expectations.** `toContainEqual` with a full event literal is `contains(&GameEvent::…)` with the
  absent optionals `None` (`former_id`, `hidden_from`); `toContainEqual(expect.objectContaining(…))`
  and `toMatchObject` are `matches!` with guards over the named fields; `toMatch(/^t-\d+:a\+b$/)` is
  a local `is_fused_id(id, "a+b")` hand check, an unanchored `/a\+b$/` is `ends_with`, the one
  alternation is `||`; `expect(() => legalActions(…)).not.toThrow()` is the call itself;
  `expect(x, message)` keeps its message. A `destroyed` killer is `Option<Option<String>>`: `None`
  for no event (TS `undefined`), `Some(None)` for `killerId: null`. `types.slice(indexOf + 1)` keeps
  TS's −1 + 1 = 0 when there is no `gameOver`.
- **Literals.** Setups and play options are `json!` copies of TS's; `[...LIBRARY]` is the const
  array itself; spread hands are built (`with_fillers`, `vec![X; n]`); the fixture `CardDef`s are
  `json_as::<CardDef>(json!(…))` of TS's object; TS's `const RADIANT_DUELIST = { … }` is a fn
  returning the `Value`.
- **TS default parameters**: `fixture(…, stats = { attack: 2, health: 2 })` takes
  `Option<AttackHealth>`, `fixtureUnit(…, keywords = [])` a `Vec<Keyword>` (every caller passes it).
- **Names** (SURFACE §7.3): every `R<n>` of a title is a leading `r<n>_` token, in title order and
  deduplicated, removed from the rest; `§`, `#`, `/` and other punctuation dropped (apostrophes
  without a gap: `pillows`), section numbers kept as digits (`§4.5` → `4_5`); a title that starts
  with `§` and cites no ruling starts `section_`. No two names in a module collide.
- `PlayerId::{P1, P2}` are imported by variant.
