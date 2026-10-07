//! Transform and Vanilla (SPEC §6.3, BUILD M3-T1): replacement in place with no Cry (R1), the
//! replaced card ceasing to exist (R35), Immutable refusing both (R23), and Vanilla clearing
//! printed keywords only (§6.3, §10.4). The fixture cards these tests need are registered here, so
//! no shared fixture has to grow for them (BUILD §0).
//!
//! Port of `packages/engine/test/effects-transform.test.ts`.

use jackioh_engine::effects::{damage, transform, vanilla};
use jackioh_engine::testkit::*;
use serde::Serialize;

use super::fixtures::catalog as catalog_fx;
use super::fixtures::combat as combat_fx;
use super::fixtures::harness::{events_of_type, in_hand, new_game, put, set_library, slot};

/// A Cry that would be loud if a Transform ever fired one (R1).
fn crier() -> CardDef {
    catalog_fx::unit_def(721, json!({ "id": "tf-crier", "name": "Crier (fixture)", "attack": 2, "health": 2 }))
}

/// A Death that would be loud if the replaced card ever reached a graveyard (§6.2, R35).
fn mourner() -> CardDef {
    catalog_fx::unit_def(722, json!({ "id": "tf-mourner", "name": "Mourner (fixture)", "attack": 3, "health": 3 }))
}

/// #8 Mr. Vanilla: Immutable text (§6.1).
fn immutable() -> CardDef {
    catalog_fx::unit_def(
        723,
        json!({
            "id": "tf-immutable",
            "name": "Immutable (fixture)",
            "attack": 4,
            "health": 4,
            "keywords": [{ "kind": "Immutable" }],
        }),
    )
}

/// #41's Sheep Token: what a Transform turns a played unit into.
fn sheep() -> CardDef {
    catalog_fx::unit_def(
        724,
        json!({
            "id": "tf-sheep",
            "name": "Sheep Token (fixture)",
            "attack": 1,
            "health": 1,
            "token": true,
            "rarity": "Token",
            "tags": ["Token"],
        }),
    )
}

fn field_spell() -> CardDef {
    catalog_fx::spell_def(
        725,
        json!({ "id": "tf-field", "index": "725", "name": "Field Spell (fixture)", "type": "Field Spell" }),
    )
}

fn trap_card() -> CardDef {
    catalog_fx::spell_def(726, json!({ "id": "tf-trap", "index": "726", "name": "Trap (fixture)", "type": "Trap" }))
}

fn one_shot() -> CardDef {
    catalog_fx::spell_def(727, json!({ "id": "tf-spell", "index": "727", "name": "Spell (fixture)" }))
}

fn scripts() -> IndexMap<String, CardScripts> {
    let crier_cry = || Script {
        cry: Some(hook(|_ctx| vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 5 })))])),
        ..Script::default()
    };
    let mourner_death = || Script {
        death: Some(hook(|_ctx| vec![damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": 7 })))])),
        ..Script::default()
    };
    let mut scripts = IndexMap::new();
    scripts.insert(
        crier().id,
        CardScripts {
            base: crier_cry(),
            radiant: crier_cry(),
        },
    );
    scripts.insert(
        mourner().id,
        CardScripts {
            base: mourner_death(),
            radiant: mourner_death(),
        },
    );
    scripts
}

fn game() -> GameState {
    let mut state = new_game("transform-test", None);
    let mut catalog = registered_catalog().clone();
    for def in [crier(), mourner(), immutable(), sheep(), field_spell(), trap_card(), one_shot()] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    registry.extend(scripts());
    register_scripts(registry);
    state.turn = 4;
    state
}

/// Apply one effect the way `resolve.ts` does, and hand back the events it emitted. TS's
/// `sinkFor(state)` is the sink built here: its rng starts at the state's cursor, as reduce does.
fn run(state: &mut GameState, effect: Effect, options: HookOptions) -> Vec<GameEvent> {
    let mut events = Vec::new();
    let mut rng = Rng::new(&state.seed, state.rng_cursor);
    {
        let mut sink = EngineSink::new(state, &mut events, &mut rng);
        let mut ctx = make_context(&mut sink, None, options);
        (effect.apply)(&mut ctx);
    }
    state.rng_cursor = rng.cursor();
    events
}

/// TS `{ controller }`.
fn by(player: PlayerId) -> HookOptions {
    HookOptions {
        controller: Some(player),
        ..HookOptions::default()
    }
}

/// `transform({ instanceId, defId })`.
fn transform_into(instance_id: &str, def_id: &str) -> Effect {
    transform(json_as(json!({ "instanceId": instance_id, "defId": def_id })))
}

/// TS `string[]` for the harness's `setLibrary`, from the ids as written.
fn owned(ids: &[&str]) -> Vec<String> {
    ids.iter().map(|id| id.to_string()).collect()
}

/// TS held the live instance and read it after an effect; Rust reads the card again by id.
fn live(state: &GameState, id: &str) -> CardInstance {
    find_instance(state, id).cloned().unwrap_or_else(|| panic!("no card {id} in the state"))
}

/// TS wrote through the live instance; Rust writes through the card found by id.
fn live_mut<'a>(state: &'a mut GameState, id: &str) -> &'a mut CardInstance {
    find_instance_mut(state, id).unwrap_or_else(|| panic!("no card {id} in the state"))
}

fn at(state: &GameState, player: PlayerId, row: Row, lane: i32) -> Option<CardInstance> {
    card_at(state, slot(player, row, lane)).cloned()
}

fn def_ids_of(cards: &[CardInstance]) -> Vec<String> {
    cards.iter().map(|card| card.def_id.clone()).collect()
}

fn to_json<T: Serialize>(value: T) -> Value {
    serde_json::to_value(value).expect("serialises")
}

mod transform_s6_3_r23_r35_m3_t1 {
    use super::*;

    #[test]
    fn replaces_a_card_in_place_with_a_new_instance_of_the_new_definition_and_fires_no_cry_r1() {
        let mut state = game();
        let old = put(&mut state, &combat_fx::plain().id, slot(PlayerId::P1, Row::Units, 3));
        {
            let card = live_mut(&mut state, &old.id);
            card.position = Some(Position::Def);
            card.damage = 2;
        }

        run(&mut state, transform_into(&old.id, &crier().id), by(PlayerId::P1));

        let now = at(&state, PlayerId::P1, Row::Units, 3);
        assert!(now.is_some());
        let now = now.expect("the replacement");
        assert_ne!(now.id, old.id);
        assert_eq!(now.def_id, crier().id);
        assert_eq!(now.owner, PlayerId::P1);
        assert_eq!(now.controller, PlayerId::P1);
        assert_eq!(now.position, Some(Position::Def));
        assert_eq!(
            now.zone,
            Zone::Field {
                player: PlayerId::P1,
                row: Row::Units,
                lane: 3,
            }
        );
        // A fresh instance: the old card's damage does not come with it, and the new body is sick.
        assert_eq!(now.damage, 0);
        assert!(!now.radiant);
        assert!(is_sick(&state, &now));
        assert_eq!(unit_view(&state, &now).attack, 2);

        // R1: a Transform result never fires a Cry, so the enemy hero is untouched.
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);
    }

    #[test]
    fn r691_a_transformed_unit_keeps_its_battle_position_def_stays_def() {
        let mut state = game();
        let old = put(&mut state, &combat_fx::plain().id, slot(PlayerId::P1, Row::Units, 3));
        live_mut(&mut state, &old.id).position = Some(Position::Def);

        run(&mut state, transform_into(&old.id, &crier().id), by(PlayerId::P1));

        let now = at(&state, PlayerId::P1, Row::Units, 3);
        assert_ne!(now.as_ref().map(|card| card.id.clone()), Some(old.id.clone()));
        assert_eq!(now.as_ref().and_then(|card| card.position), Some(Position::Def));
    }

    #[test]
    fn reports_the_transform_with_a_transformed_event() {
        let mut state = game();
        let old = put(&mut state, &combat_fx::plain().id, slot(PlayerId::P1, Row::Units, 3));
        {
            let card = live_mut(&mut state, &old.id);
            card.position = Some(Position::Def);
            card.damage = 2;
        }

        let events = run(&mut state, transform_into(&old.id, &crier().id), by(PlayerId::P1));
        let now = at(&state, PlayerId::P1, Row::Units, 3);
        assert_ne!(now.as_ref().map(|card| card.id.clone()), Some(old.id.clone()));
        assert_eq!(
            to_json(&events),
            json!([
                {
                    "type": "transformed",
                    "instanceId": old.id,
                    "fromDefId": combat_fx::plain().id,
                    "toDefId": crier().id,
                    "newInstanceId": now.map(|card| card.id),
                },
            ])
        );
    }

    #[test]
    fn r659_a_unit_a_transform_puts_on_the_field_is_summoning_sick_this_turn_however_ready_the_old_one_was() {
        let mut state = game();
        state.active = PlayerId::P1;
        let turn = state.turn;
        // A unit that has been on the field since an earlier turn, with its exertions unspent: ready.
        let old = put(&mut state, &combat_fx::plain().id, slot(PlayerId::P1, Row::Units, 2));
        live_mut(&mut state, &old.id).summoned_turn = Some(turn - 2);
        assert!(!is_sick(&state, &live(&state, &old.id)));

        run(&mut state, transform_into(&old.id, &crier().id), by(PlayerId::P1));

        let now = at(&state, PlayerId::P1, Row::Units, 2).expect("the replacement");
        assert_eq!(now.summoned_turn, Some(state.turn));
        assert!(is_sick(&state, &now));

        // The opponent's own unit, transformed on the opponent's turn, is sick for that turn too.
        state.active = PlayerId::P2;
        let theirs = put(&mut state, &combat_fx::plain().id, slot(PlayerId::P2, Row::Units, 1));
        live_mut(&mut state, &theirs.id).summoned_turn = Some(turn - 2);
        run(&mut state, transform_into(&theirs.id, &sheep().id), by(PlayerId::P2));
        assert!(is_sick(&state, &at(&state, PlayerId::P2, Row::Units, 1).expect("the sheep")));
    }

    #[test]
    fn r35_the_replaced_card_ceases_to_exist_no_graveyard_no_exile_and_no_death() {
        let mut state = game();
        let old = put(&mut state, &mourner().id, slot(PlayerId::P1, Row::Units, 1));

        let events = run(&mut state, transform_into(&old.id, &sheep().id), by(PlayerId::P1));

        assert!(find_instance(&state, &old.id).is_none());
        assert!(state.players.p1.graveyard.is_empty());
        assert!(state.players.p1.exile.is_empty());
        assert_eq!(state.counters.destroyed, 0);
        // §6.2: Death does not fire on a transform, so the enemy hero takes nothing.
        assert_eq!(state.players.p2.hero.health, HERO_HEALTH);
        assert!(events_of_type(&events, GameEventType::Destroyed).is_empty());
        assert!(events_of_type(&events, GameEventType::EnteredGraveyard).is_empty());
        assert_eq!(at(&state, PlayerId::P1, Row::Units, 1).map(|card| card.def_id), Some(sheep().id));
    }

    #[test]
    fn r23_refuses_a_transform_on_an_immutable_card_printed_or_granted() {
        let mut state = game();
        let printed = put(&mut state, &immutable().id, slot(PlayerId::P1, Row::Units, 1));
        let granted = put(&mut state, &combat_fx::plain().id, slot(PlayerId::P1, Row::Units, 2));
        live_mut(&mut state, &granted.id).granted_keywords.push(Keyword::Immutable);

        assert!(run(&mut state, transform_into(&printed.id, &sheep().id), by(PlayerId::P1)).is_empty());
        assert!(run(&mut state, transform_into(&granted.id, &sheep().id), by(PlayerId::P1)).is_empty());

        assert_eq!(at(&state, PlayerId::P1, Row::Units, 1).map(|card| card.id), Some(printed.id.clone()));
        assert_eq!(at(&state, PlayerId::P1, Row::Units, 2).map(|card| card.id), Some(granted.id.clone()));
        assert_eq!(live(&state, &printed.id).def_id, immutable().id);
        assert_eq!(live(&state, &granted.id).def_id, combat_fx::plain().id);

        // A unit with no Immutable in the same spot is replaced, so the refusal is the keyword's work.
        let open = put(&mut state, &combat_fx::plain().id, slot(PlayerId::P1, Row::Units, 3));
        assert_eq!(run(&mut state, transform_into(&open.id, &sheep().id), by(PlayerId::P1)).len(), 1);
        assert_eq!(at(&state, PlayerId::P1, Row::Units, 3).map(|card| card.def_id), Some(sheep().id));
    }

    #[test]
    fn s3_2_replaces_a_backrow_card_in_its_lane_a_field_spell_face_up_and_a_trap_face_down_r33() {
        let mut state = game();
        let hidden = put(&mut state, &trap_card().id, slot(PlayerId::P2, Row::Backrow, 2));

        run(&mut state, transform_into(&hidden.id, &field_spell().id), by(PlayerId::P2));

        let public = at(&state, PlayerId::P2, Row::Backrow, 2);
        assert_eq!(public.as_ref().map(|card| card.def_id.clone()), Some(field_spell().id));
        assert_eq!(public.as_ref().and_then(|card| card.face_up), Some(true));

        // The other way round: a Trap replacement is hidden again until it fires.
        let spell = put(&mut state, &field_spell().id, slot(PlayerId::P1, Row::Backrow, 5));
        live_mut(&mut state, &spell.id).face_up = Some(true);
        run(&mut state, transform_into(&spell.id, &trap_card().id), by(PlayerId::P1));

        let back = at(&state, PlayerId::P1, Row::Backrow, 5);
        assert_eq!(back.as_ref().map(|card| card.def_id.clone()), Some(trap_card().id));
        assert_eq!(back.as_ref().and_then(|card| card.face_up), None);
    }

    #[test]
    fn r13_keeps_a_stack_pile_the_replacement_is_the_top_and_the_dormant_card_stays_beneath() {
        let mut state = game();
        let beneath = put(&mut state, &combat_fx::plain().id, slot(PlayerId::P1, Row::Units, 2));
        let mut top = new_instance(&mut state, &combat_fx::stacker().id, PlayerId::P1, Zone::Hand { player: PlayerId::P1 });
        assert!(place_on_field(
            &mut state,
            &mut top,
            slot(PlayerId::P1, Row::Units, 2),
            PlaceOnFieldOptions { stack: Some(true) },
        ));

        run(&mut state, transform_into(&top.id, &sheep().id), by(PlayerId::P1));

        let pile = state.players.p1.units[1].as_ref().map(|pile| def_ids_of(pile));
        assert_eq!(pile, Some(vec![sheep().id, combat_fx::plain().id]));
        assert_eq!(
            active_units_of(&state, PlayerId::P1).iter().map(|card| card.def_id.clone()).collect::<Vec<_>>(),
            vec![sheep().id]
        );
        assert!(find_instance(&state, &top.id).is_none());
        assert_eq!(find_instance(&state, &beneath.id).map(|card| card.id.clone()), Some(beneath.id.clone()));
    }

    #[test]
    fn r35_replaces_a_hand_card_and_keeps_a_library_card_at_its_index() {
        let mut state = game();
        let hand = in_hand(&mut state, &combat_fx::plain().id, PlayerId::P1, 3);

        run(&mut state, transform_into(&hand[1].id, &sheep().id), by(PlayerId::P1));

        // "Same counts" per zone (R35): one card out, one card in.
        assert_eq!(state.players.p1.hand.len(), 3);
        assert!(find_instance(&state, &hand[1].id).is_none());
        assert_eq!(state.players.p1.hand.iter().filter(|card| card.def_id == sheep().id).count(), 1);
        assert_eq!(
            state.players.p1.hand.iter().filter(|card| card.def_id == combat_fx::plain().id).count(),
            2
        );

        let plain_id = combat_fx::plain().id;
        let mourner_id = mourner().id;
        let crier_id = crier().id;
        let library = set_library(&mut state, PlayerId::P2, &owned(&[plain_id.as_str(), mourner_id.as_str(), crier_id.as_str()]));
        run(&mut state, transform_into(&library[1].id, &sheep().id), by(PlayerId::P2));

        // A library is ordered top to bottom, so the replacement takes the replaced card's place.
        assert_eq!(def_ids_of(&state.players.p2.library), vec![plain_id, sheep().id, crier_id]);
        assert_eq!(
            state.players.p2.library.get(1).map(|card| card.zone.clone()),
            Some(Zone::Library { player: PlayerId::P2 })
        );
    }

    #[test]
    fn s5_1_refuses_a_definition_that_cannot_live_in_the_zone_the_old_card_occupies() {
        let mut state = game();
        let unit = put(&mut state, &combat_fx::plain().id, slot(PlayerId::P1, Row::Units, 1));

        // A Spell is never a permanent, and a Trap belongs to the backrow, not a unit zone.
        assert!(run(&mut state, transform_into(&unit.id, &one_shot().id), by(PlayerId::P1)).is_empty());
        assert!(run(&mut state, transform_into(&unit.id, &trap_card().id), by(PlayerId::P1)).is_empty());
        assert_eq!(at(&state, PlayerId::P1, Row::Units, 1).map(|card| card.id), Some(unit.id.clone()));
        assert_eq!(live(&state, &unit.id).def_id, combat_fx::plain().id);
    }

    #[test]
    fn transforms_nothing_when_no_target_was_picked() {
        let mut state = game();
        let unit = put(&mut state, &combat_fx::plain().id, slot(PlayerId::P1, Row::Units, 1));

        assert!(run(&mut state, transform(json_as(json!({ "defId": sheep().id }))), by(PlayerId::P1)).is_empty());
        assert_eq!(at(&state, PlayerId::P1, Row::Units, 1).map(|card| card.id), Some(unit.id.clone()));
    }

    #[test]
    fn makes_a_radiant_replacement_when_the_effect_asks_for_one_s5_2() {
        let mut state = game();
        let old = put(&mut state, &combat_fx::plain().id, slot(PlayerId::P1, Row::Units, 1));

        run(
            &mut state,
            transform(json_as(json!({ "instanceId": old.id, "defId": crier().id, "radiant": true }))),
            by(PlayerId::P1),
        );

        let now = at(&state, PlayerId::P1, Row::Units, 1).expect("the replacement");
        assert!(now.radiant);
        // The radiant face of the fixture doubles the printed stats.
        assert_eq!(unit_view(&state, &now).attack, 4);
        assert_eq!(unit_view(&state, &now).max_health, 4);
    }
}

mod vanilla_s6_3_r23_m3_t1 {
    use super::*;

    #[test]
    fn clears_printed_keywords_while_stats_buffs_damage_and_granted_keywords_stay() {
        let mut state = game();
        // The fixture Taunt unit is a 2/5 with Taunt.
        let unit = put(&mut state, &combat_fx::taunter().id, slot(PlayerId::P1, Row::Units, 1));
        {
            let card = live_mut(&mut state, &unit.id);
            card.buffs = AttackHealth { attack: 1, health: 1 };
            card.damage = 2;
            card.granted_keywords.push(Keyword::Rush);
        }
        assert!(unit_has(&state, &live(&state, &unit.id), KeywordKind::Taunt));

        let events = run(&mut state, vanilla(json_as(json!({ "instanceId": unit.id }))), by(PlayerId::P1));

        let now = live(&state, &unit.id);
        assert!(now.vanilla);
        assert!(!unit_has(&state, &now, KeywordKind::Taunt));
        // A granted keyword is another layer and is not printed text, so it survives (§10.4).
        assert!(unit_has(&state, &now, KeywordKind::Rush));
        let view = unit_view(&state, &now);
        assert_eq!(view.attack, 3);
        assert_eq!(view.max_health, 6);
        assert_eq!(view.health, 4);
        assert_eq!(now.def_id, combat_fx::taunter().id);

        // No instance is created and no definition changes, so both sides name the same card.
        assert_eq!(
            to_json(&events),
            json!([
                {
                    "type": "transformed",
                    "instanceId": unit.id,
                    "fromDefId": combat_fx::taunter().id,
                    "toDefId": combat_fx::taunter().id,
                    "newInstanceId": unit.id,
                },
            ])
        );
    }

    #[test]
    fn r23_refuses_a_vanilla_on_an_immutable_card_printed_or_granted() {
        let mut state = game();
        let printed = put(&mut state, &immutable().id, slot(PlayerId::P1, Row::Units, 1));
        let granted = put(&mut state, &combat_fx::taunter().id, slot(PlayerId::P1, Row::Units, 2));
        live_mut(&mut state, &granted.id).granted_keywords.push(Keyword::Immutable);

        assert!(run(&mut state, vanilla(json_as(json!({ "instanceId": printed.id }))), by(PlayerId::P1)).is_empty());
        assert!(run(&mut state, vanilla(json_as(json!({ "instanceId": granted.id }))), by(PlayerId::P1)).is_empty());

        let printed = live(&state, &printed.id);
        let granted = live(&state, &granted.id);
        assert!(!printed.vanilla);
        assert!(!granted.vanilla);
        assert!(unit_has(&state, &printed, KeywordKind::Immutable));
        assert!(unit_has(&state, &granted, KeywordKind::Taunt));
    }

    #[test]
    fn does_nothing_to_a_card_that_is_already_vanilla_and_nothing_with_no_target_picked() {
        let mut state = game();
        let unit = put(&mut state, &combat_fx::taunter().id, slot(PlayerId::P1, Row::Units, 1));

        assert_eq!(run(&mut state, vanilla(json_as(json!({ "instanceId": unit.id }))), by(PlayerId::P1)).len(), 1);
        assert!(run(&mut state, vanilla(json_as(json!({ "instanceId": unit.id }))), by(PlayerId::P1)).is_empty());
        assert!(live(&state, &unit.id).vanilla);

        assert!(run(&mut state, vanilla(json_as(json!({}))), by(PlayerId::P1)).is_empty());
    }

    #[test]
    fn clears_the_text_of_a_card_in_hand_too_so_it_is_not_a_field_only_flag_s6_3() {
        let mut state = game();
        let held = in_hand(&mut state, &combat_fx::taunter().id, PlayerId::P1, 1).remove(0);

        assert_eq!(run(&mut state, vanilla(json_as(json!({ "instanceId": held.id }))), by(PlayerId::P1)).len(), 1);
        let held = live(&state, &held.id);
        assert!(held.vanilla);
        assert!(!unit_has(&state, &held, KeywordKind::Taunt));
        assert_eq!(unit_view(&state, &held).attack, 2);
    }
}
