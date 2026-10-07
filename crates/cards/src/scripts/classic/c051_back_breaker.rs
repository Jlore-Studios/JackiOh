//! C #51 Back Breaker (SPEC §8.6 row 51). (1) Unit, Common, 3/2 → 6/4.
//!   Base:    "Stack\nDeath: Destroy every backrow card."
//!   Radiant: "Stack\nDeath: Destroy every enemy backrow card."
//!   Engine:  "Both backrows (Radiant: the enemy's), face-down cards included: the top card of each
//!            backrow zone (a dormant card under a backrow pile is not on the field, R13, §3.2).
//!            Indestructible cards stay (#98 Heroic Power, C #84 Lockdown, C #90 In Too Deep). An
//!            animated card (Animated, §6.1, R383) is in the unit row and is not backrow. Tunes: none."
//!
//! Stack is the catalog keyword (§6.2, §3.2): it may be played onto an occupied unit zone, burying the
//! card beneath (R13), which resumes when it leaves. The Death is §4.5 step 3's hook, so it fires on any
//! death — a destroy, a Tribute (a Sacrifice counts as one, §6.3), a combat — and never on a bounce or an
//! exile, which are no deaths. It is a board sweep (`destroyAll` over the backrow row): every card that
//! acts in a backrow zone is marked — face-down ones too, since a backrow scope reaches them — and only
//! the top of a backrow pile, since a dormant card is not on the field (R13); the next state check
//! collects them together (R59), the one beneath a destroyed top resuming. Indestructible ones keep
//! their zones (R46). An animated card stands in the unit row (R383) and a carried Unit is no backrow
//! card (R446), so neither is reached. Radiant: `side: "enemy"`, relative to the controller it died
//! under (the Death hook runs as the snapshot's controller, R89).
//!
//! Rulings: R13, R46, R59, R383, R89. Its proof: `test/classic/051-back-breaker.test.ts`.

use jackioh_engine::effects::destroy_all;
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-051";

pub fn script() -> CardScripts {
    let base = Script {
        death: Some(hook(|_ctx| vec![destroy_all(json_as(json!({ "side": "any", "rows": ["backrow"] })))])),
        ..Script::default()
    };

    let radiant = Script {
        death: Some(hook(|_ctx| vec![destroy_all(json_as(json!({ "side": "enemy", "rows": ["backrow"] })))])),
        ..Script::default()
    };

    CardScripts { base, radiant }
}

// C #51 Back Breaker (SPEC §8.6 row 51; BUILD M9 row C 51). (1) Unit, Common, 3/2 → 6/4: Stack;
// Death: destroy every backrow card (Radiant: every enemy backrow card).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BACK_BREAKER: &str = "classic-051";
    const VANILLA: &str = "core-008"; // 4/4
    const MENACE: &str = "core-019"; // 9/9 Taunt
    const HIT_JOB: &str = "core-016"; // Spell 3: destroy target Unit
    const FLOOD: &str = "core-017"; // Spell 4: bounce all Units
    const COLLATERAL: &str = "core-034"; // Spell 4: exile target permanent and a random card of the enemy deck
    const THE_ROCK: &str = "core-066"; // Tribute 1, Indestructible
    const MANA_WELL: &str = "core-006"; // Field Spell
    const GOING_LONG: &str = "core-084"; // Field Spell
    const SHEEPISH: &str = "core-041"; // Trap
    const HONEYPOT: &str = "core-060"; // Trap
    const HEROIC_POWER: &str = "core-098"; // Field Spell, Indestructible
    const LOCKDOWN: &str = "classic-084"; // Field Spell, Indestructible
    const IN_TOO_DEEP: &str = "classic-090"; // Field Spell, Indestructible
    const TESLA: &str = "classic-005"; // Field Trap, Animated
    const STOCKPILE: &str = "core-005";

    use crate::js;

    /// TS `SPARE: SideSetup = { hand: [STOCKPILE], library: [VANILLA, VANILLA] }`.
    fn spare() -> Value {
        json!({ "hand": [STOCKPILE], "library": spare_library() })
    }

    /// TS `SPARE.library`.
    fn spare_library() -> Value {
        json!([VANILLA, VANILLA])
    }

    /// TS `{ ...side, ...SPARE }`: the side's keys with SPARE's laid over them.
    fn with_spare(side: Value) -> Value {
        let mut out = side;
        if let (Some(fields), Some(over)) = (out.as_object_mut(), spare().as_object()) {
            for (key, value) in over {
                fields.insert(key.clone(), value.clone());
            }
        }
        out
    }

    fn backrow_ids(s: &Scenario, player: PlayerId) -> Vec<Option<String>> {
        [1, 2, 3, 4, 5].into_iter().map(|lane| s.backrow(player, lane).map(|card| card.def_id)).collect()
    }

    /// TS `bothBackrows()`: p1's backrow and p2's.
    fn both_backrows() -> (Value, Value) {
        (
            json!([MANA_WELL, { "def": SHEEPISH, "faceUp": false }]),
            json!([GOING_LONG, { "def": HONEYPOT, "faceUp": false }]),
        )
    }

    fn graveyard_defs(s: &Scenario, player: PlayerId) -> Vec<String> {
        s.pile(player, "graveyard").into_iter().map(|card| card.def_id).collect()
    }

    fn hit(s: &mut Scenario, card: &str) {
        let id = s.card(card).id.clone();
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": id }] }));
    }

    mod c51_back_breaker {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn s3_2_stack_it_may_be_played_onto_an_occupied_unit_zone_the_card_beneath_dormant_r13_and_resuming_when_it_leaves() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [BACK_BREAKER, HIT_JOB, STOCKPILE], "field": [VANILLA], "library": spare_library(), "mana": 10 },
                    "p2": spare(),
                }));
                let breaker = s.card(BACK_BREAKER).clone();
                assert!(legal_actions(s.state(), P1).iter().map(js).any(|a| {
                    a["type"] == "play" && a["instanceId"] == breaker.id.as_str() && a["zone"]["lane"] == 1
                }));
                s.play(BACK_BREAKER, json!({ "zone": 1 }));
                assert_eq!(s.unit(P1, 1).map(|card| card.def_id), Some(BACK_BREAKER.to_string()));
                let first = js(&s.view(P1))["you"]["units"][0].clone();
                assert_eq!(first["defId"], BACK_BREAKER);
                assert_eq!(first["buried"], 1);
                hit(&mut s, BACK_BREAKER);
                s.expect_in_zone(BACK_BREAKER, "graveyard");
                assert_eq!(s.unit(P1, 1).map(|card| card.def_id), Some(VANILLA.to_string()));
            }

            #[test]
            fn s8_6_death_by_a_destroy_every_backrow_card_on_both_sides_face_down_ones_included() {
                crate::register_all();
                let (backrow, p2_backrow) = both_backrows();
                let mut s = scenario(json!({
                    "p1": { "field": [BACK_BREAKER], "backrow": backrow, "hand": [HIT_JOB, STOCKPILE], "library": spare_library() },
                    "p2": with_spare(json!({ "backrow": p2_backrow })),
                }));
                hit(&mut s, BACK_BREAKER);
                assert_eq!(backrow_ids(&s, P1), vec![None, None, None, None, None]);
                assert_eq!(backrow_ids(&s, P2), vec![None, None, None, None, None]);
                for id in [MANA_WELL, SHEEPISH] {
                    assert!(graveyard_defs(&s, P1).iter().any(|card| card == id));
                }
                for id in [GOING_LONG, HONEYPOT] {
                    assert!(graveyard_defs(&s, P2).iter().any(|card| card == id));
                }
            }

            #[test]
            fn s4_5_a_combat_death_fires_it_too() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": with_spare(json!({ "field": [BACK_BREAKER], "backrow": [MANA_WELL] })),
                    "p2": with_spare(json!({ "field": [MENACE], "backrow": [GOING_LONG] })),
                }));
                s.attack(BACK_BREAKER, MENACE);
                s.expect_in_zone(BACK_BREAKER, "graveyard");
                assert!(backrow_ids(&s, P1)[0].is_none());
                assert!(backrow_ids(&s, P2)[0].is_none());
            }

            #[test]
            fn s6_3_a_tribute_is_a_death_tributed_for_the_rock_its_death_fires() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "field": [BACK_BREAKER], "backrow": [MANA_WELL], "hand": [THE_ROCK, STOCKPILE], "library": spare_library(), "mana": 10 },
                    "p2": with_spare(json!({ "backrow": [GOING_LONG] })),
                }));
                s.play(THE_ROCK, json!({ "tributes": [BACK_BREAKER] }));
                s.expect_in_zone(BACK_BREAKER, "graveyard");
                assert!(backrow_ids(&s, P1)[0].is_none());
                assert!(backrow_ids(&s, P2)[0].is_none());
            }

            #[test]
            fn r46_indestructible_backrow_cards_stay_heroic_power_lockdown_in_too_deep() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "field": [BACK_BREAKER], "backrow": [HEROIC_POWER, IN_TOO_DEEP], "hand": [HIT_JOB, STOCKPILE], "library": spare_library() },
                    "p2": with_spare(json!({ "backrow": [LOCKDOWN, MANA_WELL] })),
                }));
                hit(&mut s, BACK_BREAKER);
                assert_eq!(json!(backrow_ids(&s, P1)[0..2]), json!([HEROIC_POWER, IN_TOO_DEEP]));
                assert_eq!(json!(backrow_ids(&s, P2)[0..2]), json!([LOCKDOWN, null]));
            }

            #[test]
            fn r13_only_the_top_of_a_backrow_pile_is_destroyed_and_the_card_beneath_resumes() {
                crate::register_all();
                // A backrow pile (B5 E21): a Mana Well stacked onto Going Long, as a Stack play would put it.
                let mut s = scenario(json!({
                    "p1": { "field": [BACK_BREAKER], "hand": [HIT_JOB, STOCKPILE], "library": spare_library() },
                    "p2": with_spare(json!({ "backrow": [GOING_LONG, { "def": MANA_WELL, "stack": true }] })),
                }));
                let top = s.backrow(P2, 1).expect("p2's backrow pile should have a top");
                assert_eq!(top.def_id, MANA_WELL);
                hit(&mut s, BACK_BREAKER);
                s.expect_in_zone(&top, "graveyard");
                assert_eq!(s.backrow(P2, 1).map(|card| card.def_id), Some(GOING_LONG.to_string()));
            }

            #[test]
            fn r383_an_animated_card_stands_in_the_unit_row_and_is_spared() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "field": [BACK_BREAKER], "hand": [HIT_JOB, STOCKPILE], "library": spare_library() },
                    "p2": with_spare(json!({ "backrow": [TESLA, MANA_WELL] })),
                }));
                // Tesla steps into the unit row as its firing's last step would (B3.1, R383).
                let tesla = s.card(TESLA).clone();
                let mut rng = Rng::new(&s.state().seed.clone(), s.state().rng_cursor);
                let mut events: Vec<GameEvent> = Vec::new();
                let animated = {
                    let mut sink = EngineSink::new(s.state_mut(), &mut events, &mut rng);
                    animate_card(&mut sink, &tesla, Default::default())
                };
                assert!(animated);
                assert_eq!(s.unit(P2, 1).map(|card| card.def_id), Some(TESLA.to_string()));
                hit(&mut s, BACK_BREAKER);
                assert_eq!(s.unit(P2, 1).map(|card| card.def_id), Some(TESLA.to_string()));
                assert!(backrow_ids(&s, P2)[1].is_none());
            }

            #[test]
            fn s8_6_a_bounce_or_an_exile_is_no_death_nothing_fires() {
                crate::register_all();
                let mut bounced = scenario(json!({
                    "p1": { "field": [BACK_BREAKER], "backrow": [MANA_WELL], "hand": [FLOOD, STOCKPILE], "library": spare_library(), "mana": 10 },
                    "p2": with_spare(json!({ "backrow": [GOING_LONG] })),
                }));
                bounced.play(FLOOD, json!({}));
                bounced.expect_in_zone(BACK_BREAKER, "hand");
                assert_eq!(backrow_ids(&bounced, P1)[0].as_deref(), Some(MANA_WELL));
                assert_eq!(backrow_ids(&bounced, P2)[0].as_deref(), Some(GOING_LONG));

                let mut exiled = scenario(json!({
                    "p1": { "backrow": [MANA_WELL], "hand": [COLLATERAL, STOCKPILE], "library": spare_library(), "mana": 10 },
                    "p2": with_spare(json!({ "field": [BACK_BREAKER], "backrow": [GOING_LONG] })),
                }));
                let breaker = exiled.card(BACK_BREAKER).id.clone();
                exiled.play(COLLATERAL, json!({ "targets": [{ "pick": "instance", "instanceId": breaker }] }));
                exiled.expect_in_zone(BACK_BREAKER, "exile");
                assert_eq!(backrow_ids(&exiled, P1)[0].as_deref(), Some(MANA_WELL));
                assert_eq!(backrow_ids(&exiled, P2)[0].as_deref(), Some(GOING_LONG));
            }

            #[test]
            fn s8_6_3_2_with_stack() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "field": [BACK_BREAKER] } }));
                s.expect_stats(BACK_BREAKER, json!({ "attack": 3, "health": 2 }));
                let keywords = js(&s.stats(BACK_BREAKER).keywords);
                assert!(keywords.as_array().is_some_and(|all| all.contains(&json!({ "kind": "Stack" }))));
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn s8_6_6_4_its_death_destroys_the_enemy_backrow_only_face_down_cards_included() {
                crate::register_all();
                let (backrow, p2_backrow) = both_backrows();
                let mut s = scenario(json!({
                    "p1": {
                        "field": [{ "def": BACK_BREAKER, "radiant": true }],
                        "backrow": backrow,
                        "hand": [HIT_JOB, STOCKPILE],
                        "library": spare_library(),
                    },
                    "p2": with_spare(json!({ "backrow": p2_backrow })),
                }));
                s.expect_stats(BACK_BREAKER, json!({ "attack": 6, "health": 4 }));
                hit(&mut s, BACK_BREAKER);
                assert_eq!(json!(backrow_ids(&s, P1)[0..2]), json!([MANA_WELL, SHEEPISH]));
                assert_eq!(backrow_ids(&s, P2), vec![None, None, None, None, None]);
            }

            #[test]
            fn r89_enemy_is_the_opponent_of_the_player_it_died_under_on_p2s_side_it_takes_p1s_backrow() {
                crate::register_all();
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": with_spare(json!({ "backrow": [MANA_WELL] })),
                    "p2": {
                        "field": [{ "def": BACK_BREAKER, "radiant": true }],
                        "backrow": [GOING_LONG],
                        "hand": [HIT_JOB, STOCKPILE],
                        "library": spare_library(),
                    },
                }));
                hit(&mut s, BACK_BREAKER);
                assert!(backrow_ids(&s, P1)[0].is_none());
                assert_eq!(backrow_ids(&s, P2)[0].as_deref(), Some(GOING_LONG));
            }
        }
    }
}
