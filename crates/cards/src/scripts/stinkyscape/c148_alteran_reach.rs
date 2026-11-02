//! S #148 Alteran Reach (SPEC §8.10 row S 148; §6.1 Indestructible, §6.2 Quickdraw, §6.3, R46, R640,
//! R1847000–R1847002): (1) Field Spell, Wincon, Quickdraw, Mythic, a card of the Stinkyscape set.
//!   Base:    "Indestructible\nActivate ♾️: Pay (1) to travel one node along your starscape map and gain
//!            its reward.\nThe 20th node is the edge of the universe: reaching it adds an Edge of the
//!            Universe to your hand."
//!   Radiant: "Indestructible\nYour first travel each turn is free.\nActivate ♾️: …", the base face's
//!            text with the free travel added.
//!
//! Engine: Indestructible is a catalog keyword (R46), and Quickdraw the static flag `setup.rs` reads,
//! dealing it in the opening hand (R640). The travel is `subsystems::starscape_abilities`: an Activate
//! ♾️ for `STARSCAPE_TRAVEL_MANA` (R1847002) that moves its controller one node along their map, the
//! player's and not the card's, charted from the match rng at their first travel (R1847000), and gives
//! the node's reward (R1847001).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-148";

/// S #148.1 Edge of the Universe, the card the map's edge adds (R1847001).
const EDGE_OF_THE_UNIVERSE: &str = "stinkyscape-148-1";

fn alteran_reach(radiant: bool) -> Script {
    Script {
        // R640: dealt in the opening hand in place of a draw.
        static_flags: Some(StaticFlags {
            quickdraw: Some(true),
            ..StaticFlags::default()
        }),
        // R1847002: the paid travel; on the Radiant face a free first travel each turn switches with it.
        activations: subsystems::starscape_abilities(radiant, EDGE_OF_THE_UNIVERSE),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: alteran_reach(false),
        radiant: alteran_reach(true),
    }
}

// S #148 Alteran Reach — SPEC §8.10 row S 148, BUILD M12 row S 148.
#[cfg(test)]
mod tests {
    use super::{EDGE_OF_THE_UNIVERSE, ID, script};
    use jackioh_engine::subsystems::starscape::{
        STARSCAPE_FREE_TRAVEL_ABILITY, STARSCAPE_TRAVEL_ABILITY, starscape_amount,
    };
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const JAMMED: &str = "core-036";
    const FILLER: &str = "core-005";

    /// Neither
    /// side's turn ends by itself (R82).
    fn board_with(seed: &str, faces: &[bool]) -> Scenario {
        let reaches: Vec<Value> = faces
            .iter()
            .map(|radiant| json!({ "def": ID, "radiant": radiant }))
            .collect();
        let mut s = crate::scenario(json!({
            "seed": seed,
            "p1": { "backrow": reaches, "library": crate::filler(20), "mana": 4 },
            "p2": { "library": crate::filler(4), "health": 100 },
        }));
        s.state_mut().players.p1.auto_end_turn = Some(false);
        s.state_mut().players.p2.auto_end_turn = Some(false);
        s
    }

    fn board(seed: &str, radiant: bool) -> Scenario {
        board_with(seed, &[radiant])
    }

    fn heal_path() -> Vec<StarscapeReward> {
        let mut nodes = vec![StarscapeReward::Heal; (STARSCAPE_EDGE_TRAVELS - 1) as usize];
        nodes.push(StarscapeReward::Edge);
        nodes
    }

    fn set_map(s: &mut Scenario, nodes: Vec<StarscapeReward>, at: i32) {
        s.state_mut().players.p1.starscape = Some(StarscapeMap {
            nodes,
            at,
            travelled_on: 0,
        });
    }

    fn travel(s: &mut Scenario) -> &mut Scenario {
        s.activate(ID, json!({}))
    }

    fn at(s: &Scenario, player: PlayerId) -> i32 {
        s.state().players[player]
            .starscape
            .as_ref()
            .map_or(0, |map| map.at)
    }

    fn mana(s: &Scenario) -> i32 {
        s.state().players.p1.mana.current
    }

    fn traveled(events: &[GameEvent]) -> Vec<(PlayerId, i32, StarscapeReward)> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::StarscapeTravelled { player, node, reward } => Some((*player, *node, *reward)),
                _ => None,
            })
            .collect()
    }

    fn activations(s: &Scenario) -> Vec<(String, bool)> {
        let view = crate::js(&s.view(P1));
        view["you"]["backrow"][0]["activations"]
            .as_array()
            .expect("the Reach lists its abilities")
            .iter()
            .map(|ability| {
                (
                    ability["ability"].as_str().unwrap_or_default().to_string(),
                    ability["usable"].as_bool().unwrap_or(false),
                )
            })
            .collect()
    }

    fn legal_abilities(s: &Scenario) -> Vec<String> {
        let reach = s.backrow(P1, 1).expect("the Reach is in lane 1").id;
        legal_actions(s.state(), P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Activate {
                    instance_id, ability, ..
                } if instance_id == reach => Some(ability.unwrap_or_default()),
                _ => None,
            })
            .collect()
    }

    fn edge_in_hand(s: &Scenario) -> Option<CardInstance> {
        s.hand(P1)
            .into_iter()
            .find(|card| card.def_id == EDGE_OF_THE_UNIVERSE)
    }

    fn a_destroy_leaves_it(radiant: bool) {
        let mut s = crate::scenario(json!({
            "seed": "s148-indestructible",
            "p1": {
                "backrow": [{ "def": ID, "radiant": radiant }],
                "hand": [JAMMED, FILLER],
                "library": crate::filler(4),
                "mana": 4,
            },
            "p2": { "library": crate::filler(4) },
        }));
        let reach = s.card(ID).clone();
        s.play(JAMMED, json!({ "targets": pick::instance(&reach) }));
        s.expect_in_zone(&reach, "field");
    }

    mod s148_alteran_reach {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r640_quickdraw_deals_it_in_the_opening_hand() {
                crate::register_all();
                // R640: `setup` deals every library card carrying `quickdraw` into the opening hand,
                // the base face and the Radiant one alike.
                let scripts = script();
                assert_eq!(scripts.base.flags().quickdraw, Some(true));
                assert_eq!(scripts.radiant.flags().quickdraw, Some(true));
                // Plain Core cards, then the Reach: one copy of each (§2.6 L3).
                let others = [
                    "core-002", "core-005", "core-006", "core-008", "core-011", "core-012", "core-013", "core-015",
                    "core-016", "core-019", "core-020", "core-025", "core-026", "core-032", "core-036", "core-043",
                    "core-044", "core-053", "core-055", "core-001", "core-003", "core-004", "core-009", "core-010",
                ];
                let mut deck: Vec<String> = others.iter().map(|id| id.to_string()).collect();
                deck.push(ID.to_string());
                let created = create_game(&CreateGameOptions {
                    seed: "s148-quickdraw".to_string(),
                    decks: (deck.clone(), deck),
                    ..CreateGameOptions::default()
                });
                let state = begin_game(&created).state;
                assert!(state.players.p1.hand.iter().any(|card| card.def_id == ID));
                assert!(state.players.p1.library.iter().all(|card| card.def_id != ID));
            }

            #[test]
            fn r46_a_destroy_leaves_it_on_the_field() {
                a_destroy_leaves_it(false);
            }

            #[test]
            fn r1847000_the_first_travel_charts_the_map_from_the_match_rng() {
                let mut s = board("s148-chart", false);
                assert_eq!(s.state().players.p1.starscape, None, "no map before the first travel");
                let cursor = s.state().rng_cursor;
                travel(&mut s);
                let map = s
                    .state()
                    .players
                    .p1
                    .starscape
                    .clone()
                    .expect("the travel charts a map");
                assert_eq!(map.nodes.len() as i32, STARSCAPE_EDGE_TRAVELS);
                assert_eq!(map.nodes.last(), Some(&StarscapeReward::Edge), "the edge is the last node");
                for node in &map.nodes[..map.nodes.len() - 1] {
                    assert!(
                        STARSCAPE_REWARDS.iter().any(|(reward, _)| reward == node),
                        "{node:?} is a node reward"
                    );
                }
                assert_eq!(map.at, 1);
                assert!(s.state().rng_cursor > cursor, "the chart draws from the match rng");
                assert_eq!(traveled(s.last_events()), vec![(P1, 1, map.nodes[0])]);

                let mut again = board("s148-chart", false);
                travel(&mut again);
                assert_eq!(
                    again.state().players.p1.starscape.as_ref().map(|map| &map.nodes),
                    Some(&map.nodes),
                    "the same seed charts the same map"
                );
            }

            #[test]
            fn r1847001_each_travel_costs_1_and_gives_its_nodes_reward() {
                let mut s = board("s148-rewards", false);
                let mut nodes = vec![
                    StarscapeReward::Heal,
                    StarscapeReward::Draw,
                    StarscapeReward::Mana,
                    StarscapeReward::Damage,
                ];
                nodes.extend(vec![StarscapeReward::Heal; (STARSCAPE_EDGE_TRAVELS - 5) as usize]);
                nodes.push(StarscapeReward::Edge);
                set_map(&mut s, nodes, 0);

                let health = s.state().players.p1.hero.health;
                let before = mana(&s);
                travel(&mut s);
                assert_eq!(mana(&s), before - STARSCAPE_TRAVEL_MANA, "a travel costs (1)");
                assert_eq!(
                    s.state().players.p1.hero.health,
                    health + starscape_amount(StarscapeReward::Heal).unwrap_or(0),
                    "Heal heals p1's hero"
                );

                let hand = s.hand(P1).len();
                travel(&mut s);
                assert_eq!(
                    s.hand(P1).len() as i32,
                    hand as i32 + starscape_amount(StarscapeReward::Draw).unwrap_or(0),
                    "Draw draws"
                );

                let before = mana(&s);
                travel(&mut s);
                assert_eq!(
                    mana(&s),
                    before - STARSCAPE_TRAVEL_MANA + starscape_amount(StarscapeReward::Mana).unwrap_or(0),
                    "Mana gives back mana"
                );

                let enemy = s.state().players.p2.hero.health;
                travel(&mut s);
                assert_eq!(
                    s.state().players.p2.hero.health,
                    enemy - starscape_amount(StarscapeReward::Damage).unwrap_or(0),
                    "Damage hits the enemy hero"
                );
                assert_eq!(
                    traveled(s.events()),
                    vec![
                        (P1, 1, StarscapeReward::Heal),
                        (P1, 2, StarscapeReward::Draw),
                        (P1, 3, StarscapeReward::Mana),
                        (P1, 4, StarscapeReward::Damage),
                    ]
                );
            }

            #[test]
            fn r1847001_any_number_of_travels_a_turn_while_the_mana_lasts() {
                let mut s = board("s148-many", false);
                set_map(&mut s, heal_path(), 0);
                let travels = mana(&s) / STARSCAPE_TRAVEL_MANA;
                for _ in 0..travels {
                    travel(&mut s);
                }
                assert_eq!(at(&s, P1), travels);
                assert_eq!(mana(&s), 0);
                s.expect_refused_with(travel, "more than your mana");
            }

            #[test]
            fn r1847001_the_twentieth_travel_adds_edge_of_the_universe_and_no_travel_is_offered() {
                let mut s = board("s148-edge", false);
                set_map(&mut s, heal_path(), STARSCAPE_EDGE_TRAVELS - 1);
                travel(&mut s);
                assert_eq!(at(&s, P1), STARSCAPE_EDGE_TRAVELS);
                let edge = edge_in_hand(&s).expect("the edge adds an Edge of the Universe");
                assert!(!edge.radiant, "on its base face");
                assert_eq!(legal_abilities(&s), Vec::<String>::new(), "no travel is offered");
                assert_eq!(
                    activations(&s),
                    vec![(STARSCAPE_TRAVEL_ABILITY.to_string(), false)],
                    "the ability stays listed, unusable"
                );
                s.expect_refused_with(travel, "can't be activated now");
            }

            #[test]
            fn r1847001_a_full_hand_burns_the_edge() {
                let mut s = crate::scenario(json!({
                    "seed": "s148-edge-burn",
                    "p1": {
                        "backrow": [ID],
                        "hand": crate::filler(HAND_CAP as usize),
                        "library": crate::filler(4),
                    },
                    "p2": { "library": crate::filler(4) },
                }));
                s.state_mut().players.p1.auto_end_turn = Some(false);
                set_map(&mut s, heal_path(), STARSCAPE_EDGE_TRAVELS - 1);
                travel(&mut s);
                assert_eq!(edge_in_hand(&s), None, "a full hand has no room for it");
                assert!(
                    s.last_events().iter().any(
                        |event| matches!(event, GameEvent::Burned { def_id, .. } if def_id == EDGE_OF_THE_UNIVERSE)
                    ),
                    "it burns (§2.4)"
                );
                assert_eq!(at(&s, P1), STARSCAPE_EDGE_TRAVELS, "the edge is reached all the same");
            }

            #[test]
            fn r1847000_a_second_alteran_reach_travels_the_same_map() {
                let mut s = board_with("s148-second", &[false, false]);
                let first = s.backrow(P1, 1).expect("a Reach in lane 1");
                let second = s.backrow(P1, 2).expect("a Reach in lane 2");
                s.activate(&first, json!({}));
                s.activate(&second, json!({}));
                assert_eq!(at(&s, P1), 2, "both Reaches moved p1's one map");
                assert_eq!(s.state().players.p2.starscape, None, "p2 has no map");
            }

            #[test]
            fn declares_no_numbers() {
                assert!(crate::card_def(ID).params.is_none());
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r46_a_destroy_leaves_it_on_the_field_too() {
                a_destroy_leaves_it(true);
            }

            #[test]
            fn r1847002_the_first_travel_each_turn_is_free_and_the_next_costs_1() {
                let mut s = board("s148-free", true);
                set_map(&mut s, heal_path(), 0);
                s.state_mut().players.p1.mana.current = STARSCAPE_TRAVEL_MANA;
                travel(&mut s);
                assert_eq!(mana(&s), STARSCAPE_TRAVEL_MANA, "the first travel this turn is free");
                travel(&mut s);
                assert_eq!(mana(&s), 0, "the second costs its mana");
                s.expect_refused_with(travel, "more than your mana");

                s.end_turn().end_turn();
                assert_eq!(s.state().active, P1);
                s.state_mut().players.p1.mana.current = 0;
                travel(&mut s);
                assert_eq!(at(&s, P1), 3, "a new turn's first travel is free again");
            }

            #[test]
            fn r1847002_legal_actions_the_view_and_the_reducer_agree() {
                let mut s = board("s148-agree", true);
                set_map(&mut s, heal_path(), STARSCAPE_EDGE_TRAVELS - 2);

                assert_eq!(activations(&s), vec![(STARSCAPE_FREE_TRAVEL_ABILITY.to_string(), true)]);
                assert_eq!(legal_abilities(&s), vec![STARSCAPE_FREE_TRAVEL_ABILITY.to_string()]);

                travel(&mut s);
                assert_eq!(activations(&s), vec![(STARSCAPE_TRAVEL_ABILITY.to_string(), true)]);
                assert_eq!(legal_abilities(&s), vec![STARSCAPE_TRAVEL_ABILITY.to_string()]);

                travel(&mut s);
                assert_eq!(at(&s, P1), STARSCAPE_EDGE_TRAVELS);
                assert_eq!(activations(&s), vec![(STARSCAPE_TRAVEL_ABILITY.to_string(), false)]);
                assert_eq!(legal_abilities(&s), Vec::<String>::new());
                s.expect_refused_with(travel, "can't be activated now");
            }

            #[test]
            fn r1847001_the_radiant_face_adds_edge_of_the_universe_on_its_base_face() {
                let mut s = board("s148-radiant-edge", true);
                set_map(&mut s, heal_path(), STARSCAPE_EDGE_TRAVELS - 1);
                travel(&mut s);
                let edge = edge_in_hand(&s).expect("the edge adds an Edge of the Universe");
                assert!(!edge.radiant, "on its base face, whichever face traveled");
            }
        }
    }
}
