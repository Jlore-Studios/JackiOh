// Two card-specific additions of Classic #88 Siphon Squad (R403), proved through fixture scripts:
//   - §10.4 layer 5's "an aura that sets attack to a value applies after every other layer"
//     (`StatMod.setAttack`, the Radiant face's "Enemy Units have 0 Attack");
//   - "When …, Tribute this", a condition every state check reads, the one right after the card arrives
//     included (`Script.tributeWhen`, read in `stateCheck.ts`), which reaches a face-down card too.
//
// Port of `packages/engine/test/self-tribute.test.ts`.

use jackioh_engine::subsystems::fuse::{FuseArgs, fuse};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::catalog::unit_def;
use crate::rules::fixtures::harness::{events_of_type, in_hand, new_game, put, slot};

fn def(id: &str, type_: &str, index: u32) -> CardDef {
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": id,
        "set": "Core",
        "type": type_,
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": 2,
        "base": { "keywords": [], "text": id },
        "radiant": { "keywords": [], "text": id },
    }))
}

/// Radiant Siphon Squad's shape: enemy Units have 0 Attack; Tribute this when they have none.
fn zeroer() -> CardDef {
    def("st-zeroer", "Field Trap", 9601)
}
/// A +3 attack aura on its controller's Units, to show the set beats every other layer.
fn booster() -> CardDef {
    def("st-booster", "Field Spell", 9602)
}
fn body() -> CardDef {
    unit_def(9603, json!({ "attack": 4, "health": 4 }))
}
/// A Field Trap with no text, to fuse onto the zeroer (R77: a fusion keeps every ingredient's text).
fn blank() -> CardDef {
    def("st-blank", "Field Trap", 9604)
}

fn scripts() -> Vec<(String, CardScripts)> {
    vec![
        (
            zeroer().id,
            CardScripts {
                base: Script {
                    aura: Some(aura_hook(|args| {
                        let me = args.self_.controller;
                        vec![AuraEntry {
                            applies: Box::new(move |unit: &CardInstance| unit.controller != me),
                            mod_: StatMod {
                                set_attack: Some(0),
                                ..StatMod::default()
                            },
                        }]
                    })),
                    tribute_when: Some(read_hook(|args| {
                        active_units_of(args.state, opponent_of(args.self_.controller)).is_empty()
                    })),
                    ..Script::default()
                },
                radiant: Script::default(),
            },
        ),
        (
            booster().id,
            CardScripts {
                base: Script {
                    aura: Some(aura_hook(|args| {
                        let me = args.self_.controller;
                        vec![AuraEntry {
                            applies: Box::new(move |unit: &CardInstance| unit.controller == me),
                            mod_: StatMod {
                                attack: Some(3),
                                ..StatMod::default()
                            },
                        }]
                    })),
                    ..Script::default()
                },
                radiant: Script::default(),
            },
        ),
    ]
}

fn game(seed: &str) -> GameState {
    let mut state = new_game(&format!("self-tribute-{seed}"), None);
    let mut catalog = registered_catalog().clone();
    for def in [zeroer(), booster(), body(), blank()] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut registry = registered_scripts().clone();
    for (id, script) in scripts() {
        registry.insert(id, script);
    }
    register_scripts(registry);
    state.phase = Phase::Main;
    state
}

/// TS `sinkFor(state)`'s three parts side by side, so the state stays readable between engine calls
/// (TS read the same object through `state` and `sink.state`).
struct Bench {
    state: GameState,
    events: Vec<GameEvent>,
    rng: Rng,
}

impl Bench {
    fn sink_for(state: GameState) -> Bench {
        let rng = Rng::new(&state.seed, state.rng_cursor);
        Bench {
            state,
            events: Vec::new(),
            rng,
        }
    }

    fn sink(&mut self) -> EngineSink<'_> {
        EngineSink::new(&mut self.state, &mut self.events, &mut self.rng)
    }
}

/// The card as it stands in the state now (TS held the live object).
fn live<'a>(state: &'a GameState, card: &CardInstance) -> &'a CardInstance {
    find_instance(state, &card.id).expect("the card is still in the game")
}

fn live_mut<'a>(state: &'a mut GameState, card: &CardInstance) -> &'a mut CardInstance {
    find_instance_mut(state, &card.id).expect("the card is still in the game")
}

fn zone_of(state: &GameState, card: &CardInstance) -> ZoneName {
    live(state, card).zone.z()
}

mod r403_section_10_4_an_aura_that_sets_attack_applies_after_every_other_layer_c88 {
    use super::*;

    #[test]
    fn r403_a_buff_and_a_3_aura_do_not_lift_it_the_unit_reads_0_raw_and_floored() {
        let mut state = game("set");
        put(
            &mut state,
            &zeroer().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        put(
            &mut state,
            &booster().id,
            slot(PlayerId::P2, Row::Backrow, 1),
            Default::default(),
        );
        let enemy = put(
            &mut state,
            &body().id,
            slot(PlayerId::P2, Row::Units, 1),
            Default::default(),
        );
        live_mut(&mut state, &enemy).buffs.attack += 5;

        assert_eq!(unit_view(&state, live(&state, &enemy)).attack, 0);
        assert_eq!(unclamped_attack(&state, live(&state, &enemy)), 0);
        // Its own side is untouched by it.
        let mine = put(
            &mut state,
            &body().id,
            slot(PlayerId::P1, Row::Units, 1),
            Default::default(),
        );
        assert_eq!(unit_view(&state, live(&state, &mine)).attack, 4);
    }
}

mod r403_when_tribute_this_is_read_at_every_state_check_c88 {
    use super::*;

    #[test]
    fn r403_while_the_condition_holds_the_card_is_sacrificed_face_down_at_the_next_check() {
        let mut state = game("tribute");
        let trap = put(
            &mut state,
            &zeroer().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        live_mut(&mut state, &trap).face_up = Some(false);
        let mut b = Bench::sink_for(state);

        state_check(&mut b.sink());

        assert_eq!(zone_of(&b.state, &trap), ZoneName::Graveyard);
        let destroyed: Vec<Value> = events_of_type(&b.events, GameEventType::Destroyed)
            .into_iter()
            .map(|event| serde_json::to_value(event).unwrap()["instanceId"].clone())
            .collect();
        assert_eq!(destroyed, vec![json!(trap.id)]);
    }

    #[test]
    fn r403_while_the_opponent_has_a_unit_it_stays_once_it_has_none_the_next_check_takes_it() {
        let mut state = game("stays");
        let trap = put(
            &mut state,
            &zeroer().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        let enemy = put(
            &mut state,
            &body().id,
            slot(PlayerId::P2, Row::Units, 1),
            Default::default(),
        );
        let mut b = Bench::sink_for(state);

        state_check(&mut b.sink());
        assert_eq!(zone_of(&b.state, &trap), ZoneName::Field);

        live_mut(&mut b.state, &enemy).marked_destroyed = Some(true);
        state_check(&mut b.sink());

        assert_eq!(zone_of(&b.state, &enemy), ZoneName::Graveyard);
        assert_eq!(zone_of(&b.state, &trap), ZoneName::Graveyard);
    }
}

mod r77_r102_r403_a_fusion_keeps_both_additions_of_its_ingredients {
    use super::*;

    #[test]
    fn r403_a_field_trap_fused_onto_the_zeroer_still_zeroes_enemy_attack_and_is_tributed_when_they_have_no_unit()
     {
        let mut state = game("fused");
        let target = put(
            &mut state,
            &zeroer().id,
            slot(PlayerId::P1, Row::Backrow, 1),
            Default::default(),
        );
        let enemy = put(
            &mut state,
            &body().id,
            slot(PlayerId::P2, Row::Units, 1),
            Default::default(),
        );
        let mut b = Bench::sink_for(state);
        let ingredient = in_hand(&mut b.state, &blank().id, PlayerId::P1, 1)
            .into_iter()
            .next()
            .expect("no ingredient");

        let on = live(&b.state, &target).clone();
        let fused = fuse(
            &mut b.sink(),
            FuseArgs {
                ingredients: vec![ingredient],
                target: Some(on),
                ..Default::default()
            },
        );

        assert_eq!(fused.map(|card| card.id), Some(target.id.clone()));
        assert_eq!(unit_view(&b.state, live(&b.state, &enemy)).attack, 0);
        state_check(&mut b.sink());
        assert_eq!(zone_of(&b.state, &target), ZoneName::Field);

        live_mut(&mut b.state, &enemy).marked_destroyed = Some(true);
        state_check(&mut b.sink());
        assert_eq!(zone_of(&b.state, &target), ZoneName::Graveyard);
    }
}
