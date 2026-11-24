//! SPEC §8.11 Training Grounds #50 Legacy: Glowy Jelly Bean,
//! the Legacy card of Core #26 Glowy Jelly Bean (`core-026`), a card of the held Training Grounds set (R1420,
//! R2194000), built by #2195.
//!
//! The code is a copy of the Core card's at the foundation's commit, not a re-export, so a later Core
//! patch does not move it (R2195003); it is named `Legacy: <the Core card's name>`, tagged Legacy
//! (R2511000), and is its own card, which a deck holds beside the Core card (R2511001). The Core
//! file's notes, as they stood:
//!
//! #26 Glowy Jelly Bean (SPEC §8.2): "Choose a card in your hand; it becomes Radiant", radiant
//! "Choose 2" — a Radiant cell that changes only a number changes only that number (§8 Conventions),
//! so the radiant face is the same effect, taken twice.
//!
//! The pick is a DECLARED `hand` choice: it travels in the play action's `targets` and never pauses
//! resolution (R81). R90: a hand declaration offers only the chooser's own hand (§9.1), never the bean
//! itself, and never the same card twice. A play stays legal with the answers that exist, so with one
//! other card in hand ("Radiant takes both cards it can", §8.2 Engine) the second `setRadiant`
//! fizzles (§8 Conventions) and the spell still counts as played.
//!
//! R74 makes the radiant flag the whole model, so the chosen card's stats and text swap while it sits
//! in hand (§5.2); a card that is already Radiant is untouched (§6.3 Make Radiant).
//!
//! Issue #4665 patched the Legacy card itself, not the Core one (R4665006): (2), from (3), and "Make
//! a card in your hand Radiant. It costs ({surcharge}) more." (base) and "Make a card in your hand
//! Radiant." (Radiant). The base face's pick takes `setCostMod` of the declared `surcharge` (1, R386)
//! after it is made Radiant, as #72 Reminisce prices a chosen hand card; the Radiant face picks one
//! card, from two, and adds no surcharge. `script_v0_6_12c` keeps the copy as it was (R767000).

use jackioh_engine::effects::set_cost_mod;
use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-050";

/// `picks` hand cards made Radiant; with `surcharges`, each then costs the declared `surcharge` more
/// (R4665006).
fn glowy_jelly_bean(picks: i32, surcharges: bool) -> Script {
    Script {
        targets: vec![TargetDecl::hand(picks, picks, json!({ "of": ["hand"] }))],
        cry: Some(hook(move |ctx| {
            let surcharge = if surcharges { param(&*ctx, "surcharge") } else { 0 };
            let mut effects = Vec::new();
            for index in 0..picks {
                let chosen = json!({ "of": "chosen", "index": index });
                effects.push(set_radiant(json_as(json!({ "target": chosen }))));
                if surcharge != 0 {
                    effects.push(set_cost_mod(json_as(json!({ "target": chosen, "amount": surcharge }))));
                }
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: glowy_jelly_bean(1, true),
        radiant: glowy_jelly_bean(1, false),
    }
}

/// R767000, R4665006: through v0.6.12c the copy of Core #26 as it was: one pick on the base face,
/// two on the Radiant one, and no surcharge; `KEPT_SCRIPTS` folds those replays under it.
pub fn script_v0_6_12c() -> CardScripts {
    CardScripts {
        base: glowy_jelly_bean(1, false),
        radiant: glowy_jelly_bean(2, false),
    }
}

// #26 Glowy Jelly Bean (SPEC §8.2, BUILD M4-T4 row 26) (R81).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    /// A Unit whose faces differ in their numbers alone.
    const TWO_FACED: &str = "stand-in (3) 6/8 R12/16";

    /// The ids of p1's Radiant hand cards, which is the whole of what this card does (R74).
    fn radiant_hand(state: &GameState) -> Vec<String> {
        state
            .players
            .p1
            .hand
            .iter()
            .filter(|card| card.radiant)
            .map(|card| card.id.clone())
            .collect()
    }

    fn hand_id_of(state: &GameState, def_id: &str) -> String {
        match state.players.p1.hand.iter().find(|held| held.def_id == def_id) {
            Some(card) => card.id.clone(),
            None => panic!("no {def_id} in p1's hand"),
        }
    }

    /// It costs 2 on both faces since issue #4665 (3 before); R74 leaves a cost alone when a card
    /// becomes Radiant (§5.2), and the base face's surcharge is the chosen card's `costMod`.
    fn glowy(hand: Value) -> Scenario {
        scenario(json!({ "p1": { "hand": hand, "mana": 3 } }))
    }

    mod n26_glowy_jelly_bean_base {
        use super::*;

        #[test]
        fn r81_flags_the_hand_card_the_play_chose_with_no_prompt_opened() {
            crate::register_all();
            let mut s = glowy(json!(["traininggrounds-050", "core-005", "core-016"]));
            let chosen = hand_id_of(s.state(), "core-005");
            let other = hand_id_of(s.state(), "core-016");

            s.play("traininggrounds-050", json!({ "targets": [{ "pick": "instance", "instanceId": chosen }] }));

            // R81: the pick traveled in the play action, so resolution never paused.
            assert!(s.state().pending.is_none());
            assert_eq!(radiant_hand(s.state()), vec![chosen.clone()]);
            assert!(!s.card(&other).radiant);
            s.expect_in_zone("traininggrounds-050", "graveyard")
                .expect_events(json!(["cardPlayed", "radiantSet"]));
        }

        #[test]
        fn r74_the_flag_is_the_whole_model_so_the_card_s_stats_swap_while_it_sits_in_hand_s5_2() {
            crate::register_all();
            let mut s = glowy(json!(["traininggrounds-050", TWO_FACED]));
            let chosen = hand_id_of(s.state(), TWO_FACED);

            s.play("traininggrounds-050", json!({ "targets": [{ "pick": "instance", "instanceId": chosen }] }));

            // The 6/8 is a 12/16 now: no card id changed, only the flag.
            s.expect_stats(&chosen, json!({ "attack": 12, "maxHealth": 16 }));
            assert_eq!(s.card(&chosen).def_id, TWO_FACED);
            assert_eq!(s.card(&chosen).zone.z(), ZoneName::Hand);
        }

        #[test]
        fn s6_3_leaves_a_card_that_is_already_radiant_alone_nothing_in_core_un_sets_the_flag() {
            crate::register_all();
            let mut s = glowy(json!(["traininggrounds-050", { "def": "core-005", "radiant": true }]));
            let chosen = hand_id_of(s.state(), "core-005");

            s.play("traininggrounds-050", json!({ "targets": [{ "pick": "instance", "instanceId": chosen }] }));

            assert_eq!(radiant_hand(s.state()), vec![chosen.clone()]);
            // It was already Radiant, so the flag is unchanged; the cue still goes out, because the card is
            // hidden from the opponent and its absence would tell them its face (R177, R97).
            assert_eq!(
                s.last_events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::RadiantSet { .. }))
                    .count(),
                1
            );
        }

        /// BUILD M13 TG 50 "never itself" (R90): no legal play of the Bean names the Bean, and a play
        /// that names it is refused.
        #[test]
        fn r90_the_bean_is_never_its_own_pick() {
            crate::register_all();
            let mut s = glowy(json!(["traininggrounds-050", "core-005"]));
            let bean = hand_id_of(s.state(), "traininggrounds-050");
            let other = hand_id_of(s.state(), "core-005");

            let plays: Vec<Value> = legal_actions(s.state(), PlayerId::P1)
                .iter()
                .map(crate::js)
                .filter(|action| action["type"] == json!("play") && action["instanceId"] == json!(bean))
                .collect();
            assert!(!plays.is_empty());
            let picks: Vec<Value> = plays
                .iter()
                .flat_map(|play| play["targets"].as_array().cloned().unwrap_or_default())
                .map(|target| target["instanceId"].clone())
                .collect();
            assert!(picks.contains(&json!(other)), "{picks:?}");
            assert!(!picks.contains(&json!(bean)), "{picks:?}");

            s.expect_refused(|s| s.play("traininggrounds-050", json!({ "targets": [{ "pick": "instance", "instanceId": bean }] })));
            assert!(radiant_hand(s.state()).is_empty());
        }

        #[test]
        fn s8_conventions_an_empty_hand_fizzles_the_pick_and_the_spell_still_counts_as_played() {
            crate::register_all();
            let mut s = glowy(json!(["traininggrounds-050"]));

            s.play("traininggrounds-050", json!({}));

            assert!(s.state().pending.is_none());
            assert!(radiant_hand(s.state()).is_empty());
            // "The spell still counts as played": the `cardPlayed` event and the game counter, not
            // `turnLog`, which an empty hand's auto-end-of-turn has already cleared by now.
            assert_eq!(
                s.events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::CardPlayed { .. }))
                    .count(),
                1
            );
            assert_eq!(s.state().counters.played, 1);
            s.expect_in_zone("traininggrounds-050", "graveyard");
        }
    }

    /// R4665006: the base face's surcharge, the declared `surcharge` on the chosen card.
    mod surcharge_r4665006 {
        use super::*;

        #[test]
        fn r4665006_the_card_it_makes_radiant_costs_1_more() {
            crate::register_all();
            let mut s = glowy(json!(["traininggrounds-050", "core-005", "core-016"]));
            let chosen = hand_id_of(s.state(), "core-005");
            let other = hand_id_of(s.state(), "core-016");

            s.play("traininggrounds-050", json!({ "targets": [{ "pick": "instance", "instanceId": chosen }] }));

            assert_eq!(radiant_hand(s.state()), vec![chosen.clone()]);
            assert_eq!(s.card(&chosen).cost_mod, 1);
            assert_eq!(s.card(&other).cost_mod, 0);
        }

        /// R386: the surcharge reads through `param`: set to 2, the chosen card costs 2 more.
        #[test]
        fn r386_r4665006_the_surcharge_is_declared() {
            crate::register_all();
            let mut s = glowy(json!(["traininggrounds-050", "core-005"]));
            let chosen = hand_id_of(s.state(), "core-005");
            set_param(s.card_mut("traininggrounds-050"), "surcharge", 2);

            s.play("traininggrounds-050", json!({ "targets": [{ "pick": "instance", "instanceId": chosen }] }));

            assert_eq!(radiant_hand(s.state()), vec![chosen.clone()]);
            assert_eq!(s.card(&chosen).cost_mod, 2);
        }
    }

    mod n26_glowy_jelly_bean_radiant {
        use super::*;

        /// R4665006: the Radiant face picks one card since issue #4665 (two before) and adds no surcharge.
        #[test]
        fn r81_r4665006_chooses_1_and_adds_no_surcharge() {
            crate::register_all();
            let mut s = glowy(json!([{ "def": "traininggrounds-050", "radiant": true }, "core-005", "core-016"]));
            let chosen = hand_id_of(s.state(), "core-005");
            let other = hand_id_of(s.state(), "core-016");

            s.play("traininggrounds-050", json!({ "targets": [{ "pick": "instance", "instanceId": chosen }] }));

            assert!(s.state().pending.is_none());
            assert_eq!(radiant_hand(s.state()), vec![chosen.clone()]);
            assert!(!s.card(&other).radiant);
            assert_eq!(s.card(&chosen).cost_mod, 0);
        }
    }

    /// R767000, R4665006: the scripts kept for v0.6.12c.
    mod kept_v0_6_12c {
        use super::*;

        #[test]
        fn r767000_r4665006_the_kept_radiant_face_chooses_2_and_neither_face_adds_a_surcharge() {
            crate::under_kept(super::super::ID, super::super::script_v0_6_12c(), || {
                let mut s = glowy(json!([{ "def": "traininggrounds-050", "radiant": true }, "core-005", "core-016"]));
                let first = hand_id_of(s.state(), "core-005");
                let second = hand_id_of(s.state(), "core-016");
                s.play(
                    "traininggrounds-050",
                    json!({
                        "targets": [
                            { "pick": "instance", "instanceId": first },
                            { "pick": "instance", "instanceId": second },
                        ],
                    }),
                );
                let mut flagged = radiant_hand(s.state());
                flagged.sort();
                let mut wanted = vec![first.clone(), second.clone()];
                wanted.sort();
                assert_eq!(flagged, wanted);
                assert_eq!(s.card(&first).cost_mod, 0);

                let mut s = glowy(json!(["traininggrounds-050", "core-005"]));
                let chosen = hand_id_of(s.state(), "core-005");
                s.play("traininggrounds-050", json!({ "targets": [{ "pick": "instance", "instanceId": chosen }] }));
                assert_eq!(s.card(&chosen).cost_mod, 0);
            });
        }
    }

    /// R2195003: a Core patch does not move the snapshot: issue #4665 gave Core #26 Tradable and
    /// Flashback and the Legacy card neither, and patched the Legacy card alone (R4665006).
    #[test]
    fn r2195003_r4665006_core_26_s_tradable_and_flashback_are_not_the_legacy_card_s() {
        let core = crate::card_def("core-026");
        let legacy = crate::card_def(super::ID);
        for kind in [KeywordKind::Tradable, KeywordKind::Flashback] {
            for (core_face, legacy_face) in [(&core.base, &legacy.base), (&core.radiant, &legacy.radiant)] {
                assert!(core_face.keywords.iter().any(|keyword| keyword.kind() == kind), "{kind:?}");
                assert!(!legacy_face.keywords.iter().any(|keyword| keyword.kind() == kind), "{kind:?}");
            }
        }
        assert_eq!(
            (core.cost, legacy.cost),
            (jackioh_engine::CardCost::Fixed(3), jackioh_engine::CardCost::Fixed(2))
        );
    }
}
