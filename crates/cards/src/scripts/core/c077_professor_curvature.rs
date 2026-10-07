//! #77 Professor Curvature (SPEC §8.3, R48, R65, R363, §2.2, §10.1).
//!
//! Base: "Cry: (4)+ Cost cards cost (1) less on your next turn."; radiant "... cost (2) less ...".
//! Patch v0.1.1 widened it from cards whose cost is exactly 4 to cards whose cost is 4 or more
//! (R363), and cut its body to 3/3 → 6/6; the stats come from the catalog.
//!
//! §8's Engine cell: "Delayed player modifier, checked against current cost at play; expires at that
//! turn's cleanup." That is one `PlayerModifier`, not a delayed effect:
//!
//!   R363 "Cost (4)+" → `minCurrentCost: 4`, which `mana::effective_cost` reads AFTER `cost_mod` and the
//!        flat discounts, exactly where R65 puts it: "add player discounts; apply Professor
//!        Curvature if the result is then 4 or more; floor at 0". So a card the board has already
//!        discounted from 5 to 4 is caught, and a printed-4 card another discount has already taken
//!        to 3 is not.
//!   R48  "on your NEXT turn" → `{ until: "nextTurnOf", player, fromTurn }`. `mana::modifier_is_live`
//!        answers false while `state.turn == from_turn`, so the discount does nothing on the turn
//!        Curvature was played, and `modifiers::expire_modifiers` drops it at the cleanup of that
//!        player's next turn (§2.2: "Cleanup expires … Professor Curvature's discount on its turn").
//!
//! The modifier sits on the controller's own `mods`, so it is read only when that player's cards are
//! costed; the opponent's turn in between cannot reach it even while it is live.

use jackioh_engine::prelude::*;

pub const ID: &str = "core-077";

/// R363: the least current cost the discount reaches, checked after every other modifier (R65).
const TARGET_COST: i32 = 4;

/// The two faces differ only in how much the discount is worth.
fn professor_curvature(amount: i32) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            vec![add_player_modifier(json_as(json!({
                "player": "self",
                "mod": {
                    "kind": "costDiscount",
                    "amount": amount,
                    "minCurrentCost": TARGET_COST,
                    // R48: it covers the controller's NEXT turn, so it survives the turn it was created on.
                    "expiry": { "until": "nextTurnOf", "player": ctx.controller, "fromTurn": ctx.state.turn },
                },
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: professor_curvature(1),
        radiant: professor_curvature(2),
    }
}

// #77 Professor Curvature — SPEC §8.3, R48, R65, R363.
//
// BUILD M4-T4: "Next turn only: cards whose current cost is 4 or more −1 (radiant −2); not this turn;
// expires (R48, R363)".
//
// The three clauses are read off the hand's own cost, which `view_for` computes with
// `mana::effective_cost` (§10.8, R65) — the same number the play validator charges — and confirmed by
// actually paying for the cost-4 card on the turn the discount is live.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const CURVATURE: &str = "core-077";

    /// The probes, in hand and never played except where a test says so.
    const COST_3: &str = "core-070"; // Spiteful Stab (Flood costs (4) since patch v0.2.0)
    const COST_4: &str = "core-025"; // 4-mana 7/7 — a Unit, so it has a lane to enter
    const COST_6: &str = "core-029"; // GIGA Glowy Jelly Bean

    /// There is no printed cost-5 card in Core, so "Cost (4)+" above 4 is read at 6 (§8.3, R363).
    const PROBES: [&str; 3] = [COST_3, COST_4, COST_6];

    /// Both sides keep a card and a unit, so no turn auto-ends underneath a cross-turn test (R82).
    const LIBRARY: [&str; 4] = ["core-008", "core-008", "core-008", "core-008"];

    /// The cost a card in the viewer's own hand shows now (§10.8, R65).
    fn hand_cost(s: &Scenario, def_id: &str) -> i32 {
        let view = s.view(Some(PlayerId::P1));
        let HandView::Cards(hand) = &view.you.hand else {
            panic!("§10.8: the viewer's own hand is a list of cards");
        };
        let Some(card) = hand.iter().find(|entry| entry.def_id == def_id) else {
            panic!("{def_id} is not in p1's hand");
        };
        card.cost
    }

    /// Harness gap (reported): there is no `mods()` accessor, so the modifier is read off the state in
    /// the test file. REVIEW B1.7's purity grep covers `src` only, so this is fine here.
    fn discounts(s: &Scenario) -> Vec<PlayerModifier> {
        s.state()
            .players
            .p1
            .mods
            .iter()
            .filter(|modifier| matches!(modifier.kind, ModifierKind::CostDiscount { .. }))
            .cloned()
            .collect()
    }

    fn board(radiant: bool) -> Scenario {
        let hand: Vec<Value> = std::iter::once(json!({ "def": CURVATURE, "radiant": radiant }))
            .chain(PROBES.iter().map(|probe| json!(probe)))
            .collect();
        scenario(json!({
            "p1": {
                "hand": hand,
                "library": LIBRARY,
            },
            "p2": { "hand": ["core-005"], "field": ["core-019"], "library": LIBRARY },
        }))
    }

    /// Two `end_turn()`s: the opponent really takes a turn in between, so this is p1's NEXT turn.
    fn to_my_next_turn(s: &mut Scenario) {
        s.end_turn();
        s.end_turn();
    }

    mod professor_curvature_base {
        use super::*;

        #[test]
        fn r48_the_discount_does_nothing_on_the_turn_professor_curvature_was_played() {
            let mut s = board(false);

            s.play(CURVATURE, json!({}));

            assert_eq!(hand_cost(&s, COST_4), 4);
            assert_eq!(hand_cost(&s, COST_3), 3);
            assert_eq!(hand_cost(&s, COST_6), 6);
            // It is installed all the same: R48 is `modifier_is_live`'s "not on `from_turn`", not an absent mod.
            assert_eq!(discounts(&s).len(), 1);
            let installed = serde_json::to_value(&s.state().players.p1.mods[0]).unwrap();
            assert_eq!(installed["kind"], json!("costDiscount"));
            assert_eq!(installed["amount"], json!(1));
            assert_eq!(installed["minCurrentCost"], json!(4));
            assert_eq!(installed["expiry"], json!({ "until": "nextTurnOf", "player": "p1", "fromTurn": 9 }));
        }

        #[test]
        fn r363_plus_r65_on_the_controllers_next_turn_a_cost_4_card_costs_3_a_cost_6_card_5_and_a_cost_3_card_is_untouched() {
            let mut s = board(false);
            s.play(CURVATURE, json!({}));

            to_my_next_turn(&mut s);

            assert_eq!(hand_cost(&s, COST_4), 3);
            // R65, R363: Curvature applies when the cost is 4 or more after every other modifier.
            assert_eq!(hand_cost(&s, COST_3), 3);
            assert_eq!(hand_cost(&s, COST_6), 5);
        }

        #[test]
        fn r48_the_discount_is_really_charged_the_cost_4_card_is_paid_at_3() {
            let mut s = board(false);
            s.play(CURVATURE, json!({}));
            to_my_next_turn(&mut s);

            // Mana refreshed to MAX_MANA (4) at the start of this turn (§2.3).
            s.expect_mana(PlayerId::P1, 4);
            s.play(COST_4, json!({}));
            s.expect_mana(PlayerId::P1, 1);
        }

        #[test]
        fn r48_the_discount_expires_at_that_turns_cleanup() {
            let mut s = board(false);
            s.play(CURVATURE, json!({}));
            to_my_next_turn(&mut s);
            assert_eq!(hand_cost(&s, COST_4), 3);

            // Ending the turn it covered is the cleanup that drops it (§2.2).
            s.end_turn();

            assert_eq!(discounts(&s).len(), 0);
            assert_eq!(hand_cost(&s, COST_4), 4);
        }

        #[test]
        fn the_cry_fires_on_play_from_hand_and_the_body_is_the_printed_3_3_s8_3() {
            let mut s = board(false);

            s.play(CURVATURE, json!({}));

            s.expect_events(json!(["cardPlayed", "modifierChanged"]));
            s.expect_stats(CURVATURE, json!({ "attack": 3, "maxHealth": 3, "health": 3 }));
        }
    }

    // R169, BUILD M5-T4 ("badge list equals the view's modifiers"). Until R169 the discount existed
    // only in `state.players[p].mods`, which no view carried and no component drew, so a player had no
    // way to know Professor Curvature was on them — least of all on the turn it was played, where R48
    // makes it change no card's cost either. The proof is the view, not the state.
    mod professor_curvature_visible_to_the_player_while_active_r169_s10_8 {
        use super::*;

        #[test]
        fn shows_a_badge_on_the_controllers_seat_the_moment_it_resolves_saying_it_waits_for_next_turn() {
            let mut s = board(false);

            s.play(CURVATURE, json!({}));

            let badges = s.view(Some(PlayerId::P1)).you.modifiers;
            assert_eq!(badges.len(), 1, "the discount is on the board and so is its badge");
            assert_eq!(
                badges.first().map(|badge| badge.label.clone()),
                Some("(4)+ Cost cards cost (1) less (next turn)".to_string())
            );
            // §10.3 names the badge by id, so the animation lands on the element the view carries.
            assert_eq!(
                badges.first().map(|badge| badge.id.clone()),
                s.state().players.p1.mods.first().map(|modifier| modifier.id.clone())
            );
        }

        #[test]
        fn the_badge_drops_its_hedge_on_the_turn_the_discount_bites_and_goes_at_that_turns_cleanup() {
            let mut s = board(false);
            s.play(CURVATURE, json!({}));

            to_my_next_turn(&mut s);

            let labels: Vec<String> =
                s.view(Some(PlayerId::P1)).you.modifiers.iter().map(|modifier| modifier.label.clone()).collect();
            assert_eq!(labels, vec!["(4)+ Cost cards cost (1) less"]);
            // R48: the same cleanup that ends the discount ends the badge, so neither outlives the other.
            s.end_turn();
            assert!(s.view(Some(PlayerId::P1)).you.modifiers.is_empty());
        }

        #[test]
        fn the_opponent_sees_it_too_a_cry_resolved_face_up_is_public_s10_5_step_4() {
            let mut s = board(true);

            s.play(CURVATURE, json!({}));

            let labels: Vec<String> = s
                .view(Some(PlayerId::P2))
                .opponent
                .modifiers
                .iter()
                .map(|modifier| modifier.label.clone())
                .collect();
            assert_eq!(labels, vec!["(4)+ Cost cards cost (2) less (next turn)"]);
        }
    }

    mod professor_curvature_radiant {
        use super::*;

        #[test]
        fn r48_radiant_does_nothing_on_the_turn_it_was_played_either() {
            let mut s = board(true);

            s.play(CURVATURE, json!({}));

            assert_eq!(hand_cost(&s, COST_4), 4);
            let installed = serde_json::to_value(&s.state().players.p1.mods[0]).unwrap();
            assert_eq!(installed["kind"], json!("costDiscount"));
            assert_eq!(installed["amount"], json!(2));
            assert_eq!(installed["minCurrentCost"], json!(4));
        }

        #[test]
        fn r363_plus_r65_radiant_takes_a_cost_4_card_to_2_and_a_cost_6_card_to_4_next_turn_cost_3_untouched() {
            let mut s = board(true);
            s.play(CURVATURE, json!({}));

            to_my_next_turn(&mut s);

            assert_eq!(hand_cost(&s, COST_4), 2);
            assert_eq!(hand_cost(&s, COST_3), 3);
            assert_eq!(hand_cost(&s, COST_6), 4);
            s.play(COST_4, json!({}));
            s.expect_mana(PlayerId::P1, 2);
        }

        #[test]
        fn r48_radiant_expires_at_that_turns_cleanup_too() {
            let mut s = board(true);
            s.play(CURVATURE, json!({}));
            to_my_next_turn(&mut s);

            s.end_turn();

            assert_eq!(discounts(&s).len(), 0);
        }

        #[test]
        fn the_radiant_body_is_the_printed_6_6_s8_3() {
            let mut s = board(true);

            s.play(CURVATURE, json!({}));

            s.expect_stats(CURVATURE, json!({ "attack": 6, "maxHealth": 6, "health": 6 }));
        }
    }
}
