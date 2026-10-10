//! C #66 EU Striker (SPEC §8.6 row 66, BUILD M9 Classic row C 66). (2) Unit, Human, 5/4 → 10/8, Common.
//!   Base:    "While this is in your hand: After you play a Unit, summon this. / After you play a card,
//!            return this to your hand."
//!   Radiant: "Rush / (the same)."
//!   Engine:  "Hand and deck triggers (§6.2): a hand trigger on your play of a Unit once it has
//!            resolved (its `cardResolved`, R548), summoning this (no Cry, R1; summoning sick, §4.1)
//!            into your leftmost open, unlocked, unreserved unit zone (R64; none open: it stays in hand);
//!            a field trigger on your play of any card once it has resolved (its `cardResolved`, R548),
//!            returning this to your hand (R78's reset; the hand cap applies).
//!            Neither trigger answers the play that moved the card (R401, R119): the Unit that summons it
//!            doesn't bounce it, and the card that bounces it doesn't summon it back. Tunes: none."
//! Dispatch (§10.3, R212, §10.5, R70, B5), bounce (§6.3, §2.4), printed Rush (§10.4).

use jackioh_engine::effects::{bounce, summon_this};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-066";

/// R548: a play of this card's controller's that has resolved, other than this card's own (R119).
fn your_other_play<'e>(ctx: &EffectContext<'_>, event: &'e GameEvent) -> Option<&'e GameEvent> {
    let GameEvent::CardResolved { player, instance_id, .. } = event else {
        return None;
    };
    if *player != ctx.controller {
        return None;
    }
    if ctx.self_.as_ref().is_some_and(|me| me.id == *instance_id) {
        return None;
    }
    Some(event)
}

/// While this is in your hand: after you play a Unit, summon this (B5 E26).
fn arrive() -> TriggerDef {
    TriggerDef::new("eu-striker-arrive", &[GameEventType::CardResolved], |ctx, event| {
        let played = your_other_play(ctx, event);
        match played {
            Some(GameEvent::CardResolved { def_id, .. })
                if def_of(Some(&*ctx.state), def_id).type_ == CardType::Unit =>
            {
                vec![summon_this()]
            }
            _ => vec![],
        }
    })
}

/// On the field: after you play a card, return this to your hand (§6.3 Bounce, R78).
fn leave() -> TriggerDef {
    TriggerDef::new("eu-striker-leave", &[GameEventType::CardResolved], |ctx, event| {
        if your_other_play(ctx, event).is_none() {
            vec![]
        } else {
            vec![bounce(json_as(json!({ "target": { "of": "self" } })))]
        }
    })
}

pub fn script() -> CardScripts {
    let base = Script {
        hand_triggers: vec![arrive()],
        triggers: vec![leave()],
        ..Script::default()
    };

    // The same script: the Radiant face differs only in what the engine reads off the catalog (its doubled
    // stats and Rush).
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// C #66 EU Striker (SPEC §8.6 row 66). R548: both triggers answer a play of yours once it has
// resolved (§10.5); neither answers the play that moved it (R401, R119; R78 reset; R70 cast).
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::testkit::*;
    use std::sync::Arc;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const STRIKER: &str = "classic-066";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4
    const BIG_FELINOR: &str = "core-043"; // (4) Unit: Cry: Destroy all non-Felinor Units.
    const STOCKPILE: &str = "core-005"; // (1) Spell
    const CN_VIRUS: &str = "core-090-1"; // (1) Spell, Cast on draw
    const GRAND: &str = "classic-072"; // Trap: counters the opponent's non-Unit plays.
    const FILLER: &str = "core-010"; // (0) Spell
    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy target Unit.
    const LOCKDOWN: &str = "classic-084"; // After a permanent is played, Lock its zone.

    use crate::js;

    fn summoned(s: &Scenario, id: &str) -> Vec<GameEvent> {
        s.last_events()
            .iter()
            .filter(|event| matches!(event, GameEvent::Summoned { instance_id, .. } if instance_id == id))
            .cloned()
            .collect()
    }

    fn unit_id(s: &Scenario, player: PlayerId, lane: i32) -> Option<String> {
        s.unit(player, lane).map(|unit| unit.id.clone())
    }

    fn unit_def(s: &Scenario, player: PlayerId, lane: i32) -> Option<String> {
        s.unit(player, lane).map(|unit| unit.def_id.clone())
    }

    fn held(s: &Scenario, player: PlayerId, def_id: &str) -> Vec<CardInstance> {
        s.hand(player).iter().filter(|card| card.def_id == def_id).cloned().collect()
    }

    mod c_66_eu_striker {
        use super::*;

        #[test]
        fn is_a_2_5_4_human_unit_10_8_rush_radiant_a_hand_trigger_and_a_field_trigger_on_card_resolved() {
            crate::register_all();
            let def = js(&crate::card_def(ID));
            assert_eq!(def["cost"], 2);
            assert_eq!(def["tags"], json!(["Human"]));
            assert_eq!(
                json!([def["base"]["attack"], def["base"]["health"], def["radiant"]["attack"], def["radiant"]["health"]]),
                json!([5, 4, 10, 8]),
            );
            assert_eq!(def["base"]["keywords"], json!([]));
            assert_eq!(def["radiant"]["keywords"], json!([{ "kind": "Rush" }]));
            assert!(def["params"].is_null());
            let scripts = script();
            assert_eq!(
                scripts.base.hand_triggers.iter().map(|trigger| js(&trigger.on)).collect::<Vec<Value>>(),
                vec![json!(["cardResolved"])],
            );
            assert_eq!(
                scripts.base.triggers.iter().map(|trigger| js(&trigger.on)).collect::<Vec<Value>>(),
                vec![json!(["cardResolved"])],
            );
            // The radiant face is the base script itself, so its triggers are the very same closures.
            assert_eq!(scripts.radiant.hand_triggers.len(), scripts.base.hand_triggers.len());
            assert_eq!(scripts.radiant.triggers.len(), scripts.base.triggers.len());
            for (radiant, base) in scripts.radiant.hand_triggers.iter().zip(&scripts.base.hand_triggers) {
                assert_eq!(radiant.id, base.id);
                assert!(Arc::ptr_eq(&radiant.run, &base.run));
            }
            for (radiant, base) in scripts.radiant.triggers.iter().zip(&scripts.base.triggers) {
                assert_eq!(radiant.id, base.id);
                assert!(Arc::ptr_eq(&radiant.run, &base.run));
            }
        }

        mod base {
            use super::*;

            #[test]
            fn r548_from_your_hand_after_you_play_a_unit_it_is_summoned_into_your_leftmost_open_zone_no_cry() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [STRIKER, VANILLA, FILLER] } }));
                let striker = s.card(STRIKER).clone();
                s.play(VANILLA, json!({}));
                assert_eq!(unit_def(&s, P1, 1), Some(VANILLA.to_string()));
                assert_eq!(unit_id(&s, P1, 2), Some(striker.id.clone()));
                assert_eq!(summoned(&s, &striker.id).len(), 1);
                // A summon is no play.
                assert!(!s.last_events().iter().any(
                    |event| matches!(event, GameEvent::CardPlayed { instance_id, .. } if *instance_id == striker.id)
                ));
            }

            #[test]
            fn r64_the_leftmost_open_zone_a_gap_to_the_left_of_the_played_unit_included() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [STRIKER, VANILLA, FILLER], "field": [{ "def": VANILLA, "lane": 2 }] },
                }));
                let striker = s.card(STRIKER).clone();
                let played = held(&s, P1, VANILLA).into_iter().next().expect("setup");
                s.play(&played, json!({ "zone": 4 }));
                assert_eq!(unit_id(&s, P1, 1), Some(striker.id.clone()));
                assert_eq!(unit_id(&s, P1, 4), Some(played.id.clone()));
            }

            #[test]
            fn r64_a_locked_zone_is_passed_over_c_84_lockdowns_lock_on_an_emptied_lane_1_sends_it_to_lane_2() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [STRIKER, VANILLA, HIT_JOB, VANILLA], "backrow": [LOCKDOWN], "mana": 9 },
                }));
                let striker = s.card(STRIKER).clone();
                let vanillas = held(&s, P1, VANILLA);
                let (Some(first), Some(second)) = (vanillas.first().cloned(), vanillas.get(1).cloned()) else {
                    panic!("setup");
                };
                // The first Vanilla's lane Locks; the Striker it summons is no play and takes lane 2, unlocked.
                s.play(&first, json!({ "zone": 1 }));
                assert_eq!(unit_id(&s, P1, 2), Some(striker.id.clone()));
                // Hit Job clears lane 1 (still Locked) and bounces the Striker; the next Unit summons it again.
                s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": first.id }] }));
                s.expect_in_zone(&striker, "hand");
                s.play(&second, json!({ "zone": 4 }));
                assert!(s.unit(P1, 1).is_none());
                assert_eq!(unit_id(&s, P1, 2), Some(striker.id.clone()));
            }

            #[test]
            fn s4_1_it_is_summoning_sick_it_cannot_attack_the_turn_it_arrives() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [STRIKER, VANILLA, FILLER] },
                    "p2": { "field": [VANILLA], "hand": [FILLER] },
                }));
                s.play(VANILLA, json!({}));
                let striker = s.card(STRIKER).clone();
                let target = s.unit(P2, 1);
                s.expect_refused_with(
                    |s| match &target {
                        Some(enemy) => s.attack(&striker, enemy),
                        None => s.attack(&striker, "hero"),
                    },
                    "summoning sick",
                );
            }

            #[test]
            fn r548_after_the_unit_resolves_its_cry_happens_first_so_big_felinors_sweep_does_not_reach_it() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [STRIKER, BIG_FELINOR, FILLER] } }));
                s.play(BIG_FELINOR, json!({}));
                s.expect_in_zone(STRIKER, "field");
                assert_eq!(unit_def(&s, P1, 2), Some(STRIKER.to_string()));
            }

            #[test]
            fn r64_with_no_open_unit_zone_it_stays_in_your_hand() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [STRIKER, VANILLA, FILLER], "field": [VANILLA, VANILLA, VANILLA, VANILLA] },
                }));
                s.play(VANILLA, json!({}));
                s.expect_in_zone(STRIKER, "hand");
            }

            #[test]
            fn r548_on_the_field_after_you_play_any_card_it_returns_to_your_hand_reset_r78() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": {
                        "hand": [STOCKPILE, FILLER],
                        "field": [{ "def": STRIKER, "damage": 2 }],
                        "library": [VANILLA, VANILLA],
                    },
                }));
                s.expect_stats(STRIKER, json!({ "health": 2 }));
                s.play(STOCKPILE, json!({}));
                let striker = s.card(STRIKER).clone();
                s.expect_in_zone(&striker, "hand");
                assert_eq!(striker.damage, 0);
                s.expect_events(json!(["cardResolved", "bounced"]));
            }

            #[test]
            fn r401_r548_the_unit_that_summons_it_doesnt_bounce_it() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [STRIKER, VANILLA, FILLER] } }));
                s.play(VANILLA, json!({}));
                s.expect_in_zone(STRIKER, "field");
            }

            #[test]
            fn r401_r548_the_card_that_bounces_it_doesnt_summon_it_back_a_unit_play_returns_it_and_it_stays_in_hand() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [VANILLA, FILLER, VANILLA], "field": [STRIKER] } }));
                let vanillas = held(&s, P1, VANILLA);
                let (Some(first), Some(second)) = (vanillas.first().cloned(), vanillas.get(1).cloned()) else {
                    panic!("setup");
                };
                s.play(&first, json!({}));
                s.expect_in_zone(STRIKER, "hand");
                // A Spell played next is no Unit: it stays in hand.
                s.play(FILLER, json!({}));
                s.expect_in_zone(STRIKER, "hand");
                // The next Unit you play summons it again.
                s.play(&second, json!({}));
                s.expect_in_zone(STRIKER, "field");
            }

            #[test]
            fn r401_r548_a_unit_play_bounces_the_one_on_the_field_and_summons_the_one_in_hand_and_neither_comes_back() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [STRIKER, VANILLA, FILLER], "field": [STRIKER] } }));
                let on_field = s.unit(P1, 1);
                let in_hand = held(&s, P1, STRIKER).into_iter().next();
                let (Some(on_field), Some(in_hand)) = (on_field, in_hand) else {
                    panic!("setup");
                };
                s.play(VANILLA, json!({}));
                s.expect_in_zone(&on_field, "hand");
                s.expect_in_zone(&in_hand, "field");
            }

            #[test]
            fn r119_r401_its_own_play_doesnt_bounce_it() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [STRIKER, FILLER] } }));
                s.play(STRIKER, json!({}));
                s.expect_in_zone(STRIKER, "field");
            }

            #[test]
            fn r70_a_cast_is_a_play_a_spell_cast_on_draw_returns_it_to_your_hand() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [STOCKPILE, FILLER], "field": [STRIKER], "library": [CN_VIRUS, VANILLA, VANILLA] },
                }));
                s.play(STOCKPILE, json!({}));
                s.expect_in_zone(STRIKER, "hand");
                assert!(s.last_events().iter().any(
                    |event| matches!(event, GameEvent::CardResolved { def_id, .. } if def_id == CN_VIRUS)
                ));
            }

            #[test]
            fn a_countered_card_is_no_play_under_the_opponents_grand_counterspell_it_stays_on_the_field() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [STOCKPILE, FILLER], "field": [STRIKER] },
                    "p2": { "backrow": [GRAND], "hand": [FILLER] },
                }));
                s.play(STOCKPILE, json!({}));
                assert!(s.last_events().iter().any(|event| matches!(event, GameEvent::Countered { .. })));
                s.expect_in_zone(STRIKER, "field");
            }

            #[test]
            fn the_opponents_plays_do_nothing_their_unit_leaves_yours_in_hand_their_card_leaves_yours_on_the_field() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [STRIKER, FILLER], "field": [STRIKER] },
                    "p2": { "hand": [VANILLA, STOCKPILE, FILLER] },
                    "active": "p2",
                }));
                s.play(VANILLA, json!({}));
                s.play(STOCKPILE, json!({}));
                assert_eq!(held(&s, P1, STRIKER).len(), 1);
                assert_eq!(unit_def(&s, P1, 1), Some(STRIKER.to_string()));
            }

            #[test]
            fn s2_4_the_hand_cap_returned_to_a_full_hand_it_burns_into_your_graveyard() {
                crate::register_all();
                let nine: Vec<&str> = (0..9).map(|_| FILLER).collect();
                let mut hand = vec![STOCKPILE];
                hand.extend(nine);
                let mut s = scenario(json!({
                    "p1": { "hand": hand, "field": [STRIKER], "library": [VANILLA, VANILLA] },
                }));
                // Stockpile leaves the hand (9 left) and draws 2 (10, then a burn); the return finds the hand full.
                s.play(STOCKPILE, json!({}));
                s.expect_in_zone(STRIKER, "graveyard");
            }

            #[test]
            fn r97_in_your_hand_the_opponents_view_never_names_it_the_summon_is_the_first_they_see_of_it() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [STRIKER, VANILLA, FILLER] } }));
                let striker = s.card(STRIKER).clone();
                assert!(!js(&s.view(P2)).to_string().contains(STRIKER));
                s.play(VANILLA, json!({}));
                let theirs = js(&s.view(P2));
                assert_eq!(theirs["opponent"]["units"][1]["defId"], STRIKER);
                let summon_event = theirs["events"]
                    .as_array()
                    .expect("the view's events")
                    .iter()
                    .find(|event| event["type"] == "summoned" && event["instanceId"] == striker.id.as_str());
                assert!(summon_event.is_some());
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn is_a_10_8_with_rush_summoned_from_your_hand_after_a_unit_and_may_attack_a_unit_at_once() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": STRIKER, "radiant": true }, VANILLA, FILLER] },
                    "p2": { "field": [VANILLA], "hand": [FILLER] },
                }));
                s.play(VANILLA, json!({}));
                let striker = s.card(STRIKER).clone();
                s.expect_stats(&striker, json!({ "attack": 10, "health": 8 }));
                assert_eq!(
                    s.stats(&striker).keywords.iter().map(|keyword| js(keyword)["kind"].clone()).collect::<Vec<Value>>(),
                    vec![json!("Rush")],
                );
                let enemy = s.unit(P2, 1).expect("no enemy");
                s.attack(&striker, &enemy);
                s.expect_in_zone(&enemy, "graveyard");
            }

            #[test]
            fn r401_r548_the_same_two_triggers_after_you_play_a_card_it_returns_to_your_hand_and_its_own_play_doesnt() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": STRIKER, "radiant": true }, STOCKPILE, FILLER], "library": [VANILLA, VANILLA] },
                }));
                s.play(STRIKER, json!({}));
                s.expect_in_zone(STRIKER, "field");
                s.play(STOCKPILE, json!({}));
                s.expect_in_zone(STRIKER, "hand");
            }
        }
    }
}
