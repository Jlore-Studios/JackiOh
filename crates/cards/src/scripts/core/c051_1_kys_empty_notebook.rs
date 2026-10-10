//! #51.1 KY's Empty Notebook (SPEC §8.3, §7, §5.1, §6.3 Draw; R4, R11, R50, R58, R60).
//! Spell token, cost 1, tags KY + Token. The only generator is #51 KY's Private Tutor's
//! "No match at all → add a KY's Empty Notebook".
//!   Base:    "Draw 1"
//!   Radiant: "Draw 2" — §8 Conventions: a cell that changes only a number changes only that number.
//!
//! The whole card is one Draw. Everything §2.4 hangs off a draw — cast-on-draw, fatigue, the hand
//! cap of 10 (R4) and R58's chain cap — belongs to the draw pipeline, so neither face counts cards.
//!
//! Being a token is data (§7): §5.1's one query keeps Token-tagged cards out of every random pool
//! unless the card names the pool itself; the tests prove it against `crates/cards/src/query.rs`.
//! R11: a SPELL token goes to the GY like any spell (§3.2), so Discover can find it (R50, #72 Reminisce).

use jackioh_engine::prelude::*;

pub const ID: &str = "core-051-1";

/// A spell's script is its `cry` hook (§10.9): the on-resolve hook, fired by `run_hook`. §8: base
/// draws 1, radiant draws 2 — the declared number `draw` (R386), read off the face that is running.
fn notebook() -> Script {
    Script {
        cry: Some(hook(|ctx| vec![draw(json_as(json!({ "count": param(&*ctx, "draw") })))])),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = notebook();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// #51.1 KY's Empty Notebook (SPEC §8.3, §7, §5.1, §6.3 Draw; R4, R11, R50, R60).
// BUILD M4-T4 row 51.1: "Draw 1; radiant 2; absent from every random pool".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    /// Two ordinary library cards, top first, so a draw is visible as an id moving to the hand.
    const LIBRARY: [&str; 3] = ["core-005", "core-016", "core-010"];

    /// §2.5/R82 TURN ANCHOR. #10 Rapid Replenish is a 0-cost Spell and therefore always an affordable
    /// play, so one in hand keeps a side's turn from auto-ending underneath the assertions. It is a
    /// Spell, so it adds no body to the board and nothing to a draw or a hand-size count but its own.
    const ANCHOR: &str = "core-010";

    /// The ids a query answers, in its order.
    fn ids<T: std::borrow::Borrow<CardDef>>(defs: Vec<T>) -> Vec<String> {
        defs.iter().map(|def| <T as std::borrow::Borrow<CardDef>>::borrow(def).id.clone()).collect()
    }

    #[test]
    fn r386_an_upgrade_draws_2_and_a_radiant_degrade_1() {
        for (radiant, upgrade, draws) in [(false, true, 2), (true, false, 1)] {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": [{ "def": "core-051-1", "radiant": radiant }, ANCHOR], "library": LIBRARY } }));
            let moved = if upgrade {
                crate::upgrade_number(&mut s, "core-051-1", "draw")
            } else {
                crate::degrade_number(&mut s, "core-051-1", "draw")
            };
            assert_eq!(moved, draws);
            s.play("core-051-1", json!({}));
            assert_eq!(s.state().players.p1.library.len(), LIBRARY.len() - draws as usize);
        }
    }

    mod n51_1_ky_s_empty_notebook_base {
        use super::*;

        #[test]
        fn draws_1_the_top_library_card_moves_to_hand() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": ["core-051-1"], "library": LIBRARY } }));
            let top = s.pile(P1, "library").first().cloned();

            s.play("core-051-1", json!({}));

            assert_eq!(s.state().players.p1.library.len(), 2);
            assert!(top.is_some());
            if let Some(top) = top {
                s.expect_in_zone(&top, "hand");
            }
            s.expect_events(json!(["cardPlayed", "drawn"]));
        }

        #[test]
        fn r11_a_spell_token_goes_to_the_graveyard_like_any_spell() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": ["core-051-1"], "library": LIBRARY } }));

            s.play("core-051-1", json!({})).expect_in_zone("core-051-1", "graveyard");
            // R50: which is why #72 Reminisce can then Discover it back out of the graveyard.
            let graveyard: Vec<String> = s.pile(P1, "graveyard").iter().map(|card| card.def_id.clone()).collect();
            assert!(graveyard.contains(&"core-051-1".to_string()));
        }

        #[test]
        fn s2_4_an_empty_library_draws_nothing_but_fatigue_and_the_token_still_counts_as_played() {
            crate::register_all();
            // The anchor keeps the turn alive. Without it the Notebook empties p1's hand, §2.5 auto-ends
            // the turn under the assertions, p2 takes one off an empty library too, and `start_turn`
            // clears `turn_log` before `cards_played` is read. The tell is a `turnAutoEnded` followed by
            // a fatigue `damage`.
            let mut s = scenario(json!({ "p1": { "hand": ["core-051-1", ANCHOR], "library": [] } }));

            s.play("core-051-1", json!({}));

            // Still p1's own turn, so every number below belongs to the play above (§2.5).
            assert_eq!(s.state().active, P1);
            // Nothing was drawn: the hand holds the anchor and nothing else.
            let hand: Vec<String> = s.pile(P1, "hand").iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(hand, vec![ANCHOR.to_string()]);
            assert_eq!(s.state().players.p1.turn_log.cards_played, 1);
            // §2.4/R3: no card is drawn, so no `drawn` event — the fatigue damage instance is what happened.
            assert_eq!(
                s.events().iter().filter(|event| event.event_type() == GameEventType::Drawn).count(),
                0
            );
            s.expect_health(P1, 29).expect_events(json!(["damage"]));
        }
    }

    mod n51_1_ky_s_empty_notebook_radiant {
        use super::*;

        #[test]
        fn s8_conventions_only_the_number_changes_so_it_draws_2() {
            crate::register_all();
            let mut s = scenario(json!({ "p1": { "hand": ["core-051-1"], "library": LIBRARY } }));
            // HARNESS GAP (reported): `SideSetup.hand` takes no `{ def, radiant }` form.
            s.card_mut("core-051-1").radiant = true;

            s.play("core-051-1", json!({}));

            assert_eq!(s.state().players.p1.library.len(), 1);
            assert_eq!(s.state().players.p1.hand.len(), 2);
            let hand: Vec<String> = s.pile(P1, "hand").iter().map(|card| card.def_id.clone()).collect();
            assert_eq!(hand, vec!["core-005".to_string(), "core-016".to_string()]);
        }

        #[test]
        fn r4_the_hand_caps_at_10_so_the_second_draw_of_a_nine_card_hand_is_burned() {
            crate::register_all();
            // Nine other cards plus the Notebook is a full hand (HAND_CAP 10); playing it leaves nine, so the
            // first draw fills the tenth slot and the second is burned to the graveyard (R4).
            let mut s = scenario(json!({
                "p1": {
                    "hand": [
                        "core-051-1",
                        "core-001",
                        "core-002",
                        "core-003",
                        "core-004",
                        "core-005",
                        "core-006",
                        "core-007",
                        "core-008",
                        "core-009",
                    ],
                    "library": LIBRARY,
                },
            }));
            s.card_mut("core-051-1").radiant = true;

            s.play("core-051-1", json!({}));

            assert_eq!(s.state().players.p1.hand.len(), 10);
            s.expect_events(json!(["burned"]));
        }
    }

    mod n51_1_ky_s_empty_notebook_s5_1_absent_from_every_random_pool {
        use super::*;

        #[test]
        fn s5_1_the_whole_non_token_catalog_leaves_it_out_and_holds_n51_that_generates_it() {
            crate::register_all();
            let all = ids(crate::query::query(&json_as(json!({}))));

            assert!(!all.contains(&"core-051-1".to_string()));
            assert!(all.contains(&"core-051".to_string()));
        }

        #[test]
        fn s5_1_the_ky_pool_is_core_n31_n51_n82_and_classic_n41_n42_n62_the_token_tagged_ky_cards_are_never_offered() {
            crate::register_all();
            // The pool #57 Conjure KY generates from: the KY tag of every set (R380), minus tokens, minus
            // the generator.
            assert_eq!(
                ids(crate::query::pool("core-057", &json_as(json!({ "tags": ["KY"] })))),
                vec![
                    "core-031".to_string(),
                    "core-051".to_string(),
                    "core-082".to_string(),
                    "classicplus-041".to_string(),
                    "classicplus-042".to_string(),
                    "classicplus-062".to_string(),
                ]
            );
            assert!(!ids(crate::query::query(&json_as(json!({ "tags": ["KY"] })))).contains(&"core-051-1".to_string()));
        }

        #[test]
        fn r60_it_is_reachable_only_by_a_query_that_names_the_token_pool_itself_s5_1() {
            crate::register_all();
            assert!(ids(crate::query::query(&json_as(json!({ "tags": ["Token"] })))).contains(&"core-051-1".to_string()));
            assert_eq!(
                ids(crate::query::query(&json_as(json!({ "defId": "core-051-1" })))),
                vec!["core-051-1".to_string()]
            );
            // A cost or type filter that does not ask for tokens still cannot reach it.
            assert!(
                !ids(crate::query::query(&json_as(json!({ "type": "Spell", "cost": 1 })))).contains(&"core-051-1".to_string())
            );
        }
    }
}
