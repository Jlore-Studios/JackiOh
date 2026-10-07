//! Destroy and Sacrifice (SPEC §6.3, §4.5, R11, R12, R46, R78, BUILD M3-T1).
//! The fixture defs and scripts these tests need are registered here, on top of the shared fixture
//! catalog, so no shared fixture has to grow for them (CLAUDE.md, BUILD §0).
//!
//! Port of `packages/engine/test/effects-destroy.test.ts`.

use jackioh_engine::effects::damage;
use jackioh_engine::effects::destroy::{destroy, sacrifice};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::token_def;
use crate::rules::fixtures::combat::indestructible;
use crate::rules::fixtures::harness::{events_of_type, new_game, put, slot};

// ---------------------------------------------------------------------------
// Fixture cards.
// ---------------------------------------------------------------------------

/// TS `unitDefOf(name, overrides)`: `nextIndex` starts at 750 and is bumped once per call, in the
/// order the TS file declares its defs; `overrides` is spread over the literal (a shallow merge).
fn unit_def_of(name: &str, index: u32, overrides: Value) -> CardDef {
    let mut def = json!({
        "id": format!("ds-{name}"),
        "index": index.to_string(),
        "name": format!("{name} (destroy)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 1,
        "base": { "attack": 2, "health": 2, "keywords": [], "text": name },
        "radiant": { "attack": 4, "health": 4, "keywords": [], "text": format!("{name} radiant") },
    });
    if let Value::Object(fields) = overrides {
        for (key, value) in fields {
            def[key.as_str()] = value;
        }
    }
    json_as(def)
}

/// A 2/2 whose Death trigger pings the enemy hero, so "counts as a death" is observable.
fn dier() -> CardDef {
    unit_def_of("dier", 751, json!({}))
}

/// Indestructible with a Death trigger: Sacrifice bypasses the ward, so the ping still lands.
fn warded_dier() -> CardDef {
    unit_def_of(
        "warded-dier",
        752,
        json!({
            "base": { "attack": 4, "health": 4, "keywords": [{ "kind": "Indestructible" }], "text": "warded" },
            "radiant": { "attack": 8, "health": 8, "keywords": [{ "kind": "Indestructible" }], "text": "warded" },
        }),
    )
}

/// #22-style: its Death hook reads what it remembered while on the field (R78).
fn rememberer() -> CardDef {
    unit_def_of("rememberer", 753, json!({}))
}

/// §4.5 step 1: a backrow card is collected only when an effect marked it destroyed.
fn field_spell() -> CardDef {
    unit_def_of(
        "field-spell",
        754,
        json!({
            "type": "Field Spell",
            "base": { "keywords": [], "text": "field spell" },
            "radiant": { "keywords": [], "text": "field spell" },
        }),
    )
}

/// TS `tokenDef("rush")` (its `tags` default, `["Token"]`, written out).
fn rush_token() -> CardDef {
    token_def("rush", &[Tag::Token])
}

fn defs() -> Vec<CardDef> {
    vec![dier(), warded_dier(), rememberer(), field_spell()]
}

fn both(script: Script) -> CardScripts {
    CardScripts { base: script.clone(), radiant: script }
}

fn ping_enemy_hero() -> Script {
    Script {
        death: Some(hook(|_ctx| vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 3 })))])),
        ..Script::default()
    }
}

fn scripts() -> IndexMap<String, CardScripts> {
    let mut scripts: IndexMap<String, CardScripts> = IndexMap::new();
    scripts.insert(dier().id, both(ping_enemy_hero()));
    scripts.insert(warded_dier().id, both(ping_enemy_hero()));
    // R78: the Death hook reads the memory this card held just before it left the field.
    scripts.insert(
        rememberer().id,
        both(Script {
            death: Some(hook(|ctx| {
                let amount = ctx.self_.as_ref().and_then(|card| card.memory.get("meal")).and_then(Value::as_i64);
                match amount {
                    Some(amount) => vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": amount })))],
                    None => vec![],
                }
            })),
            ..Script::default()
        }),
    );
    scripts
}

/// A fresh game whose catalog and script registry also carry this file's fixtures.
fn game(seed: &str) -> GameState {
    let mut state = new_game(seed, None);
    let mut catalog = registered_catalog().clone();
    for def in defs() {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    registry.extend(scripts());
    register_scripts(registry);
    state.turn = 3;
    state
}

/// TS `game()`'s default seed.
fn default_game() -> GameState {
    game("effects-destroy")
}

#[derive(Default)]
struct RunOptions {
    controller: Option<PlayerId>,
    self_: Option<CardInstance>,
}

/// A sink plus `apply`, so one test can run an effect and then the state check on the same events.
/// TS's sink held the state; here the state is lent to each call, so the test reads it in between as
/// TS read its live objects.
struct Runner {
    events: Vec<GameEvent>,
    rng: Rng,
}

/// TS `runner(state)`: a sink whose rng starts at the state's cursor, as reduce does.
fn runner(state: &GameState) -> Runner {
    Runner { events: Vec::new(), rng: Rng::new(&state.seed, state.rng_cursor) }
}

impl Runner {
    fn apply(&mut self, state: &mut GameState, effect: Effect, target: Option<&CardInstance>, options: RunOptions) {
        let targets: Vec<Selection> = match target {
            None => vec![],
            Some(target) => vec![Selection::Instance { instance_id: target.id.clone() }],
        };
        {
            let mut sink = EngineSink::new(state, &mut self.events, &mut self.rng);
            let mut ctx = make_context(
                &mut sink,
                options.self_.as_ref(),
                HookOptions {
                    controller: Some(options.controller.unwrap_or(PlayerId::P1)),
                    targets: Some(targets),
                    ..Default::default()
                },
            );
            (effect.apply)(&mut ctx);
        }
        state.rng_cursor = self.rng.cursor();
    }

    fn check(&mut self, state: &mut GameState) {
        let mut sink = EngineSink::new(state, &mut self.events, &mut self.rng);
        state_check(&mut sink);
    }
}

fn chosen() -> Value {
    json!({ "of": "chosen" })
}

/// The card as it stands in the state now (TS held the live object).
fn live<'a>(state: &'a GameState, id: &str) -> &'a CardInstance {
    find_instance(state, id).expect("the card is in the state")
}

fn live_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).expect("the card is in the state")
}

/// `cardAt(state, ref)?.id`.
fn id_at(state: &GameState, at: ZoneSlot) -> Option<String> {
    card_at(state, &at).map(|card| card.id.clone())
}

fn ids(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.id.clone()).collect()
}

/// One field of each event, as TS's `.map((e) => e.<key>)` read it.
fn pluck<T: serde::Serialize>(events: &T, key: &str) -> Vec<Value> {
    match serde_json::to_value(events).expect("events serialise") {
        Value::Array(items) => items.into_iter().map(|item| item.get(key).cloned().unwrap_or(Value::Null)).collect(),
        other => panic!("expected a list of events, got {other}"),
    }
}

// ---------------------------------------------------------------------------
// destroy
// ---------------------------------------------------------------------------

mod destroy_m3_t1 {
    use super::*;

    /// TS: "§6.3 marks the card and leaves it on the field until the state check moves it".
    #[test]
    fn marks_the_card_and_leaves_it_on_the_field_until_the_state_check_moves_it() {
        let mut state = default_game();
        let victim = put(&mut state, &dier().id, slot(PlayerId::P1, Row::Units, 2), json!({}));
        let mut run = runner(&state);

        run.apply(&mut state, destroy(json_as(json!({ "target": chosen() }))), Some(&victim), RunOptions::default());

        assert_eq!(live(&state, &victim.id).marked_destroyed, Some(true));
        assert_eq!(id_at(&state, slot(PlayerId::P1, Row::Units, 2)), Some(victim.id.clone()));
        assert_eq!(run.events, Vec::<GameEvent>::new());

        run.check(&mut state);

        assert_eq!(id_at(&state, slot(PlayerId::P1, Row::Units, 2)), None);
        assert_eq!(ids(&state.players[PlayerId::P1].graveyard), vec![victim.id.clone()]);
        assert_eq!(pluck(&events_of_type(&run.events, GameEventType::Destroyed), "instanceId"), vec![json!(victim.id)]);
        assert_eq!(
            pluck(&events_of_type(&run.events, GameEventType::EnteredGraveyard), "instanceId"),
            vec![json!(victim.id)]
        );
    }

    /// TS: "§4.5 two cards marked by one effect die in the same state check (R59)".
    #[test]
    fn r59_two_cards_marked_by_one_effect_die_in_the_same_state_check() {
        let mut state = default_game();
        let first = put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 1), json!({}));
        let second = put(&mut state, "fx-2", slot(PlayerId::P2, Row::Units, 1), json!({}));
        let mut run = runner(&state);

        run.apply(&mut state, destroy(json_as(json!({ "target": chosen() }))), Some(&first), RunOptions::default());
        run.apply(&mut state, destroy(json_as(json!({ "target": chosen() }))), Some(&second), RunOptions::default());
        run.check(&mut state);

        assert_eq!(
            pluck(&events_of_type(&run.events, GameEventType::Destroyed), "instanceId"),
            vec![json!(first.id), json!(second.id)]
        );
        assert_eq!(state.players[PlayerId::P1].graveyard.len(), 1);
        assert_eq!(state.players[PlayerId::P2].graveyard.len(), 1);
    }

    #[test]
    fn r46_an_indestructible_unit_ignores_a_destroy_mark_and_stays_on_the_field() {
        let mut state = default_game();
        let warded = put(&mut state, &indestructible.id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        live_mut(&mut state, &warded.id).position = Some(Position::Def);
        let mut run = runner(&state);

        run.apply(&mut state, destroy(json_as(json!({ "target": chosen() }))), Some(&warded), RunOptions::default());
        run.check(&mut state);

        assert_eq!(id_at(&state, slot(PlayerId::P1, Row::Units, 1)), Some(warded.id.clone()));
        assert_eq!(live(&state, &warded.id).marked_destroyed, Some(false));
        assert_eq!(live(&state, &warded.id).position, Some(Position::Atk));
        assert_eq!(state.players[PlayerId::P1].graveyard.len(), 0);
    }

    /// TS: "§4.5 marks a backrow card, which the state check collects too".
    #[test]
    fn marks_a_backrow_card_which_the_state_check_collects_too() {
        let mut state = default_game();
        let card = put(&mut state, &field_spell().id, slot(PlayerId::P1, Row::Backrow, 3), json!({}));
        let mut run = runner(&state);

        run.apply(&mut state, destroy(json_as(json!({ "target": chosen() }))), Some(&card), RunOptions::default());
        run.check(&mut state);

        assert_eq!(id_at(&state, slot(PlayerId::P1, Row::Backrow, 3)), None);
        assert_eq!(ids(&state.players[PlayerId::P1].graveyard), vec![card.id.clone()]);
    }

    #[test]
    fn r12_a_stolen_unit_destroyed_goes_to_its_owner_s_graveyard() {
        let mut state = default_game();
        let mut theirs = new_instance(&mut state, "fx-4", PlayerId::P2, Zone::Hand { player: PlayerId::P2 });
        assert!(place_on_field(&mut state, &mut theirs, &slot(PlayerId::P1, Row::Units, 1), Default::default()));
        let theirs = live(&state, &theirs.id).clone();
        let mut run = runner(&state);

        run.apply(&mut state, destroy(json_as(json!({ "target": chosen() }))), Some(&theirs), RunOptions::default());
        run.check(&mut state);

        assert_eq!(ids(&state.players[PlayerId::P2].graveyard), vec![theirs.id.clone()]);
        assert_eq!(state.players[PlayerId::P1].graveyard.len(), 0);
    }

    #[test]
    fn r11_a_destroyed_unit_token_vanishes_and_reaches_no_graveyard() {
        let mut state = default_game();
        let token = put(&mut state, &rush_token().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let mut run = runner(&state);

        run.apply(&mut state, destroy(json_as(json!({ "target": chosen() }))), Some(&token), RunOptions::default());
        run.check(&mut state);

        assert_eq!(id_at(&state, slot(PlayerId::P1, Row::Units, 1)), None);
        assert_eq!(state.players[PlayerId::P1].graveyard.len(), 0);
        assert!(events_of_type(&run.events, GameEventType::EnteredGraveyard).is_empty());
    }

    /// TS: "§6.3 marks nothing for a card that is not on the field".
    #[test]
    fn marks_nothing_for_a_card_that_is_not_on_the_field() {
        let mut state = default_game();
        let card = new_instance(&mut state, "fx-1", PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        state.players[PlayerId::P1].hand.push(card.clone());
        let mut run = runner(&state);

        run.apply(&mut state, destroy(json_as(json!({ "target": chosen() }))), Some(&card), RunOptions::default());
        run.check(&mut state);

        assert_eq!(live(&state, &card.id).marked_destroyed, None);
        assert_eq!(ids(&state.players[PlayerId::P1].hand), vec![card.id.clone()]);
    }
}

// ---------------------------------------------------------------------------
// sacrifice
// ---------------------------------------------------------------------------

mod sacrifice_m3_t1 {
    use super::*;

    /// TS: "§6.3 moves your own unit from the field to the graveyard at once, with no state check".
    #[test]
    fn moves_your_own_unit_from_the_field_to_the_graveyard_at_once_with_no_state_check() {
        let mut state = default_game();
        let victim = put(&mut state, "fx-1", slot(PlayerId::P1, Row::Units, 2), json!({}));
        let mut run = runner(&state);

        run.apply(&mut state, sacrifice(json_as(json!({ "target": chosen() }))), Some(&victim), RunOptions::default());

        assert_eq!(id_at(&state, slot(PlayerId::P1, Row::Units, 2)), None);
        assert_eq!(ids(&state.players[PlayerId::P1].graveyard), vec![victim.id.clone()]);
        assert_eq!(pluck(&events_of_type(&run.events, GameEventType::Destroyed), "instanceId"), vec![json!(victim.id)]);
        assert_eq!(
            pluck(&events_of_type(&run.events, GameEventType::EnteredGraveyard), "instanceId"),
            vec![json!(victim.id)]
        );
    }

    /// TS: "§6.3 counts as a death: the destroyed counter rises and the Death trigger fires".
    #[test]
    fn counts_as_a_death_the_destroyed_counter_rises_and_the_death_trigger_fires() {
        let mut state = default_game();
        let victim = put(&mut state, &dier().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let mut run = runner(&state);

        run.apply(&mut state, sacrifice(json_as(json!({ "target": chosen() }))), Some(&victim), RunOptions::default());

        assert_eq!(state.counters.destroyed, 1);
        assert_eq!(state.players[PlayerId::P2].hero.health, HERO_HEALTH - 3);
        assert_eq!(pluck(&events_of_type(&run.events, GameEventType::Damage), "amount"), vec![json!(3)]);
    }

    /// TS: "§6.3 bypasses Indestructible, which a destroy mark cannot".
    #[test]
    fn bypasses_indestructible_which_a_destroy_mark_cannot() {
        let mut state = default_game();
        let warded = put(&mut state, &warded_dier().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let mut run = runner(&state);

        run.apply(&mut state, sacrifice(json_as(json!({ "target": chosen() }))), Some(&warded), RunOptions::default());

        assert_eq!(id_at(&state, slot(PlayerId::P1, Row::Units, 1)), None);
        assert_eq!(ids(&state.players[PlayerId::P1].graveyard), vec![warded.id.clone()]);
        assert_eq!(state.players[PlayerId::P2].hero.health, HERO_HEALTH - 3);
    }

    #[test]
    fn r78_the_death_hook_reads_the_card_as_it_was_just_before_it_left_the_field() {
        let mut state = default_game();
        let victim = put(&mut state, &rememberer().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        live_mut(&mut state, &victim.id).memory.insert("meal".to_string(), json!(7));
        let victim = live(&state, &victim.id).clone();
        let mut run = runner(&state);

        run.apply(&mut state, sacrifice(json_as(json!({ "target": chosen() }))), Some(&victim), RunOptions::default());

        assert_eq!(state.players[PlayerId::P2].hero.health, HERO_HEALTH - 7);
        // R78: the instance itself is wiped on the way out, so the hook read a snapshot.
        assert_eq!(live(&state, &victim.id).memory, IndexMap::<String, Value>::new());
    }

    #[test]
    fn r11_a_sacrificed_unit_token_vanishes_and_enters_no_graveyard() {
        let mut state = default_game();
        let token = put(&mut state, &rush_token().id, slot(PlayerId::P1, Row::Units, 1), json!({}));
        let mut run = runner(&state);

        run.apply(&mut state, sacrifice(json_as(json!({ "target": chosen() }))), Some(&token), RunOptions::default());

        assert_eq!(id_at(&state, slot(PlayerId::P1, Row::Units, 1)), None);
        assert_eq!(state.players[PlayerId::P1].graveyard.len(), 0);
        assert_eq!(state.players[PlayerId::P1].exile.len(), 0);
        assert_eq!(pluck(&events_of_type(&run.events, GameEventType::Destroyed), "instanceId"), vec![json!(token.id)]);
        assert!(events_of_type(&run.events, GameEventType::EnteredGraveyard).is_empty());
    }

    /// TS: "§6.3 refuses an enemy unit unless a Tribute allows it (#55)".
    #[test]
    fn refuses_an_enemy_unit_unless_a_tribute_allows_it_55() {
        let mut state = default_game();
        let theirs = put(&mut state, "fx-4", slot(PlayerId::P2, Row::Units, 1), json!({}));
        let mut run = runner(&state);

        run.apply(
            &mut state,
            sacrifice(json_as(json!({ "target": chosen() }))),
            Some(&theirs),
            RunOptions { controller: Some(PlayerId::P1), ..Default::default() },
        );

        assert_eq!(id_at(&state, slot(PlayerId::P2, Row::Units, 1)), Some(theirs.id.clone()));
        assert_eq!(run.events, Vec::<GameEvent>::new());

        let theirs = live(&state, &theirs.id).clone();
        run.apply(
            &mut state,
            sacrifice(json_as(json!({ "target": chosen(), "allowEnemy": true }))),
            Some(&theirs),
            RunOptions { controller: Some(PlayerId::P1), ..Default::default() },
        );

        assert_eq!(id_at(&state, slot(PlayerId::P2, Row::Units, 1)), None);
        assert_eq!(ids(&state.players[PlayerId::P2].graveyard), vec![theirs.id.clone()]);
    }

    #[test]
    fn r12_a_sacrificed_stolen_unit_goes_to_its_owner_s_graveyard() {
        let mut state = default_game();
        let mut theirs = new_instance(&mut state, "fx-4", PlayerId::P2, Zone::Hand { player: PlayerId::P2 });
        assert!(place_on_field(&mut state, &mut theirs, &slot(PlayerId::P1, Row::Units, 1), Default::default()));
        let theirs = live(&state, &theirs.id).clone();
        let mut run = runner(&state);

        run.apply(
            &mut state,
            sacrifice(json_as(json!({ "target": chosen() }))),
            Some(&theirs),
            RunOptions { controller: Some(PlayerId::P1), ..Default::default() },
        );

        assert_eq!(ids(&state.players[PlayerId::P2].graveyard), vec![theirs.id.clone()]);
        assert_eq!(state.players[PlayerId::P1].graveyard.len(), 0);
    }

    /// TS: "§6.3 does nothing for a card that is not on the field".
    #[test]
    fn does_nothing_for_a_card_that_is_not_on_the_field() {
        let mut state = default_game();
        let card = new_instance(&mut state, &dier().id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        state.players[PlayerId::P1].hand.push(card.clone());
        let mut run = runner(&state);

        run.apply(&mut state, sacrifice(json_as(json!({ "target": chosen() }))), Some(&card), RunOptions::default());

        assert_eq!(ids(&state.players[PlayerId::P1].hand), vec![card.id.clone()]);
        assert_eq!(state.counters.destroyed, 0);
        assert_eq!(run.events, Vec::<GameEvent>::new());
    }
}
