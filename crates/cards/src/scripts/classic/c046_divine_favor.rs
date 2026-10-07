//! C #46 Divine Favor (SPEC §8.6 row 46, BUILD M9 Classic row C 46). (1) Spell, Rare.
//!   Base:    "Draw until you have as many cards in hand as your opponent." (1×, not tunable, R749)
//!   Radiant: the same, at 2× — "draw until you have twice as many cards in hand as your opponent".
//!   Engine:  "Read as it resolves, with this Spell already out of your hand: before each draw it
//!            compares your hand with the opponent's (Radiant: with twice the opponent's) and draws one
//!            card while yours is smaller; a draw that adds no card to your hand (a fatigue hit, a burn at
//!            the hand cap, a card cast on draw, R58, or a draw the draw limit stops, §2.4) ends it, so it
//!            can't loop; a hand already at the mark draws nothing. A `preview` (R280) shows how many
//!            cards it would draw now. Tunes: multiplier 2 ↑, on the Radiant face only (R749)."
//!
//! The loop is the engine's `drawWhile`: the mark read before each draw, the first draw that adds no
//! card ending it. The Spell is resolving, out of the hand (§10.5 step 4). `preview` (R280, proved in
//! `test/preview.test.ts`): the draws it asks for now, from the two hands' public sizes, this card
//! left out of "yours" in hand; label "Draw". `param(ctx, "multiplier")` (R386).

use jackioh_engine::effects::{DrawWhileArgs, draw_while};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-046";

/// R280: the label the preview's value follows, an exact substring of both faces' text.
const PREVIEW_LABEL: &str = "Draw";

/// The cards still wanted: `multiplier ×` the opponent's hand less your own, never below 0. `leaving`
/// counts the cards about to leave your hand before the draws begin (this card, asked in hand; TS's
/// default is 0, which every caller here writes out).
fn draws_wanted(state: &GameState, player: PlayerId, multiplier: i32, leaving: i32) -> i32 {
    let yours = zone_count(state, player, OffFieldZone::Hand) - leaving;
    let mark = multiplier * zone_count(state, opponent_of(player), OffFieldZone::Hand);
    (mark - yours).max(0)
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|_ctx| {
            vec![draw_while(DrawWhileArgs {
                more: Arc::new(|now: &mut EffectContext<'_>| -> bool {
                    draws_wanted(&now.state, now.controller, param(now, "multiplier"), 0) > 0
                }),
                player: None,
            })]
        })),
        preview: Some(condition_hook(|ctx| {
            let in_hand = if ctx.zone == ConditionZone::Hand && matches!(ctx.self_.zone, Zone::Hand { .. }) {
                1
            } else {
                0
            };
            vec![PreviewValue {
                label: PREVIEW_LABEL.to_string(),
                value: draws_wanted(ctx.state, ctx.controller, param(&ctx, "multiplier"), in_hand),
                display: None,
                ids: None,
            }]
        })),
        ..Script::default()
    };

    // The same script: the Radiant face differs only in its declared multiplier (2), read through `param`.
    CardScripts {
        radiant: base.clone(),
        base,
    }
}

// C #46 Divine Favor — SPEC §8.6 row 46, BUILD M9 Classic row C 46: "Read as it resolves (this Spell has
// left your hand): draws one at a time until your hand holds as many cards as the opponent's; level or
// ahead → no draw; a draw that adds no card (fatigue, a burn, a cast-on-draw card, a draw a limit
// stops) ends it, so it never loops; its preview is the number of draws it asks for now (R280); radiant:
// until you hold twice as many; its tuned number (multiplier) reads through `param()` (R386)". The base
// face prints no multiplier, so it is tuned on the Radiant face only (R749).
//
// The preview's proofs are in `test/preview.test.ts` (its C #46 section), with the set of hooked cards.
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::effects::{TuneDirection, TuneRow, applicable_changes};
    use jackioh_engine::testkit::*;

    const FAVOR: &str = "classic-046";
    const MACHINE: &str = "classic-049"; // Anti-Greed Machine: players can't draw more than 1 card each turn.
    const FILLER: &str = "core-010"; // (0) Spell
    const STOCKPILE: &str = "core-005";
    const MENACE: &str = "core-019";
    const CN_VIRUS: &str = "core-090-1"; // Cast on draw
    const HINDER: &str = "core-021"; // Cast on draw: … Discard 1 at random (R682: no prompt).

    use crate::js;

    /// The TS default for `player` is `"p1"`; every caller passes it.
    fn drawn(events: &[GameEvent], player: PlayerId) -> Vec<Value> {
        events.iter().map(js).filter(|event| event["type"] == "drawn" && event["player"] == js(&player)).collect()
    }

    /// The TS default for `defId` is `STOCKPILE`; every caller passes it.
    fn many(count: usize, def_id: &str) -> Vec<String> {
        (0..count).map(|_| def_id.to_string()).collect()
    }

    mod c46_divine_favor {
        use super::*;

        #[test]
        fn is_a_1_spell_its_multiplier_is_a_declared_number_1_radiant_2_tuned_on_the_radiant_face_only_it_declares_a_preview() {
            crate::register_all();
            let def = js(&registered_catalog()[ID]);
            assert_eq!(def["type"], "Spell");
            assert_eq!(def["cost"], 1);
            assert_eq!(
                def["params"],
                json!([{ "key": "multiplier", "base": 1, "radiant": 2, "better": "up", "step": 1, "min": 1, "tunedOn": "radiant" }]),
            );
            let scripts = script();
            assert!(scripts.base.preview.is_some());
            // TS `expect(radiant).toBe(base)`: the Radiant face is the same script, the same hooks present.
            assert_eq!(scripts.radiant.preview.is_some(), scripts.base.preview.is_some());
            assert_eq!(scripts.radiant.cry.is_some(), scripts.base.cry.is_some());
        }

        mod base {
            use super::*;

            #[test]
            fn draws_until_your_hand_holds_as_many_cards_as_the_opponents_this_spell_already_out_of_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FAVOR, FILLER], "library": many(6, MENACE) },
                    "p2": { "hand": many(4, STOCKPILE) },
                }));
                s.play(FAVOR, json!({}));
                assert_eq!(drawn(&s.last_events(), PlayerId::P1).len(), 3);
                assert_eq!(s.hand(PlayerId::P1).len(), 4);
            }

            #[test]
            fn level_or_ahead_no_draw() {
                crate::register_all();
                let mut level = scenario(json!({
                    "p1": { "hand": [FAVOR, FILLER, FILLER], "library": many(3, MENACE) },
                    "p2": { "hand": many(2, STOCKPILE) },
                }));
                level.play(FAVOR, json!({}));
                assert_eq!(drawn(&level.last_events(), PlayerId::P1).len(), 0);
                let mut ahead = scenario(json!({
                    "p1": { "hand": [FAVOR, FILLER, FILLER, FILLER], "library": many(3, MENACE) },
                    "p2": { "hand": many(1, STOCKPILE) },
                }));
                ahead.play(FAVOR, json!({}));
                assert_eq!(drawn(&ahead.last_events(), PlayerId::P1).len(), 0);
            }

            #[test]
            fn s2_4_a_fatigue_hit_adds_no_card_and_ends_it_one_hit_however_far_behind() {
                crate::register_all();
                let mut s = scenario(json!({ "p1": { "hand": [FAVOR], "library": [MENACE] }, "p2": { "hand": many(6, STOCKPILE) } }));
                s.play(FAVOR, json!({}));
                assert_eq!(drawn(&s.last_events(), PlayerId::P1).len(), 1);
                assert_eq!(s.state().players.p1.fatigue_count, 1);
            }

            #[test]
            fn s2_4_a_burn_at_the_hand_cap_adds_no_card_and_ends_it() {
                crate::register_all();
                let mut hand = vec![json!({ "def": FAVOR, "radiant": true })];
                hand.extend(many(9, FILLER).into_iter().map(Value::from));
                let mut s = scenario(json!({
                    "p1": { "hand": hand, "library": many(5, MENACE) },
                    "p2": { "hand": many(6, STOCKPILE) },
                }));
                s.play(FAVOR, json!({}));
                assert_eq!(s.hand(PlayerId::P1).len(), 10);
                assert_eq!(s.last_events().iter().map(js).filter(|event| event["type"] == "burned").count(), 1);
                assert_eq!(s.pile(PlayerId::P1, "library").len(), 3);
            }

            #[test]
            fn r58_a_card_cast_on_draw_adds_no_card_and_ends_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FAVOR], "library": [CN_VIRUS, MENACE, MENACE, MENACE] },
                    "p2": { "hand": many(4, STOCKPILE) },
                }));
                s.play(FAVOR, json!({}));
                assert!(s
                    .last_events()
                    .iter()
                    .map(js)
                    .any(|event| event["type"] == "cardPlayed" && event["defId"] == CN_VIRUS));
                // The draw's chain repeats into one card (§2.4), and the draws stop there.
                assert_eq!(s.hand(PlayerId::P1).len(), 1);
                assert_eq!(s.pile(PlayerId::P1, "library").len(), 2);
            }

            #[test]
            fn s9_3_a_cast_on_draw_ends_it_too_r682_no_prompt_the_chain_still_repeats_into_one_menace() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FAVOR, FILLER], "library": [HINDER, MENACE, MENACE] },
                    "p2": { "hand": many(4, STOCKPILE) },
                }));
                s.play(FAVOR, json!({}));
                // Hinder's discard is random (R682): no prompt opens, and the one card held goes.
                assert!(s.state().pending.is_none());
                let round: GameState =
                    serde_json::from_str(&serde_json::to_string(s.state()).expect("the state serialises"))
                        .expect("the state parses back");
                assert_eq!(&round, s.state());
                // Hinder's chain repeats into one Menace; Divine Favor asks for no more.
                assert_eq!(js(&s.card(FILLER).zone)["z"], "graveyard");
                assert_eq!(s.state().players.p1.hand.iter().map(|card| card.def_id.as_str()).collect::<Vec<_>>(), vec![MENACE]);
                assert_eq!(s.state().players.p1.library.len(), 1);
            }

            #[test]
            fn b5_e3_a_draw_a_limit_stops_adds_no_card_and_ends_it() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FAVOR], "field": [MACHINE], "library": many(4, MENACE) },
                    "p2": { "hand": many(4, STOCKPILE) },
                }));
                s.play(FAVOR, json!({}));
                assert_eq!(drawn(&s.last_events(), PlayerId::P1).len(), 1);
                assert_eq!(s.last_events().iter().map(js).filter(|event| event["type"] == "drawLimited").count(), 1);
            }

            #[test]
            fn reads_the_opponents_hand_as_it_resolves_their_hand_is_the_mark_whatever_their_deck_holds() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FAVOR], "library": many(5, MENACE) },
                    "p2": { "hand": many(2, STOCKPILE), "library": many(9, MENACE) },
                }));
                s.play(FAVOR, json!({}));
                assert_eq!(drawn(&s.last_events(), PlayerId::P1).len(), 2);
            }

            #[test]
            fn r749_the_base_faces_multiplier_is_not_tunable_an_upgrades_menu_offers_no_number_and_a_recorded_step_still_draws_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FAVOR, FILLER], "library": many(6, MENACE) },
                    "p2": { "hand": many(3, STOCKPILE) },
                }));
                assert!(!applicable_changes(s.state(), s.card(FAVOR), TuneDirection::Upgrade).contains(&TuneRow::Number));
                assert!(!applicable_changes(s.state(), s.card(FAVOR), TuneDirection::Degrade).contains(&TuneRow::Number));
                step_param(s.card_mut(FAVOR), "multiplier", 1);
                s.play(FAVOR, json!({}));
                // As many as the opponent's three, as the text says: the Filler and two draws.
                assert_eq!(s.hand(PlayerId::P1).len(), 3);
                assert_eq!(drawn(&s.last_events(), PlayerId::P1).len(), 2);
            }

            #[test]
            fn r97_the_drawn_cards_are_never_named_in_the_opponents_view() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FAVOR], "library": many(2, MENACE) },
                    "p2": { "hand": many(2, STOCKPILE) },
                }));
                s.play(FAVOR, json!({}));
                let ids: Vec<String> = s.hand(PlayerId::P1).iter().map(|card| card.id.clone()).collect();
                let text = js(&s.view(PlayerId::P2)).to_string();
                for id in &ids {
                    assert!(!text.contains(id.as_str()));
                }
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn draws_until_you_hold_twice_as_many_cards_as_the_opponent() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": FAVOR, "radiant": true }, FILLER], "library": many(8, MENACE) },
                    "p2": { "hand": many(3, STOCKPILE) },
                }));
                s.play(FAVOR, json!({}));
                assert_eq!(s.hand(PlayerId::P1).len(), 6);
                assert_eq!(drawn(&s.last_events(), PlayerId::P1).len(), 5);
            }

            #[test]
            fn at_twice_as_many_already_no_draw() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": FAVOR, "radiant": true }, FILLER, FILLER], "library": many(3, MENACE) },
                    "p2": { "hand": many(1, STOCKPILE) },
                }));
                s.play(FAVOR, json!({}));
                assert_eq!(drawn(&s.last_events(), PlayerId::P1).len(), 0);
            }

            #[test]
            fn r386_its_multiplier_steps_from_2_a_degrades_step_makes_it_1() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": FAVOR, "radiant": true }], "library": many(8, MENACE) },
                    "p2": { "hand": many(3, STOCKPILE) },
                }));
                assert!(applicable_changes(s.state(), s.card(FAVOR), TuneDirection::Degrade).contains(&TuneRow::Number));
                step_param(s.card_mut(FAVOR), "multiplier", -1);
                s.play(FAVOR, json!({}));
                assert_eq!(s.hand(PlayerId::P1).len(), 3);
            }

            #[test]
            fn s2_4_a_fatigue_hit_ends_it_on_the_radiant_face_too() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [{ "def": FAVOR, "radiant": true }], "library": [] },
                    "p2": { "hand": many(3, STOCKPILE) },
                }));
                s.play(FAVOR, json!({}));
                assert_eq!(s.state().players.p1.fatigue_count, 1);
                assert_eq!(s.hand(PlayerId::P1).len(), 0);
            }
        }
    }
}
