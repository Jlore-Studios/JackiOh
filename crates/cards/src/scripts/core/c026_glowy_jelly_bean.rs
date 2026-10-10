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

pub const ID: &str = "core-026";

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
            let mut s = glowy(json!(["core-026", "core-005", "core-016"]));
            let chosen = hand_id_of(s.state(), "core-005");
            let other = hand_id_of(s.state(), "core-016");

            s.play("core-026", json!({ "targets": [{ "pick": "instance", "instanceId": chosen }] }));

            // R81: the pick travelled in the play action, so resolution never paused.
            assert!(s.state().pending.is_none());
            assert_eq!(radiant_hand(s.state()), vec![chosen.clone()]);
            assert!(!s.card(&other).radiant);
            s.expect_in_zone("core-026", "graveyard")
                .expect_events(json!(["cardPlayed", "radiantSet"]));
        }

        #[test]
        fn r74_the_flag_is_the_whole_model_so_the_card_s_stats_swap_while_it_sits_in_hand_s5_2() {
            crate::register_all();
            let mut s = glowy(json!(["core-026", "core-013"]));
            let chosen = hand_id_of(s.state(), "core-013");

            s.play("core-026", json!({ "targets": [{ "pick": "instance", "instanceId": chosen }] }));

            // #13 Jlockeed Shredder-10 is 8/10 → 16/20: no card id changed, only the flag.
            s.expect_stats(&chosen, json!({ "attack": 16, "maxHealth": 20 }));
            assert_eq!(s.card(&chosen).def_id, "core-013");
            assert_eq!(s.card(&chosen).zone.z(), ZoneName::Hand);
        }

        #[test]
        fn s6_3_leaves_a_card_that_is_already_radiant_alone_nothing_in_core_un_sets_the_flag() {
            crate::register_all();
            let mut s = glowy(json!(["core-026", { "def": "core-005", "radiant": true }]));
            let chosen = hand_id_of(s.state(), "core-005");

            s.play("core-026", json!({ "targets": [{ "pick": "instance", "instanceId": chosen }] }));

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

        #[test]
        fn s8_conventions_an_empty_hand_fizzles_the_pick_and_the_spell_still_counts_as_played() {
            crate::register_all();
            let mut s = glowy(json!(["core-026"]));

            s.play("core-026", json!({}));

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
            s.expect_in_zone("core-026", "graveyard");
        }
    }

    mod n26_glowy_jelly_bean_radiant {
        use super::*;

        #[test]
        fn chooses_2_and_both_cards_get_the_flag_from_one_play_r81() {
            crate::register_all();
            let mut s = glowy(json!([{ "def": "core-026", "radiant": true }, "core-005", "core-016", "core-010"]));
            let first = hand_id_of(s.state(), "core-005");
            let second = hand_id_of(s.state(), "core-016");
            let untouched = hand_id_of(s.state(), "core-010");

            s.play(
                "core-026",
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
            let mut s = glowy(json!([{ "def": "core-026", "radiant": true }, "core-005"]));
            let only = hand_id_of(s.state(), "core-005");

            // R90: "a declaration the board cannot satisfy does not refuse the play: the play is legal with
            // the answers that exist and the effect fizzles on resolution". So one selection reaches a
            // declaration that asks for two, and the second `setRadiant` finds nothing.
            s.play("core-026", json!({ "targets": [{ "pick": "instance", "instanceId": only }] }));

            assert_eq!(radiant_hand(s.state()), vec![only.clone()]);
            s.expect_in_zone("core-026", "graveyard");
        }
    }
}
