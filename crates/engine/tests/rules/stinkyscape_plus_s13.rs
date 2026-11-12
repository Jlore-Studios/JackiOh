// The verbs Stinkyscape+ S13 (issue #3560) added, each through a fixture script before a card uses
// it: once-a-game marks per player (R3560000), a card played from its own graveyard at a price
// (R3560001), two board reads (R3560005, R3560006) and Ignores Taunt (R3560007). The cards' own
// tests play them in the real scripts; these pin the verbs themselves. Ids are `s13-`.

use jackioh_engine::effects;
use jackioh_engine::testkit::PlayerId::{P1, P2};
use jackioh_engine::testkit::*;

use crate::rules::fixtures::harness::{apply_now, put, run_effect, setup_catalog, slot};

const CRAWLERISH: &str = "s13-crawlerish";
const TAUNT: &str = "s13-taunt";
const RUNNER: &str = "s13-runner";
const PLAIN: &str = "s13-plain";

fn unit_def(index: u32, id: &str, attack: i32, health: i32, cost: i32, keywords: Value) -> CardDef {
    json_as(json!({
        "id": id,
        "index": index.to_string(),
        "name": format!("{id} (S13 fixture)"),
        "set": "Core",
        "type": "Unit",
        "tags": [],
        "rarity": "Common",
        "token": false,
        "cost": cost,
        "base": { "attack": attack, "health": health, "keywords": keywords, "text": id },
        "radiant": { "attack": attack, "health": health, "keywords": keywords, "text": id },
    }))
}

fn flags_script(flags: Option<StaticFlags>) -> CardScripts {
    let script = Script {
        static_flags: flags,
        ..Script::default()
    };
    CardScripts {
        base: script.clone(),
        radiant: script,
    }
}

fn register() {
    setup_catalog();
    let mut catalog: CardDefs = registered_catalog().clone();
    for def in [
        unit_def(35600, CRAWLERISH, 1, 3, 0, json!([])),
        unit_def(35601, TAUNT, 1, 5, 1, json!([{ "kind": "Taunt" }])),
        unit_def(35602, RUNNER, 3, 3, 1, json!([{ "kind": "Rush" }])),
        unit_def(35603, PLAIN, 1, 3, 1, json!([])),
    ] {
        catalog.insert(def.id.clone(), def);
    }
    register_catalog(catalog);
    let mut scripts: IndexMap<String, CardScripts> = registered_scripts().clone();
    scripts.insert(
        CRAWLERISH.to_string(),
        flags_script(Some(StaticFlags {
            graveyard_price: Some(1),
            ..StaticFlags::default()
        })),
    );
    scripts.insert(TAUNT.to_string(), flags_script(None));
    scripts.insert(
        RUNNER.to_string(),
        flags_script(Some(StaticFlags {
            ignores_taunt: Some(true),
            ..StaticFlags::default()
        })),
    );
    scripts.insert(PLAIN.to_string(), flags_script(None));
    register_scripts(scripts);
}

fn board(p1: Value, p2: Value) -> Scenario {
    register();
    let mut s = scenario(json!({ "seed": "s13", "p1": p1, "p2": p2 }));
    for player in [P1, P2] {
        s.state_mut().players[player].auto_end_turn = Some(false);
    }
    s
}

fn plays_of(s: &Scenario, id: &str) -> Vec<ActionBody> {
    legal_actions(s.state(), P1)
        .into_iter()
        .filter(|action| matches!(action, ActionBody::Play { instance_id, .. } if instance_id == id))
        .collect()
}

mod r3560000_once_a_game_marks_are_per_player {
    use super::*;

    #[test]
    fn r3560000_the_mark_is_set_and_not_doubled_and_the_other_player_stays_unset() {
        let mut s = board(json!({}), json!({}));
        assert!(!once_a_game_used(s.state(), P1, "s13:key"));
        run_effect(
            s.state_mut(),
            effects::mark_once_a_game(json_as(json!({ "key": "s13:key" }))),
            P1,
        );
        assert!(once_a_game_used(s.state(), P1, "s13:key"));
        assert!(!once_a_game_used(s.state(), P2, "s13:key"));
        let before = s.state().players[P1].once_a_game.clone();
        run_effect(
            s.state_mut(),
            effects::mark_once_a_game(json_as(json!({ "key": "s13:key" }))),
            P1,
        );
        assert_eq!(s.state().players[P1].once_a_game, before);
        run_effect(
            s.state_mut(),
            effects::mark_once_a_game(json_as(json!({ "key": "s13:other" }))),
            P2,
        );
        assert!(once_a_game_used(s.state(), P2, "s13:other"));
        assert!(!once_a_game_used(s.state(), P1, "s13:other"));
    }
}

mod r3560001_graveyard_price_offers_and_prices_the_play {
    use super::*;

    fn graveyard_board(mana: i32) -> Scenario {
        board(
            json!({ "mana": mana, "graveyard": [CRAWLERISH, PLAIN], "library": [PLAIN, PLAIN, PLAIN] }),
            json!({}),
        )
    }

    fn grave_id(s: &Scenario, def: &str) -> String {
        s.pile(P1, "graveyard")
            .into_iter()
            .find(|card| card.def_id == def)
            .expect("setup")
            .id
    }

    #[test]
    fn r3560001_with_mana_1_it_is_offered_and_costs_1() {
        let s = graveyard_board(1);
        let id = grave_id(&s, CRAWLERISH);
        assert!(!plays_of(&s, &id).is_empty());
        let card = find_instance(s.state(), &id).expect("held");
        assert_eq!(printed_cost(s.state(), card), 1);
        assert_eq!(play_cost(s.state(), card), 1);
    }

    #[test]
    fn r3560001_with_mana_0_it_is_refused() {
        let s = graveyard_board(0);
        let id = grave_id(&s, CRAWLERISH);
        assert!(plays_of(&s, &id).is_empty());
    }

    #[test]
    fn r3560001_it_is_offered_again_after_it_dies_and_a_plain_card_is_never_offered() {
        let s = graveyard_board(3);
        let id = grave_id(&s, CRAWLERISH);
        let plain = grave_id(&s, PLAIN);
        assert!(plays_of(&s, &plain).is_empty());
        let action = json!({ "type": "play", "playerId": "p1", "instanceId": id, "zone": { "row": "units", "lane": 1 }, "nonce": "s13-grave-1" });
        let result = reduce(s.state(), &json_as::<Action>(action));
        assert!(result.error.is_none(), "{:?}", result.error);
        let after = result.state;
        let fielded = after.players[P1]
            .units
            .iter()
            .flatten()
            .flatten()
            .find(|card| card.def_id == CRAWLERISH)
            .expect("played from the graveyard")
            .clone();
        // Back to the graveyard: the self-grant admits it again.
        let mut state = after;
        apply_now(
            &mut state,
            effects::destroy(json_as(
                json!({ "target": { "of": "instance", "instanceId": fielded.id } }),
            )),
        );
        assert!(
            state.players[P1]
                .graveyard
                .iter()
                .any(|card| card.def_id == CRAWLERISH)
        );
        let again = state.players[P1]
            .graveyard
            .iter()
            .find(|card| card.def_id == CRAWLERISH)
            .expect("back in the graveyard");
        let offered: Vec<String> = legal_actions(&state, P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Play { instance_id, .. } => Some(instance_id),
                _ => None,
            })
            .collect();
        assert!(offered.contains(&again.id));
        assert!(!offered.contains(&plain));
    }
}

mod r3560005_exiling_summon_lands_reads_void_dormant_and_halves {
    use super::*;

    #[test]
    fn r3560005_true_on_a_plain_occupied_zone_false_on_a_dormant_pile_or_a_slashed_lane() {
        let mut s = board(
            json!({ "field": [{ "def": PLAIN, "lane": 1 }], "hand": [CRAWLERISH] }),
            json!({}),
        );
        let source = s.card(CRAWLERISH).clone();
        let plain = ZoneSlot {
            player: P1,
            row: Row::Units,
            lane: 1,
        };
        assert!(exiling_summon_lands(s.state(), &source, plain));
        // A pile holding a Dormant card is left alone.
        let dormant = put(s.state_mut(), PLAIN, slot(P1, Row::Units, 2), json!({}));
        find_instance_mut(s.state_mut(), &dormant.id)
            .expect("the unit")
            .slumber = Some(Slumber { turns: Some(2) });
        assert!(!exiling_summon_lands(
            s.state(),
            &source,
            ZoneSlot {
                player: P1,
                row: Row::Units,
                lane: 2,
            }
        ));
        // A slashed lane is no landing.
        s.state_mut().slashed_lane = Some(3);
        assert!(!exiling_summon_lands(
            s.state(),
            &source,
            ZoneSlot {
                player: P1,
                row: Row::Units,
                lane: 3,
            }
        ));
    }
}

mod r3560006_def_on_field_reads_both_sides {
    use super::*;

    #[test]
    fn r3560006_it_reads_both_sides() {
        let s = board(json!({ "field": [PLAIN] }), json!({ "field": [PLAIN] }));
        assert!(def_on_field(s.state(), PLAIN));
        assert!(!def_on_field(s.state(), TAUNT));
        let s = board(json!({}), json!({ "field": [{ "def": TAUNT, "lane": 2 }] }));
        assert!(def_on_field(s.state(), TAUNT));
        assert!(!def_on_field(s.state(), PLAIN));
    }
}

mod r3560007_ignores_taunt_lets_the_attacker_past_a_taunt {
    use super::*;

    fn walled() -> Scenario {
        board(
            json!({ "field": [{ "def": RUNNER, "lane": 1 }, { "def": PLAIN, "lane": 2 }] }),
            json!({ "field": [{ "def": TAUNT, "lane": 1 }, { "def": PLAIN, "lane": 2 }] }),
        )
    }

    fn offered_targets(s: &Scenario, attacker: &str) -> Vec<String> {
        legal_actions(s.state(), P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Attack {
                    attacker_id,
                    target_id,
                } if attacker_id == attacker => Some(target_id),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn r3560007_the_runner_may_hit_the_hero_and_a_non_taunt_unit_past_a_taunt() {
        let s = walled();
        let runner = s.unit_at(P1, 1);
        let plain = s.unit_at(P1, 2);
        let taunt = s.unit_at(P2, 1);
        let enemy_plain = s.unit_at(P2, 2);
        // The runner is Rush, so its summon turn still reaches.
        assert!(can_attack(s.state(), &runner, &DamageTarget::Hero { player: P2 }));
        assert!(can_attack(
            s.state(),
            &runner,
            &DamageTarget::Unit {
                instance: enemy_plain.clone()
            }
        ));
        assert!(can_attack(
            s.state(),
            &runner,
            &DamageTarget::Unit {
                instance: taunt.clone()
            }
        ));
        // A plain attacker is bound by the same Taunt.
        assert!(!can_attack(s.state(), &plain, &DamageTarget::Hero { player: P2 }));
        assert!(!can_attack(
            s.state(),
            &plain,
            &DamageTarget::Unit {
                instance: enemy_plain.clone()
            }
        ));
        assert!(can_attack(
            s.state(),
            &plain,
            &DamageTarget::Unit { instance: taunt }
        ));
        // `legal_actions` agrees: the runner is offered the hero, the plain attacker is not.
        let offered = offered_targets(&s, &runner.id);
        assert!(offered.contains(&"hero-p2".to_string()));
        assert!(offered.contains(&enemy_plain.id));
        let plain_offered = offered_targets(&s, &plain.id);
        assert!(!plain_offered.contains(&"hero-p2".to_string()));
        assert!(!plain_offered.contains(&enemy_plain.id));
    }
}
