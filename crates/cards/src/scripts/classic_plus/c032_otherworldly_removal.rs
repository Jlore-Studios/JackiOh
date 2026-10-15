//! C+ #32 Otherworldly Removal (SPEC §8.7 row 32; §2.4, §7, R4, R11). (2) Spell, Epic.
//!   Base:    "Add an Execute, a Brawl and a Blade Storm to your hand."
//!   Radiant: "Add a Radiant Execute, a Radiant Brawl and a Radiant Blade Storm to your hand."
//!
//! The three Spell tokens C+ #32.1 to #32.3, created in that order and added to the caster's hand
//! through §2.4's pipeline, so a full hand burns what doesn't fit (R4). They are Spell tokens: each
//! lives in hand like a real card and goes to the graveyard once played (R11). The opponent sees three
//! cards added and never which (R97).

use jackioh_engine::effects::add_to_hand;
use jackioh_engine::prelude::*;

pub const ID: &str = "classicplus-032";

/// §8.7 C+ #32: Execute, Brawl, Blade Storm — the order they are added in. TS read each id off its
/// catalog definition (`cardDef(id).id`), which only proved the three exist; the tests below add all
/// three by these ids.
const TOKENS: [&str; 3] = ["classicplus-032-1", "classicplus-032-2", "classicplus-032-3"];

fn otherworldly_removal(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            TOKENS
                .iter()
                .map(|def_id| {
                    let mut args = json!({ "defId": def_id });
                    if radiant {
                        args["radiant"] = json!(true);
                    }
                    add_to_hand(json_as(args))
                })
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: otherworldly_removal(false),
        radiant: otherworldly_removal(true),
    }
}

// C+ #32 Otherworldly Removal — SPEC §8.7 row 32, BUILD M9 Classic+ row C+ 32: "Adds Execute, Brawl
// and Blade Storm (C+ #32.1–#32.3) to your hand in that order, a full hand burning what doesn't fit
// (with 8 other cards in hand, Blade Storm burns); they are Spell tokens and go to the graveyard when
// played (R11); hidden from the opponent (R97); radiant all three Radiant".
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const REMOVAL: &str = "classicplus-032";
    const EXECUTE: &str = "classicplus-032-1";
    const BRAWL: &str = "classicplus-032-2";
    const STORM: &str = "classicplus-032-3";
    const FILLER: &str = "core-005";
    const HIDDEN: &str = "hidden";

    /// Each `addedToHand` event's `(defId, instanceId)`.
    fn added(events: &[GameEvent]) -> Vec<(String, String)> {
        events
            .iter()
            .filter_map(|event| match event {
                GameEvent::AddedToHand { def_id, instance_id, .. } => Some((def_id.clone(), instance_id.clone())),
                _ => None,
            })
            .collect()
    }

    fn added_defs(events: &[GameEvent]) -> Vec<String> {
        added(events).into_iter().map(|(def_id, _)| def_id).collect()
    }

    fn hand_defs(s: &Scenario) -> Vec<String> {
        s.hand(P1).iter().map(|card| card.def_id.clone()).collect()
    }

    mod c_n32_otherworldly_removal {
        use super::*;

        #[test]
        fn is_the_card_it_says() {
            crate::register_all();
            assert_eq!(ID, REMOVAL);
            assert_eq!(crate::card_def(ID).id, REMOVAL);
        }

        mod base {
            use super::*;

            #[test]
            fn s8_7_adds_an_execute_a_brawl_and_a_blade_storm_to_your_hand_in_that_order() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [REMOVAL, FILLER] } }));
                s.play(REMOVAL, json!({}));
                assert_eq!(hand_defs(&s), [FILLER, EXECUTE, BRAWL, STORM]);
                assert!(s.hand(P1).iter().all(|card| !card.radiant));
                assert_eq!(added_defs(s.events()), [EXECUTE, BRAWL, STORM]);
                s.expect_in_zone(REMOVAL, "graveyard");
            }

            #[test]
            fn r4_a_full_hand_burns_what_doesn_t_fit_with_8_other_cards_in_hand_blade_storm_burns() {
                crate::register_all();
                let mut hand = vec![json!(REMOVAL)];
                hand.extend((0..8).map(|_| json!(FILLER)));
                let mut s = scenario(json!({ "p1": { "hand": hand } }));
                s.play(REMOVAL, json!({}));
                let held = hand_defs(&s);
                assert_eq!(held.len(), 10);
                assert_eq!(held[8..], [EXECUTE, BRAWL]);
                assert!(
                    s.events()
                        .iter()
                        .any(|event| matches!(event, GameEvent::Burned { def_id, .. } if def_id == STORM))
                );
                let graveyard: Vec<String> = s.pile(P1, "graveyard").iter().map(|card| card.def_id.clone()).collect();
                assert!(graveyard.iter().any(|id| id == STORM));
            }

            #[test]
            fn r11_they_are_spell_tokens_one_played_goes_to_the_graveyard_like_any_spell() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [REMOVAL, FILLER], "mana": 8 } }));
                s.play(REMOVAL, json!({}));
                s.play(BRAWL, json!({}));
                s.expect_in_zone(BRAWL, "graveyard");
            }

            #[test]
            fn r97_the_opponent_sees_three_cards_added_and_never_which() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [REMOVAL, FILLER] } }));
                s.play(REMOVAL, json!({}));
                let theirs = added(&s.view(P2).events);
                assert_eq!(theirs.len(), 3);
                for (def_id, instance_id) in &theirs {
                    assert_eq!(def_id, HIDDEN);
                    assert_eq!(instance_id, HIDDEN);
                }
                assert_eq!(added_defs(&s.view(P1).events), [EXECUTE, BRAWL, STORM]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn s8_7_adds_a_radiant_execute_a_radiant_brawl_and_a_radiant_blade_storm() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [{ "def": REMOVAL, "radiant": true }, FILLER] } }));
                s.play(REMOVAL, json!({}));
                let fresh: Vec<CardInstance> = s.hand(P1).into_iter().filter(|card| card.def_id != FILLER).collect();
                let ids: Vec<String> = fresh.iter().map(|card| card.def_id.clone()).collect();
                assert_eq!(ids, [EXECUTE, BRAWL, STORM]);
                assert!(fresh.iter().all(|card| card.radiant));
            }
        }
    }
}
