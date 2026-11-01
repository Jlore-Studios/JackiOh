//! S #12.3 Psychic Scream (SPEC §8.10 row S 12.3; §3.2, §6.3, R12, R1867006): (3) Spell,
//! Other-Origins, Token (printed Epic).
//!   Base:    "Shuffle all permanents into your opponent's deck."
//!   Radiant: "Shuffle all permanents into your opponent's deck. Each costs ({surcharge}) more (up
//!            to (4))." — surcharge 2, the Radiant face alone.
//!
//! reset (R78), a unit token ceasing (R11); a Dormant card and anything beneath a Stack pile never
//! match the scope (R13, R1839000). The card becomes the opponent's, owner and controller alike
//! (R12), so it lands in their deck (R311), a full one refusing the rest to the opponent's
//! graveyard (R80, R316). The Radiant surcharge is R1704003's take-back figured in the hook:
//! `surcharge` less the overshoot past SURCHARGE_UP_TO, whole from an X-cost card (an X is no cost
//! a number may move), so the raise lands before the move and every card still stands for it (R174).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-012-3";

/// R1704003: the Radiant raise stops at this cost.
const SURCHARGE_UP_TO: i32 = 4;

fn psychic_scream(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            // Every permanent on the field of both sides, in R68 order.
            let scope: BoardScope =
                json_as(json!({ "side": "any", "rows": ["units", "backrow"] }));
            let ids: Vec<String> =
                cards_in_scope(ctx, &scope).iter().map(|card| card.id.clone()).collect();
            let surcharge = param(&*ctx, "surcharge");
            let mut effects = Vec::new();
            for id in ids {
                if radiant {
                    // R1704003's take-back off the card's own cost now: the raise stops at the cap,
                    // and an X-cost card takes none of the raise.
                    let raise = match find_instance(&*ctx.state, &id)
                        .and_then(|card| own_cost(&*ctx.state, card))
                    {
                        None => 0,
                        Some(after) => {
                            (surcharge - (after + surcharge - SURCHARGE_UP_TO).max(0)).max(0)
                        }
                    };
                    if raise > 0 {
                        effects.push(set_cost_mod(json_as(json!({
                            "target": { "of": "instance", "instanceId": id },
                            "amount": raise,
                        }))));
                    }
                }
                effects.push(shuffle_card_into(json_as(json!({
                    "instanceId": id,
                    "player": "enemy",
                }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: psychic_scream(false),
        radiant: psychic_scream(true),
    }
}

// S #12.3 Psychic Scream — SPEC §8.10 row S 12.3, BUILD M12 row S 12.3 (R11, R1867006, R80).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const SCREAM: &str = "stinkyscape-012-3";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9.
    const SILAS: &str = "core-052"; // (4) Unit 4/4.
    const WELL: &str = "core-006"; // (3) Field Spell.
    const SHEEPISH: &str = "core-041"; // (1) Trap.
    const SHEEP: &str = "core-t-sheep"; // Unit token.
    const FILLER: &str = "core-005";

    fn shuffled(s: &Scenario) -> Vec<String> {
        s.events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::ShuffledIn { instance_id, .. } => Some(instance_id.clone()),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r1867006_every_permanent_of_both_sides_goes_into_the_opponents_deck_as_their_card() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [SCREAM, FILLER], "field": [VANILLA], "backrow": [WELL] },
                "p2": { "hand": [FILLER], "field": [MENACE], "backrow": [SHEEPISH] },
            }));
            let mine = s.unit_at(P1, 1).id.clone();
            let my_backrow = s.backrow(P1, 1).expect("my backrow card").id.clone();
            let theirs = s.unit_at(P2, 1).id.clone();
            let their_backrow = s.backrow(P2, 1).expect("their backrow card").id.clone();

            s.play(SCREAM, json!({}));

            assert!(s.unit(P1, 1).is_none());
            assert!(s.unit(P2, 1).is_none());
            assert!(s.backrow(P1, 1).is_none());
            assert!(s.backrow(P2, 1).is_none());
            assert!(
                s.events()
                    .iter()
                    .all(|event| !matches!(event, GameEvent::Destroyed { .. }))
            );
            // R68 order: my Units, my backrow, their Units, their backrow.
            assert_eq!(shuffled(&s), vec![mine.clone(), my_backrow, theirs.clone(), their_backrow]);
            for id in [&mine, &theirs] {
                let moved = s.card(id);
                assert_eq!(moved.owner, P2);
                assert_eq!(moved.controller, P2);
                assert!(matches!(moved.zone, Zone::Library { player: PlayerId::P2 }));
            }
            let defs: Vec<String> =
                s.pile(P2, "library").iter().map(|card| card.def_id.clone()).collect();
            assert!(defs.contains(&VANILLA.to_string()));
            assert!(defs.contains(&WELL.to_string()));
            assert!(defs.contains(&MENACE.to_string()));
            assert!(defs.contains(&SHEEPISH.to_string()));
        }

        #[test]
        fn r1867006_face_down_cards_go_too_a_dormant_one_stays() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [SCREAM, FILLER] },
                "p2": {
                    "hand": [FILLER],
                    "field": [
                        { "def": VANILLA, "lane": 1 },
                        { "def": VANILLA, "lane": 2, "slumber": { "turns": 2 } },
                    ],
                    "backrow": [{ "def": SHEEPISH, "faceUp": false, "lane": 1 }],
                },
            }));
            let open = s.unit_at(P2, 1).id.clone();
            let dormant = s.unit_at(P2, 2).id.clone();

            s.play(SCREAM, json!({}));

            let moved = s.card(&open);
            assert_eq!(moved.owner, P2);
            assert!(matches!(moved.zone, Zone::Library { player: PlayerId::P2 }));
            assert!(s.backrow(P2, 1).is_none());
            assert_eq!(
                s.pile(P2, "library").iter().filter(|card| card.def_id == SHEEPISH).count(),
                1
            );
            // The Dormant card is out of reach (R1839000).
            assert_eq!(s.card(&dormant).id, dormant);
            s.expect_in_zone(&dormant, "field");
        }

        #[test]
        fn r1867006_a_unit_token_is_gone_and_a_full_deck_refuses_the_rest() {
            crate::register_all();
            let library = vec![json!(FILLER); 60];
            let mut s = scenario(json!({
                "p1": { "hand": [SCREAM, FILLER], "field": [SHEEP, VANILLA] },
                "p2": { "hand": [FILLER], "library": library },
            }));
            let token = s.unit_at(P1, 1).id.clone();
            let unit = s.unit_at(P1, 2).id.clone();

            s.play(SCREAM, json!({}));

            s.expect_in_zone(&token, "gone");
            assert_eq!(s.pile(P2, "library").len(), 60);
            // Refused to the opponent's graveyard (R80, R316).
            s.expect_in_zone(&unit, "graveyard");
            assert_eq!(s.card(&unit).owner, P2);
            assert!(
                s.events()
                    .iter()
                    .any(|event| matches!(event, GameEvent::LibraryOverflow { .. }))
            );
        }

        #[test]
        fn r1867006_the_opponent_later_draws_the_casters_card_as_their_own() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [SCREAM, FILLER, FILLER], "field": [VANILLA] },
                "p2": { "hand": [FILLER], "library": [] },
            }));
            let mine = s.unit_at(P1, 1).id.clone();

            s.play(SCREAM, json!({}));
            s.end_turn(); // p2's turn: the draw.

            let drawn = s.hand(P2).into_iter().find(|card| card.id == mine).expect("drawn by p2");
            assert_eq!(drawn.owner, P2);
            assert_eq!(drawn.def_id, VANILLA);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r1867006_each_card_costs_2_more_there_a_3_card_4_and_a_4_card_still_4() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": SCREAM, "radiant": true }, FILLER] },
                "p2": { "hand": [FILLER], "field": [MENACE, SILAS] },
            }));
            let three = s.unit_at(P2, 1).id.clone();
            let four = s.unit_at(P2, 2).id.clone();

            s.play(SCREAM, json!({}));

            assert_eq!(s.card(&three).cost_mod, 1);
            assert_eq!(s.card(&four).cost_mod, 0);
            assert_eq!(effective_cost(s.state(), s.card(&three), Default::default()), 4);
            assert_eq!(effective_cost(s.state(), s.card(&four), Default::default()), 4);
        }

        #[test]
        fn r386_an_upgrade_of_the_surcharge_raises_more_still_up_to_4() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": SCREAM, "radiant": true }, FILLER] },
                "p2": { "hand": [FILLER], "field": [VANILLA] },
            }));
            step_param(s.card_mut(SCREAM), "surcharge", 1);
            let one = s.unit_at(P2, 1).id.clone();

            s.play(SCREAM, json!({}));

            assert_eq!(s.card(&one).cost_mod, 3);
            assert_eq!(effective_cost(s.state(), s.card(&one), Default::default()), 4);
        }
    }
}
