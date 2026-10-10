//! #41 Sheepish (SPEC §8.2). Trap, cost 1, Epic.
//!   Base:    "Reveals when your opponent plays a Unit: After it resolves, transform it into a Sheep Token."
//!   Radiant: the same, "Add a Lava Golem to your hand. It costs (0)." (keeps every clause of base, §8, R277).
//!
//! TIMING (R427; rewrites R17's Sheepish half): it answers §10.5 step 7's `cardResolved`, after the Cry and
//! every Echo repeat (a cast Unit too, R70). Not `summoned`, which also covers Recruit, copies, tokens and
//! Reborn, none of them a play (R1, R61). Every condition lives in `when`, so a Spell, a backrow card or the
//! controller's own Unit leaves the trap armed (R61).
//! A Unit that left the field in its own resolution still fires it: nothing to Transform (it never reaches
//! a hand, a graveyard or a Reborn body, R83), the trap consumed, the Lava Golem still added (R120). One an
//! earlier trap took off the field is no play left to answer: it stays set (R174). Immutable (R17, R23, R33):
//! `transform` refuses it and `fireTrap` consumes the trap regardless. No glow (R662): the trap waits on an
//! event, and what a glow would tell, a Unit in the opponent's hand, is hidden (§9.1).

use jackioh_engine::catalog::def_of;
use jackioh_engine::effects::{add_to_hand, transform};
use jackioh_engine::prelude::*;
use jackioh_engine::query::left_field_since_resolved;

pub const ID: &str = "core-041";

/// §7: the Sheep Token, whose only generator is this card.
const SHEEP_TOKEN: &str = "core-t-sheep";
/// #55 Lava Golem, added by the radiant text at cost 0 (`costOverride`, R65): the declared number
/// `setCost` (R386).
const LAVA_GOLEM: &str = "core-055";

/// The two faces differ only in whether the Lava Golem comes with the Sheep.
fn sheepish(lava_golem: bool) -> TriggerDef {
    TriggerDef::new(
        if lava_golem { "sheepish-radiant" } else { "sheepish" },
        &[GameEventType::CardResolved],
        move |ctx, event| {
            let GameEvent::CardResolved {
                instance_id, permanent, ..
            } = event
            else {
                return vec![];
            };
            // Named by the event, not by a TargetSpec: nobody chose this unit, the play produced it. R427:
            // only a Unit still in play from that play is transformed; one that has left finds nothing.
            let mut effects: Vec<Effect> = if *permanent {
                vec![transform(json_as(json!({ "instanceId": instance_id, "defId": SHEEP_TOKEN })))]
            } else {
                vec![]
            };
            // R120: §8's conventions make an "Also" clause independent, so it still lands when an
            // Immutable target refused the Transform (R17, R23) and the trap is still consumed (R61).
            if lava_golem {
                let cost = param(&*ctx, "setCost");
                effects.push(add_to_hand(json_as(json!({ "defId": LAVA_GOLEM, "costOverride": cost }))));
            }
            effects
        },
    )
    .with_when(|ctx, event| {
        let GameEvent::CardResolved {
            player,
            def_id,
            permanent,
            ..
        } = event
        else {
            return false;
        };
        // §8: "your opponent". A trap never answers its own controller's play.
        if *player == ctx.controller {
            return false;
        }
        // §5.1: a Unit, so a Spell or a backrow card leaves the trap armed (R61).
        if def_of(Some(&*ctx.state), def_id).type_ != CardType::Unit {
            return false;
        }
        // R174, R427: not a Unit an earlier answer to this play has already taken off the field.
        *permanent || !left_field_since_resolved(&*ctx.state, event)
    })
}

pub fn script() -> CardScripts {
    let base_trigger = sheepish(false);
    let radiant_trigger = sheepish(true);
    CardScripts {
        base: Script {
            triggers: vec![base_trigger],
            ..Script::default()
        },
        radiant: Script {
            triggers: vec![radiant_trigger],
            ..Script::default()
        },
    }
}

// #41 Sheepish (SPEC §8.2, §5.1, §10.3, §10.5 step 7; R17, R23, R33, R61, R70, R120, R174, R427).
// Must-pass row (BUILD M4-T4 #41, as R427 rewrites it): "Opponent's Unit becomes a Sheep after its Cry
// resolves; trap consumed; Immutable target → consumed with no effect; radiant adds 0-cost Lava Golem."
// "After its Cry" is proved with an observable Cry: #53 Reno heals a p1 hero left at 10 up to 30.
#[cfg(test)]
mod tests {
    use super::script;
    use jackioh_engine::testkit::*;

    /// Registers the catalog first: the engine's testkit cannot name the cards crate.
    fn scn(opts: Value) -> Scenario {
        crate::register_all();
        scenario(opts)
    }

    /// p1 is active with `card` to play; p2 holds Sheepish face-down in backrow lane 1.
    /// #16 Hit Job rides along so the turn does not auto-end into p2's turn after the play (R82).
    fn trap_set(card: Value, radiant_trap: bool) -> Scenario {
        scn(json!({
            "seed": "sheepish",
            "p1": { "hand": [card, "core-016"], "health": 10, "mana": 10 },
            "p2": { "backrow": [{ "def": "core-041", "radiant": radiant_trap, "lane": 1 }], "health": 30 },
        }))
    }

    /// A Radiant #19 Midrange Menace: "Taunt, Immutable" (§8.1 row 19), the Immutable Unit to play.
    fn immutable() -> Value {
        json!({ "def": "core-019", "radiant": true })
    }

    fn fired_trap(events: &[GameEvent]) -> bool {
        events.iter().any(|event| matches!(event, GameEvent::TrapFired { .. }))
    }

    fn transformed(events: &[GameEvent]) -> bool {
        events.iter().any(|event| matches!(event, GameEvent::Transformed { .. }))
    }

    fn unit_def(s: &Scenario, seat: &str, lane: i32) -> Option<String> {
        s.unit(seat, lane).map(|card| card.def_id)
    }

    mod base {
        use super::*;

        #[test]
        fn r427_transforms_the_opponents_played_unit_into_a_sheep_token_after_its_cry_resolves_so_the_cry_is_kept() {
            let mut s = trap_set(json!("core-053"), false);
            s.play("core-053", json!({ "zone": 1 }));

            // §10.5 step 5 runs the Cry, step 7 emits `cardResolved`, and only then does the trap fire.
            s.expect_events(json!(["cardPlayed", "healed", "cardResolved", "trapFired", "transformed"]));
            assert_eq!(unit_def(&s, "p1", 1).as_deref(), Some("core-t-sheep"));
            // #53 Reno's Cry healed p1's hero to 30 before the Unit became a Sheep.
            s.expect_health("p1", 30);
        }

        #[test]
        fn r427_what_the_cry_made_stays_15s_rush_token_is_on_the_field_beside_the_sheep() {
            let mut s = trap_set(json!("core-015"), false);
            s.play("core-015", json!({ "zone": 1 }));

            assert_eq!(unit_def(&s, "p1", 1).as_deref(), Some("core-t-sheep"));
            assert_eq!(unit_def(&s, "p1", 2).as_deref(), Some("core-t-rush"));
        }

        #[test]
        fn sec7_the_replacement_is_the_1_1_sheep_token_in_the_played_units_own_zone_sec6_3_transform() {
            let mut s = trap_set(json!("core-053"), false);
            s.play("core-053", json!({ "zone": 1 }));

            s.expect_in_zone("core-t-sheep", "field");
            s.expect_stats("core-t-sheep", json!({ "attack": 1, "health": 1, "maxHealth": 1 }));
        }

        #[test]
        fn sec5_1_the_trap_is_consumed_it_leaves_the_backrow_for_its_owners_graveyard() {
            let mut s = trap_set(json!("core-053"), false);
            s.play("core-053", json!({ "zone": 1 }));

            s.expect_in_zone("core-041", "graveyard");
            assert!(s.backrow("p2", 1).is_none());
        }

        #[test]
        fn r17_an_immutable_target_still_fires_the_trap_which_is_consumed_with_no_effect_r23() {
            // A Radiant #19 Midrange Menace is Immutable, and R23 makes Immutable refuse Transform on the
            // card itself.
            let mut s = trap_set(immutable(), false);
            s.play("core-019", json!({ "zone": 1 }));

            assert!(fired_trap(s.events()));
            assert!(!transformed(s.events()));
            assert_eq!(unit_def(&s, "p1", 1).as_deref(), Some("core-019"));
            s.expect_in_zone("core-041", "graveyard");
        }

        #[test]
        fn r61_a_spell_leaves_the_trap_armed_and_face_down_the_condition_is_a_when_not_an_empty_run() {
            let mut s = scn(json!({
                "seed": "sheepish",
                "p1": { "hand": ["core-044", "core-016"], "mana": 10 },
                "p2": { "backrow": [{ "def": "core-041", "lane": 1 }], "health": 30 },
            }));
            s.play("core-044", json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            // The spell resolved, so the play really happened and the trap really saw the event.
            s.expect_health("p2", 26);
            assert!(!fired_trap(s.events()));
            s.expect_in_zone("core-041", "field");
        }

        #[test]
        fn sec8_fires_only_on_the_opponents_play_the_controllers_own_unit_leaves_it_armed() {
            let mut s = scn(json!({
                "seed": "sheepish",
                "active": "p2",
                "turn": 2,
                "p2": {
                    "hand": ["core-053", "core-016"],
                    "backrow": [{ "def": "core-041", "lane": 1 }],
                    "health": 10,
                    "mana": 10,
                },
            }));
            s.play("core-053", json!({ "zone": 1 }));

            assert!(!fired_trap(s.events()));
            s.expect_in_zone("core-041", "field");
            assert_eq!(unit_def(&s, "p2", 1).as_deref(), Some("core-053"));
            // Reno's own Cry ran, because nothing interrupted it.
            s.expect_health("p2", 30);
        }

        #[test]
        fn r427_r174_a_unit_an_earlier_trap_answering_the_same_play_has_taken_off_the_field_is_no_play_left_to_answer_sheepish_stays_set()
        {
            // p2's backrow: Bear Honeypot in lane 1, Sheepish in lane 2. Both answer the same `cardResolved`
            // in lane order (R68): the Honeypot's two Rush Tokens attack the played 1/1 and kill it, so by the
            // time Sheepish is offered the play its Unit has been taken off the field after it resolved.
            let mut s = scn(json!({
                "seed": "sheepish-honeypot",
                "p1": { "hand": ["core-086", "core-016"], "mana": 10 },
                "p2": {
                    "backrow": [
                        { "def": "core-060", "lane": 1 },
                        { "def": "core-041", "radiant": true, "lane": 2 },
                    ],
                },
            }));
            let mrow = s.card("core-086").clone();
            s.play(&mrow, json!({ "zone": 1 }));

            let fired: Vec<String> = s
                .events()
                .iter()
                .filter_map(|event| match event {
                    GameEvent::TrapFired { def_id, .. } => Some(def_id.clone()),
                    _ => None,
                })
                .collect();
            assert_eq!(fired, vec!["core-060".to_string()]);
            assert!(!transformed(s.events()));
            s.expect_in_zone(&mrow, "graveyard");
            // Still face-down in its zone, armed for the next Unit, and no Lava Golem came of it.
            assert_eq!(s.backrow("p2", 2).map(|card| card.def_id).as_deref(), Some("core-041"));
            assert!(s.backrow("p2", 2).and_then(|card| card.face_up).is_none());
            assert!(!s.hand("p2").iter().any(|card| card.def_id == "core-055"));
        }

        #[test]
        fn r427_a_unit_that_left_the_field_in_its_own_resolution_still_fires_sheepish_which_is_consumed_and_transforms_nothing()
        {
            // A Radiant #52 Silly Silas played into lane 5 and rotating right crosses to the other side in
            // its own Cry, and its Radiant face bounces what crosses: it is in p1's hand as the play resolves.
            let mut s = scn(json!({
                "seed": "sheepish-silas",
                "p1": { "hand": [{ "def": "core-052", "radiant": true }, "core-016"], "mana": 10 },
                // Lane 3: the rotation moves the trap a step along p2's own row, and it stays p2's.
                "p2": { "backrow": [{ "def": "core-041", "radiant": true, "lane": 3 }] },
            }));
            let silas = s.card("core-052").clone();
            s.play(&silas, json!({ "zone": 5, "modes": ["right"] }));

            s.expect_in_zone(&silas, "hand");
            assert!(fired_trap(s.events()));
            assert!(!transformed(s.events()));
            s.expect_in_zone("core-041", "graveyard");
            // R120: the Radiant face's Lava Golem is its own clause, and still lands.
            assert!(
                s.hand("p2")
                    .iter()
                    .any(|card| card.def_id == "core-055" && card.cost_override == Some(0))
            );
        }

        #[test]
        fn r427_r113_a_cry_that_asks_the_unit_becomes_a_sheep_only_once_the_answer_has_resolved_the_cry_across_a_round_trip()
        {
            // #7 Jewelosco Scarab's Cry is a Discover: the play pauses at step 5 with the trap unfired.
            let mut s = trap_set(json!("core-007"), false);
            s.play("core-007", json!({ "zone": 1 }));
            assert_eq!(s.state().pending.as_ref().map(|pending| pending.kind), Some(PromptKind::Discover));
            assert!(!fired_trap(s.events()));
            assert_eq!(unit_def(&s, "p1", 1).as_deref(), Some("core-007"));

            // §9.3: the paused game is plain JSON.
            let round_trip: GameState =
                serde_json::from_value(serde_json::to_value(s.state()).expect("the state is JSON"))
                    .expect("the state reads back");
            assert_eq!(&round_trip, s.state());

            let offered = s
                .state()
                .pending
                .as_ref()
                .and_then(|pending| pending.options.first())
                .map(|option| option.key.clone());
            assert!(offered.is_some());
            s.answer(json!(offered.unwrap_or_default()));

            s.expect_events(json!(["promptAnswered", "addedToHand", "cardResolved", "trapFired", "transformed"]));
            assert_eq!(unit_def(&s, "p1", 1).as_deref(), Some("core-t-sheep"));
            s.expect_in_zone("core-041", "graveyard");
        }

        #[test]
        fn r81_r427_the_trap_declares_no_play_time_choice_and_watches_card_resolved() {
            let base = script().base;
            assert!(base.targets.is_empty());
            assert!(base.modes.is_empty());
            let watched: Vec<Vec<GameEventType>> = base.triggers.iter().map(|trigger| trigger.on.clone()).collect();
            assert_eq!(watched, vec![vec![GameEventType::CardResolved]]);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn transforms_the_played_unit_after_its_cry_and_adds_a_lava_golem_costing_0_to_the_trap_controllers_hand() {
            let mut s = trap_set(json!("core-053"), true);
            s.play("core-053", json!({ "zone": 1 }));

            assert_eq!(unit_def(&s, "p1", 1).as_deref(), Some("core-t-sheep"));
            s.expect_health("p1", 30);

            let golem = s.hand("p2").into_iter().find(|card| card.def_id == "core-055");
            let golem = golem.expect("a Lava Golem in p2's hand");
            // R65: a `costOverride` of 0 is what "It costs (0)" means, and it survives every zone.
            assert_eq!(golem.cost_override, Some(0));
        }

        #[test]
        fn r386_a_degrade_makes_the_lava_golem_cost_1_and_an_upgrade_finds_the_cost_at_its_floor_of_0() {
            let mut s = trap_set(json!("core-053"), true);
            assert!(!crate::can_upgrade_number(&s, "core-041", "setCost"));
            assert_eq!(crate::degrade_number(&mut s, "core-041", "setCost"), 1);
            s.play("core-053", json!({ "zone": 1 }));

            let golem = s.hand("p2").into_iter().find(|card| card.def_id == "core-055");
            assert_eq!(golem.and_then(|card| card.cost_override), Some(1));
        }

        #[test]
        fn r427_the_radiant_trap_is_consumed_too_once_and_the_sheep_replaces_the_unit() {
            let mut s = trap_set(json!("core-053"), true);
            s.play("core-053", json!({ "zone": 1 }));

            s.expect_events(json!(["cardPlayed", "cardResolved", "trapFired", "transformed", "addedToHand"]));
            s.expect_in_zone("core-041", "graveyard");
            assert_eq!(s.hand("p2").iter().filter(|card| card.def_id == "core-055").count(), 1);
        }

        #[test]
        fn r120_an_also_clause_stands_on_its_own_an_immutable_target_refuses_the_transform_and_the_rest_of_the_text_still_resolves()
        {
            let mut s = trap_set(immutable(), true);
            s.play("core-019", json!({ "zone": 1 }));

            assert!(fired_trap(s.events()));
            assert!(!transformed(s.events()));
            assert_eq!(unit_def(&s, "p1", 1).as_deref(), Some("core-019"));
            assert!(
                s.hand("p2")
                    .iter()
                    .any(|card| card.def_id == "core-055" && card.cost_override == Some(0))
            );
            s.expect_in_zone("core-041", "graveyard");
        }

        #[test]
        fn r61_the_radiant_face_arms_on_the_same_condition_a_spell_does_not_fire_it() {
            let mut s = scn(json!({
                "seed": "sheepish",
                "p1": { "hand": ["core-044", "core-016"], "mana": 10 },
                "p2": { "backrow": [{ "def": "core-041", "radiant": true, "lane": 1 }], "health": 30 },
            }));
            s.play("core-044", json!({ "targets": [{ "pick": "hero", "player": "p2" }] }));

            assert!(!fired_trap(s.events()));
            assert!(!s.hand("p2").iter().any(|card| card.def_id == "core-055"));
            s.expect_in_zone("core-041", "field");
        }

        #[test]
        fn r74_r427_both_faces_watch_the_same_event_so_the_radiant_text_changes_what_fires_not_when() {
            let radiant = script().radiant;
            let watched: Vec<Vec<GameEventType>> =
                radiant.triggers.iter().map(|trigger| trigger.on.clone()).collect();
            assert_eq!(watched, vec![vec![GameEventType::CardResolved]]);
        }
    }

    mod sheepish_never_glows_r662_nothing_on_the_board_decides_it {
        use super::*;

        fn armed_on_either_turn_it_never_lights_up(radiant: bool) {
            let face = if radiant { "radiant" } else { "base" };
            let scripts = script();
            assert!(scripts.base.condition_met.is_none());
            assert!(scripts.radiant.condition_met.is_none());
            let mine = scn(json!({
                "seed": format!("r662-041-{face}-mine"),
                "p1": { "backrow": [{ "def": "core-041", "radiant": radiant }], "hand": ["core-010"] },
            }));
            assert!(!backrow_glows(&mine, 1, PlayerId::P1));
            let mut theirs = scn(json!({
                "seed": format!("r662-041-{face}-theirs"),
                "active": "p2",
                "p1": { "backrow": [{ "def": "core-041", "radiant": radiant }] },
                "p2": { "hand": ["core-008", "core-010"] },
            }));
            assert!(!backrow_glows(&theirs, 1, PlayerId::P1));

            // And it still fires on the Unit: the missing glow hides nothing the trap would do.
            theirs.play("core-008", json!({}));
            assert_eq!(unit_def(&theirs, "p2", 1).as_deref(), Some("core-t-sheep"));
        }

        #[test]
        fn r662_base_armed_on_either_turn_with_the_opponent_holding_a_unit_or_not_it_never_lights_up() {
            armed_on_either_turn_it_never_lights_up(false);
        }

        #[test]
        fn r662_radiant_armed_on_either_turn_with_the_opponent_holding_a_unit_or_not_it_never_lights_up() {
            armed_on_either_turn_it_never_lights_up(true);
        }
    }
}
