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

use jackioh_engine::prelude::*;

pub const ID: &str = "traininggrounds-050";

/// The two faces differ only in how many cards the play picks.
fn glowy_jelly_bean(picks: i32) -> Script {
    Script {
        targets: vec![TargetDecl::hand(picks, picks, json!({ "of": ["hand"] }))],
        cry: Some(hook(move |_ctx| {
            (0..picks)
                .map(|index| set_radiant(json_as(json!({ "target": { "of": "chosen", "index": index } }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: glowy_jelly_bean(1),
        radiant: glowy_jelly_bean(2),
    }
}

// #26 Glowy Jelly Bean (SPEC §8.2, BUILD M4-T4 row 26): "Chosen hand card gets radiant flag,
// picked as part of the play rather than a prompt (R81); radiant chooses 2, or the one card
// available".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

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

    /// #26 costs 3 on both faces: R74 leaves the cost alone when a card becomes Radiant (§5.2).
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
            let mut s = glowy(json!(["traininggrounds-050", "core-013"]));
            let chosen = hand_id_of(s.state(), "core-013");

            s.play("traininggrounds-050", json!({ "targets": [{ "pick": "instance", "instanceId": chosen }] }));

            // #13 Jlockeed Shredder-10 is 6/10 → 12/20: no card id changed, only the flag.
            s.expect_stats(&chosen, json!({ "attack": 12, "maxHealth": 20 }));
            assert_eq!(s.card(&chosen).def_id, "core-013");
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

    mod n26_glowy_jelly_bean_radiant {
        use super::*;

        #[test]
        fn chooses_2_and_both_cards_get_the_flag_from_one_play_r81() {
            crate::register_all();
            let mut s = glowy(json!([{ "def": "traininggrounds-050", "radiant": true }, "core-005", "core-016", "core-010"]));
            let first = hand_id_of(s.state(), "core-005");
            let second = hand_id_of(s.state(), "core-016");
            let untouched = hand_id_of(s.state(), "core-010");

            s.play(
                "traininggrounds-050",
                json!({
                    "targets": [
                        { "pick": "instance", "instanceId": first },
                        { "pick": "instance", "instanceId": second },
                    ],
                }),
            );

            assert!(s.state().pending.is_none());
            let mut flagged = radiant_hand(s.state());
            flagged.sort();
            let mut wanted = vec![first.clone(), second.clone()];
            wanted.sort();
            assert_eq!(flagged, wanted);
            assert!(!s.card(&untouched).radiant);
        }

        #[test]
        fn r90_takes_the_one_card_available_when_the_hand_holds_only_one_other() {
            crate::register_all();
            let mut s = glowy(json!([{ "def": "traininggrounds-050", "radiant": true }, "core-005"]));
            let only = hand_id_of(s.state(), "core-005");

            // R90: "a declaration the board cannot satisfy does not refuse the play: the play is legal with
            // the answers that exist and the effect fizzles on resolution". So one selection reaches a
            // declaration that asks for two, and the second `setRadiant` finds nothing.
            s.play("traininggrounds-050", json!({ "targets": [{ "pick": "instance", "instanceId": only }] }));

            assert_eq!(radiant_hand(s.state()), vec![only.clone()]);
            s.expect_in_zone("traininggrounds-050", "graveyard");
        }
    }

    /// R2195003: the snapshot plays exactly as core-026, on both faces: the chosen hand cards become Radiant.
    #[test]
    fn r2195003_plays_exactly_as_core_026() {
        for radiant in [false, true] {
            crate::expect_plays_as(
                "core-026",
                super::ID,
                |id| json!({ "p1": { "hand": [{ "def": id, "radiant": radiant }, "core-008", "core-025"] } }),
                |s, id| {
                    let first = s.card("core-008").id.clone();
                    let second = s.card("core-025").id.clone();
                    let picks: Vec<_> = if radiant { vec![first, second] } else { vec![first] };
                    let targets: Vec<_> = picks.iter().map(|p| json!({ "pick": "instance", "instanceId": p })).collect();
                    s.play(id, json!({ "targets": targets }));
                },
            );
        }
    }
}
