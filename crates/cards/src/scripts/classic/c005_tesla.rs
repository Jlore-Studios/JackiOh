//! C #5 Tesla (SPEC §8.6 row 5). Field Trap, cost 2, Epic, 1/4 → 2/8 (its unit face). Both faces:
//! "Animated, Lifesteal. Reveals when your opponent summons a Unit: Deal {damage} damage to it. Then
//! summon this as a Unit in Defense Position. Later Units are not hit." Damage 4 base, 8 Radiant.
//!
//! It answers each Unit summoned on the opponent's side (§6.3 Summon); a steal across (R171) or an
//! animating card (R383) was not summoned and leaves it set. A played Unit is answered after it resolves
//! (`cardResolved`, as #60 Bear Honeypot, R17), so its Cry happens first: the play's own `summoned`
//! (§10.5 step 4, which carries `exitsFrom`, R174) is skipped.
//!
//! The hit's source is Tesla, whose Lifesteal heals the amount dealt (§4.4 step 8). It then animates in
//! Defense Position (Animated, B3.1, R383): its own lane when open, else the leftmost open, unlocked,
//! unreserved zone (R64); with none open it stays face-up in its backrow zone. A Field Trap is never
//! consumed. Face-down it is hidden until it first fires (R33). The condition lives in `when` (R99).
//! It zaps only the Unit that reveals it (R831007, patch v0.3.10, #831): the first firing remembers
//! `revealed`, and the trigger answers nothing after that, wherever the card stands.

use jackioh_engine::effects::{animate, damage, remember};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-005";

/// The instance id of the opponent's Unit this event summons (or resolves, for a play), or None.
fn arrival(ctx: &EffectContext<'_>, event: &GameEvent) -> Option<String> {
    match event {
        GameEvent::Summoned { player, row, exits_from, instance_id, .. } => {
            if *player == ctx.controller || *row != Row::Units {
                return None;
            }
            // A play's own placement (§10.5 step 4) is answered when the play resolves, below.
            if exits_from.is_some() {
                return None;
            }
            Some(instance_id.clone())
        }
        GameEvent::CardResolved { permanent, def_id, instance_id, .. } => {
            let state: &GameState = &*ctx.state;
            if !*permanent || def_of(Some(state), def_id).type_ != CardType::Unit {
                return None;
            }
            let unit = find_instance(state, instance_id)?;
            if unit.zone.z() != ZoneName::Field || unit.controller == ctx.controller {
                return None;
            }
            Some(instance_id.clone())
        }
        _ => None,
    }
}

/// What Tesla keeps once it has revealed: it zaps only the Unit that revealed it (patch v0.3.10, #831).
const REVEALED: &str = "revealed";

fn has_revealed(ctx: &EffectContext<'_>) -> bool {
    recalled(ctx, REVEALED) == Some(Value::Bool(true))
}

fn zap() -> TriggerDef {
    TriggerDef::new("tesla-zap", &[GameEventType::Summoned, GameEventType::CardResolved], |ctx, event| {
        let Some(target) = arrival(ctx, event) else {
            return vec![];
        };
        if has_revealed(ctx) {
            return vec![];
        }
        // R781001: a Field Trap can't target an Immutable Unit in play, so Tesla still fires and
        // animates but deals it nothing.
        let state: &GameState = &*ctx.state;
        let shielded = find_instance(state, &target).is_some_and(|unit| immutable_bars(state, ctx.live_self(), unit));
        let mut effects = vec![remember(json_as(json!({ "key": REVEALED, "value": true })))];
        if !shielded {
            effects.push(damage(json_as(json!({
                "to": { "of": "instance", "instanceId": target },
                "amount": param(&*ctx, "damage"),
            }))));
        }
        effects.push(animate(json_as(json!({ "position": "DEF" }))));
        effects
    })
    .with_when(|ctx, event| !has_revealed(ctx) && arrival(ctx, event).is_some())
}

pub fn script() -> CardScripts {
    let base = Script {
        triggers: vec![zap()],
        ..Script::default()
    };

    // The same script: the Radiant face's 8 is its declared damage, which `param` reads off the face;
    // its 2/8 body and keywords are printed on the catalog face.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C #5 Tesla — SPEC §8.6 row 5, BUILD M9 Classic row C 5: face-down, read by its controller only (R33);
// fires on a Unit summoned on the opponent's side, not on a steal (R171) or an animating card (R383); a
// played one only after it resolves (R17); Lifesteal heals the amount dealt; the move keeps its damage and
// counters (R78 does not apply); radiant 2/8, 8 damage; its tuned number (damage) reads via `param()` (R386).
//
// Tesla sits face-down in p1's backrow lane 3; p2, the opponent, is active and brings Units in.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TESLA: &str = "classic-005";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const TIMMY: &str = "core-011"; // (1) 3/3.
    const MENACE: &str = "core-019"; // (3) 9/9 Taunt.
    const TOKEN_MAN: &str = "core-015"; // (1) 1/1, Cry: Summon a Rush Token (3/3).
    const CALL: &str = "core-069"; // (2) Spell: Recruit 3 Units of (1) Cost or less.
    const DEFENDER: &str = "core-003"; // (1) 1/1 Taunt, Divine Shield, Reborn.
    const HIT_JOB: &str = "core-016"; // (3) Destroy target Unit.
    const FEELINGS: &str = "classic-032"; // (1) Steal an enemy permanent in a lane where you control a (1) Cost Unit.
    const FILLER: &str = "core-010";

    use crate::js;

    use crate::matches_object;

    use crate::merged;

    fn set_tesla(radiant_face: bool, lane: i32) -> Value {
        json!({ "def": TESLA, "radiant": radiant_face, "faceUp": false, "lane": lane })
    }

    fn setup(p2: Value, p1: Value, radiant_face: bool) -> Scenario {
        scenario(json!({
            "active": "p2",
            "p1": merged(
                json!({ "hand": [FILLER], "backrow": [set_tesla(radiant_face, 3)], "health": 20, "library": [VANILLA, VANILLA] }),
                p1,
            ),
            "p2": merged(json!({ "hand": [FILLER], "library": [VANILLA, VANILLA] }), p2),
        }))
    }

    fn count(events: &[GameEvent], event_type: &str) -> usize {
        events.iter().filter(|event| event.event_type().as_str() == event_type).count()
    }

    mod c_n5_tesla {
        use super::*;

        #[test]
        fn is_a_field_trap_with_animated_and_lifesteal_printed_both_faces_run_one_script() {
            crate::register_all();
            let def = crate::card_def(TESLA);
            assert_eq!(def.type_, CardType::FieldTrap);
            let kinds: Vec<KeywordKind> = def.base.keywords.iter().map(Keyword::kind).collect();
            assert_eq!(kinds, vec![KeywordKind::Animated, KeywordKind::Lifesteal]);
            // The radiant face is the very same script.
            let scripts = script();
            assert_eq!(scripts.base.triggers.len(), scripts.radiant.triggers.len());
            for (base, radiant) in scripts.base.triggers.iter().zip(&scripts.radiant.triggers) {
                assert!(Arc::ptr_eq(&base.run, &radiant.run));
            }
        }

        mod base {
            use super::*;

            #[test]
            fn r33_face_down_it_is_read_by_its_controller_only() {
                crate::register_all();
                let s = setup(json!({ "hand": [VANILLA] }), json!({}), false);
                assert!(js(&s.view(P1))["you"]["backrow"].to_string().contains(TESLA));
                assert!(!js(&s.view(P2)).to_string().contains(TESLA));
            }

            #[test]
            fn r17_a_played_unit_is_answered_after_it_resolves_4_damage_from_tesla_then_tesla_animates_in_defense() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [VANILLA, FILLER] }), json!({}), false);
                let vanilla = s.card(VANILLA).clone();

                s.play(&vanilla, json!({ "zone": 2 }));

                s.expect_in_zone(&vanilla, "graveyard");
                let tesla = s.card(TESLA).clone();
                assert!(matches_object(&js(&tesla.zone), &json!({ "z": "field", "row": "units", "lane": 3 })));
                assert_eq!(s.stats(&tesla).position, Position::Def);
                s.expect_events(json!(["cardPlayed", "cardResolved", "trapFired", "damage", "animated"]));
                let hit = s.events().iter().map(js).find(|event| event["type"] == "damage");
                assert!(matches_object(
                    &hit.unwrap_or(Value::Null),
                    &json!({ "sourceId": tesla.id, "amount": 4 })
                ));
            }

            #[test]
            fn r781002_an_immutable_unit_it_fires_on_takes_no_damage_and_tesla_still_animates() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [{ "def": MENACE, "radiant": true }, FILLER], "mana": 10 }), json!({}), false);
                let menace = s.card(MENACE).clone();

                s.play(&menace, json!({ "zone": 2 }));

                // A Field Trap can't target an Immutable Unit in play (R781001): Tesla fires and animates,
                // but its hit passes the Radiant Menace by, so its Lifesteal heals nothing.
                let tesla = s.card(TESLA).clone();
                assert!(matches_object(&js(&tesla.zone), &json!({ "z": "field", "row": "units", "lane": 3 })));
                assert_eq!(count(s.events(), "trapFired"), 1);
                assert_eq!(count(s.events(), "damage"), 0);
                s.expect_stats(&menace, json!({ "health": 18 }));
                s.expect_health(P1, 20);
            }

            #[test]
            fn s4_4_step_8_its_lifesteal_heals_you_the_amount_dealt() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [VANILLA, FILLER] }), json!({}), false);

                s.play(VANILLA, json!({ "zone": 2 }));

                s.expect_health(P1, 24);
            }

            #[test]
            fn r17_r831007_the_played_unit_s_cry_happens_first_its_token_reveals_it_and_is_the_only_one_hit() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [TOKEN_MAN, FILLER] }), json!({}), false);
                let man = s.card(TOKEN_MAN).clone();

                s.play(&man, json!({ "zone": 1 }));

                assert_eq!(count(s.events(), "trapFired"), 1);
                // The Cry's token is hit (its summon reveals Tesla); the played Unit, answered once the play
                // has resolved, is a later Unit and is not hit.
                let token = s.events().iter().find_map(|event| match event {
                    GameEvent::Summoned { def_id, instance_id, .. } if def_id == "core-t-rush" => Some(instance_id.clone()),
                    _ => None,
                });
                let Some(token) = token else {
                    panic!("no token was summoned");
                };
                let hits: Vec<String> = s
                    .events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::Damage { target_id, .. } => Some(target_id.clone()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(hits, vec![token]);
                s.expect_in_zone(&man, "field");
                // The one hit heals: 4 into the 3/3 token.
                s.expect_health(P1, 24);
            }

            #[test]
            fn s6_3_r831007_a_recruit_s_arrivals_are_summons_too_the_first_reveals_it_and_alone_is_hit() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [CALL, FILLER], "library": [TIMMY, TIMMY, VANILLA, MENACE] }), json!({}), false);

                s.play(CALL, json!({}));

                // Call to Arms recruits the three (1) Cost Units (Timmy, Timmy, Vanilla), never the (3) Menace;
                // the first one's summon reveals Tesla, and the other two are later Units.
                assert_eq!(count(s.events(), "trapFired"), 1);
                let graveyard = s.pile(P2, "graveyard");
                assert_eq!(graveyard.iter().filter(|card| card.def_id == TIMMY).count(), 1);
                assert_eq!(graveyard.iter().filter(|card| card.def_id == VANILLA).count(), 0);
            }

            #[test]
            fn s4_5_a_reborn_body_is_summoned_and_answered_on_its_controller_s_own_turn_too() {
                crate::register_all();
                // On p1's turn p1 destroys p2's Right-house defender; Reborn brings it back on p2's side, which
                // is a Unit summoned there, so p1's Tesla answers it (its Divine Shield takes the hit).
                let mut s = scenario(json!({
                    "p1": { "hand": [HIT_JOB, FILLER], "backrow": [set_tesla(false, 3)], "health": 20, "library": [VANILLA] },
                    "p2": { "hand": [FILLER], "field": [{ "def": DEFENDER, "lane": 2 }] },
                }));
                let defender = s.card(DEFENDER).clone();

                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": defender.id }] }));

                s.expect_in_zone(&defender, "field");
                assert_eq!(count(s.events(), "trapFired"), 1);
                assert_eq!(count(s.events(), "animated"), 1);
                s.expect_events(json!(["destroyed", "summoned", "trapFired", "divineShieldLost"]));
            }

            #[test]
            fn s4_4_a_divine_shield_takes_the_hit_0_dealt_so_its_lifesteal_heals_0() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [DEFENDER, FILLER] }), json!({}), false);

                s.play(DEFENDER, json!({ "zone": 1 }));

                s.expect_in_zone(DEFENDER, "field");
                s.expect_events(json!(["trapFired", "divineShieldLost"]));
                s.expect_health(P1, 20);
            }

            #[test]
            fn your_own_units_never_set_it_off() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [VANILLA, FILLER], "backrow": [set_tesla(false, 3)] },
                    "p2": { "hand": [FILLER] },
                }));

                s.play(VANILLA, json!({ "zone": 1 }));

                assert_eq!(count(s.events(), "trapFired"), 0);
                assert_eq!(s.card(TESLA).face_up, Some(false));
            }

            #[test]
            fn r171_a_unit_stolen_across_was_not_summoned_and_leaves_it_set() {
                crate::register_all();
                let mut s = setup(
                    json!({ "hand": [FEELINGS, FILLER], "field": [{ "def": TIMMY, "lane": 4 }] }),
                    json!({ "field": [{ "def": MENACE, "lane": 4 }] }),
                    false,
                );
                let menace = s.card(MENACE).clone();

                s.play(FEELINGS, json!({ "targets": [{ "pick": "instance", "instanceId": menace.id }] }));

                assert_eq!(s.card(&menace).controller, P2);
                assert_eq!(count(s.events(), "trapFired"), 0);
            }

            #[test]
            fn r383_an_animated_card_animating_on_the_opponent_s_side_was_not_summoned_and_leaves_it_set() {
                crate::register_all();
                // p1 plays a Unit on p1's turn: p2's own Tesla answers it and animates on p2's side. That move
                // is no summon, so p1's Tesla stays face-down.
                let mut s = scenario(json!({
                    "p1": { "hand": [VANILLA, FILLER], "backrow": [set_tesla(false, 3)], "library": [VANILLA] },
                    "p2": { "hand": [FILLER], "backrow": [{ "def": TESLA, "faceUp": false, "lane": 3 }], "library": [VANILLA] },
                }));
                let (Some(mine), Some(theirs)) = (s.backrow(P1, 3), s.backrow(P2, 3)) else {
                    panic!("fixture");
                };

                s.play(VANILLA, json!({ "zone": 1 }));

                assert!(matches_object(
                    &js(&s.card(&theirs).zone),
                    &json!({ "z": "field", "row": "units", "player": "p2" })
                ));
                assert_eq!(count(s.events(), "animated"), 1);
                assert_eq!(count(s.events(), "trapFired"), 1);
                assert_eq!(s.card(&mine).face_up, Some(false));
                assert!(matches_object(&js(&s.card(&mine).zone), &json!({ "row": "backrow", "lane": 3 })));
            }

            #[test]
            fn r383_r831007_no_open_unit_zone_it_stays_face_up_in_its_backrow_zone_and_hits_no_later_unit() {
                crate::register_all();
                let full = vec![VANILLA, VANILLA, VANILLA, VANILLA, VANILLA];
                let mut s = setup(json!({ "hand": [TIMMY, TIMMY, FILLER] }), json!({ "field": full }), false);
                let tesla = s.card(TESLA).clone();

                s.play(TIMMY, json!({ "zone": 1 }));
                assert!(matches_object(
                    &js(&s.card(&tesla).zone),
                    &json!({ "z": "field", "row": "backrow", "lane": 3 })
                ));
                assert_eq!(s.card(&tesla).face_up, Some(true));

                let Some(second) = s.hand(P2).into_iter().find(|card| card.def_id == TIMMY) else {
                    panic!("fixture");
                };
                s.play(&second, json!({ "zone": 2 }));

                assert_eq!(count(s.events(), "trapFired"), 1);
                s.expect_in_zone(&second, "field");
            }

            #[test]
            fn r383_its_own_lane_s_unit_zone_taken_it_animates_into_the_leftmost_open_one() {
                crate::register_all();
                let mut s = setup(
                    json!({ "hand": [VANILLA, FILLER] }),
                    json!({ "field": [{ "def": TIMMY, "lane": 3 }, { "def": TIMMY, "lane": 1 }] }),
                    false,
                );

                s.play(VANILLA, json!({ "zone": 2 }));

                assert!(matches_object(
                    &js(&s.card(TESLA).zone),
                    &json!({ "z": "field", "row": "units", "lane": 2 })
                ));
            }

            #[test]
            fn r831007_once_it_has_revealed_a_unit_played_after_it_is_not_hit_and_tesla_stays_put_in_defense() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [VANILLA, TIMMY, FILLER] }), json!({}), false);

                s.play(VANILLA, json!({ "zone": 1 }));
                s.play(TIMMY, json!({ "zone": 2 }));

                let tesla = s.card(TESLA).clone();
                assert!(matches_object(&js(&tesla.zone), &json!({ "z": "field", "row": "units", "lane": 3 })));
                assert_eq!(s.stats(&tesla).position, Position::Def);
                assert_eq!(count(s.events(), "trapFired"), 1);
                assert_eq!(count(s.events(), "animated"), 1);
                s.expect_in_zone(VANILLA, "graveyard");
                s.expect_in_zone(TIMMY, "field");
                // Nor on a later turn.
                s.end_turn();
                s.end_turn();
                let Some(later) = s.hand(P2).into_iter().find(|card| card.def_id == VANILLA) else {
                    panic!("p2 drew a Vanilla");
                };
                s.play(&later, json!({ "zone": 4 }));
                s.expect_in_zone(&later, "field");
                assert_eq!(count(s.events(), "trapFired"), 1);
            }

            #[test]
            fn r383_animated_it_is_a_unit_for_every_rule_it_can_be_attacked_and_dies_to_its_owner_s_graveyard() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [VANILLA, FILLER], "field": [{ "def": MENACE, "lane": 1 }] }), json!({}), false);

                s.play(VANILLA, json!({ "zone": 2 }));
                let tesla = s.card(TESLA).clone();
                s.attack(MENACE, &tesla);

                s.expect_in_zone(&tesla, "graveyard");
                let graveyard: Vec<String> = s.pile(P1, "graveyard").into_iter().map(|card| card.id).collect();
                assert!(graveyard.contains(&tesla.id));
            }

            #[test]
            fn r383_the_move_keeps_nothing_reset_an_animated_tesla_is_summoning_sick_on_its_first_turn_as_a_unit() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [VANILLA, FILLER] }), json!({}), false);
                s.play(VANILLA, json!({ "zone": 2 }));
                // p1's own turn comes: Tesla entered the unit row on p2's turn, so it may attack now — but the
                // turn it animated on, a p1 attack would have been refused.
                let tesla = s.card(TESLA).clone();
                assert_eq!(s.card(&tesla).summoned_turn, Some(s.state().turn));
            }

            #[test]
            fn r383_moving_is_not_leaving_the_field_its_counters_ride_from_the_backrow_into_the_unit_row() {
                crate::register_all();
                let mut tesla_setup = set_tesla(false, 3);
                tesla_setup["counters"] = json!({ "plague": 2 });
                let mut s = scenario(json!({
                    "active": "p2",
                    "p1": { "hand": [FILLER], "backrow": [tesla_setup], "library": [VANILLA] },
                    "p2": { "hand": [VANILLA, FILLER], "library": [VANILLA] },
                }));

                s.play(VANILLA, json!({ "zone": 2 }));

                let tesla = s.card(TESLA).clone();
                assert!(matches_object(&js(&tesla.zone), &json!({ "row": "units" })));
                assert_eq!(tesla.counters.plague, Some(2));
            }

            #[test]
            fn r97_the_opponent_s_view_never_named_it_before_it_fired_animated_names_it_after() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [VANILLA, FILLER] }), json!({}), false);
                assert!(!js(&s.view(P2)).to_string().contains(TESLA));

                s.play(VANILLA, json!({ "zone": 2 }));

                let theirs = js(&s.view(P2));
                let animated = theirs["events"]
                    .as_array()
                    .and_then(|events| events.iter().find(|event| event["type"] == "animated").cloned());
                assert!(animated.map(|event| event.to_string()).unwrap_or_default().contains(TESLA));
            }

            #[test]
            fn r386_an_upgrade_of_its_damage_deals_5() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [MENACE, FILLER] }), json!({}), false);
                step_param(s.card_mut(TESLA), "damage", 1);

                s.play(MENACE, json!({ "zone": 2 }));

                s.expect_stats(MENACE, json!({ "health": 4 }));
                s.expect_health(P1, 25);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r275_its_unit_face_is_a_2_8() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [VANILLA, FILLER] }), json!({}), true);

                s.play(VANILLA, json!({ "zone": 2 }));

                s.expect_stats(TESLA, json!({ "attack": 2, "health": 8, "maxHealth": 8 }));
            }

            #[test]
            fn deals_8_and_heals_8() {
                crate::register_all();
                let mut s = setup(json!({ "hand": [MENACE, FILLER] }), json!({}), true);

                s.play(MENACE, json!({ "zone": 2 }));

                s.expect_stats(MENACE, json!({ "health": 1 }));
                s.expect_health(P1, 28);
            }
        }
    }
}
