//! #10 Rapid Replenish (SPEC §8.1): a 0-cost Spell, "Combo 3: draw 3; otherwise nothing", radiant
//! "Combo 3: draw 6": the faces differ only in how many cards the Combo draws (§8 Conventions).
//!
//! Always playable needs nothing: cost 0, no target, so an unmet Combo draws nothing and the spell
//! still counts as played and goes to the graveyard (§8 Conventions, R40, R70).
//!
//! THE OFF-BY-ONE: §6.2's "Combo X" is "X or more cards were played EARLIER this turn", but §10.5
//! step 4 counts this spell before step 5 runs its script, and step 5's granted Combo draw can cast a
//! cast-on-draw card (R70) that is in the count too. §6.2 checks "at play time", so the script reads
//! `played_earlier`, this play's own place in the controller's turn log. BUILD M4-T4: two prior
//! plays draw nothing, three draw 3.
//!
//! R195, the yellow glow: `condition_met` asks `played_earlier` from the hand too (every play this
//! turn is earlier), so the glow and the draw cannot disagree.

use jackioh_engine::effects::draw;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-010";

/// "Combo 3" (§6.2): three or more cards played earlier this turn — the declared number `combo`, a
/// threshold, less being better (R386) — and the draw, the declared number `draw`: 3, 6 on the
/// Radiant face. The only difference between the two faces is how many cards the met Combo draws.
fn rapid_replenish() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let combo = param(&*ctx, "combo");
            let count = param(&*ctx, "draw");
            // The plays before this one, at play time: not this spell, and not a card its step 5 cast.
            let earlier = played_earlier(ctx.state, ctx.controller, ctx.live_self());
            if earlier >= combo {
                vec![draw(json_as(json!({ "count": count })))]
            } else {
                vec![]
            }
        })),
        // R195: hand only. The condition is about a play; a Spell never sits on the field. In hand,
        // `played_earlier` is every play this turn.
        condition_met: Some(condition_hook(|ctx: ConditionContext<'_>| {
            ctx.zone == ConditionZone::Hand
                && played_earlier(ctx.state, ctx.controller, ctx.self_) >= param(&ctx, "combo")
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = rapid_replenish();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// #10 Rapid Replenish — SPEC §8.1 row 10, BUILD M4-T4 must-pass: "2 prior plays → no draw; 3 →
// draw 3; radiant 6; counts as played either way".
//
// §6.2 counts cards played EARLIER and §10.5 step 4 has already counted this spell, so "2 prior
// plays" and "3 prior plays" are `turnLog.cardsPlayed` of 3 and 4. "Counts as played either way"
// (R40, R70) is proved with a second copy, which finds three earlier plays. The last case (hunt
// round 8): a cast-on-draw Hinder cast by /fullsend's Combo draw at §10.5 step 5 is played after,
// not earlier (§6.2). R195's yellow glow (`conditionMet`) is in condition_active.rs (README §5).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    const LIBRARY: [&str; 8] = [
        "core-020", "core-020", "core-020", "core-011", "core-011", "core-011", "core-016", "core-016",
    ];

    /// The first copy of a def still in hand, as an instance (several copies share a def id).
    fn in_hand(s: &Scenario, def_id: &str) -> CardInstance {
        match s.hand(PlayerId::P1).into_iter().find(|card| card.def_id == def_id) {
            Some(card) => card,
            None => panic!("p1 holds no {def_id}"),
        }
    }

    mod n10_rapid_replenish_s8_1_row_10 {
        use super::*;

        #[test]
        fn r40_two_prior_plays_draw_nothing_and_the_spell_still_counts_as_played_r70() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-010-boundary",
                "p1": { "hand": ["core-011", "core-011", "core-010", "core-010"], "mana": 10, "library": LIBRARY },
                "p2": { "field": ["core-020"] }
            }));

            s.play("core-011", json!({}));
            s.play("core-011", json!({}));
            let library = s.pile(PlayerId::P1, "library").len();
            let first = in_hand(&s, "core-010");

            s.play(&first, json!({}));

            // Combo 3 is not met at two earlier plays: no draw, and the library is untouched.
            assert_eq!(s.pile(PlayerId::P1, "library").len(), library);
            assert_eq!(s.hand(PlayerId::P1).len(), 1);
            // It resolved and was spent all the same (§8 Conventions: the spell still counts as played).
            s.expect_in_zone(&first, "graveyard");

            // R40/R70: because that copy counted, the next one finds three cards played earlier.
            s.play("core-010", json!({}));
            assert_eq!(s.hand(PlayerId::P1).len(), 3);
            assert_eq!(s.pile(PlayerId::P1, "library").len(), library - 3);
        }

        #[test]
        fn three_prior_plays_draw_3() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-010-combo",
                "p1": { "hand": ["core-011", "core-011", "core-011", "core-010"], "mana": 10, "library": LIBRARY },
                "p2": { "field": ["core-020"] }
            }));

            s.play("core-011", json!({}));
            s.play("core-011", json!({}));
            s.play("core-011", json!({}));
            let library = s.pile(PlayerId::P1, "library").len();
            let spell = in_hand(&s, "core-010");

            s.play(&spell, json!({}));

            assert_eq!(s.hand(PlayerId::P1).len(), 3);
            assert_eq!(s.pile(PlayerId::P1, "library").len(), library - 3);
            s.expect_in_zone(&spell, "graveyard");
            // Three separate draws (§2.4), each its own event.
            assert_eq!(
                s.last_events()
                    .iter()
                    .filter(|event| event.event_type() == GameEventType::Drawn)
                    .count(),
                3
            );
        }

        #[test]
        fn radiant_draws_6_at_combo_3() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-010-radiant",
                // #26 Glowy Jelly Bean makes the spell Radiant in hand, and counts as one of the plays.
                "p1": { "hand": ["core-026", "core-011", "core-011", "core-010"], "mana": 10, "library": LIBRARY },
                "p2": { "field": ["core-020"] }
            }));
            let spell = in_hand(&s, "core-010");

            s.play("core-026", json!({ "targets": [{ "pick": "instance", "instanceId": spell.id }] }));
            s.play("core-011", json!({}));
            s.play("core-011", json!({}));
            let library = s.pile(PlayerId::P1, "library").len();

            s.play(&spell, json!({}));

            assert_eq!(s.hand(PlayerId::P1).len(), 6);
            assert_eq!(s.pile(PlayerId::P1, "library").len(), library - 6);
            s.expect_in_zone(&spell, "graveyard");
        }

        #[test]
        fn the_play_is_legal_at_0_mana_with_no_prior_plays_and_draws_nothing() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "core-010-always-playable",
                "p1": { "hand": ["core-010", "core-011"], "mana": 0, "library": LIBRARY },
                "p2": { "field": ["core-020"] }
            }));
            let library = s.pile(PlayerId::P1, "library").len();

            // §8.1: "always playable" — cost 0, no declared choices, no Combo requirement to play it.
            s.play("core-010", json!({}));

            assert_eq!(s.pile(PlayerId::P1, "library").len(), library);
            assert_eq!(s.hand(PlayerId::P1).len(), 1);
            s.expect_in_zone("core-010", "graveyard");
        }

        #[test]
        fn r386_an_upgrade_meets_combo_2_and_a_degrade_needs_combo_4() {
            for (upgrade, combo, drawn) in [(true, 2, 3), (false, 4, 0)] {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "core-010-tuned",
                    "p1": { "hand": ["core-011", "core-011", "core-011", "core-010"], "mana": 10, "library": LIBRARY },
                    "p2": { "field": ["core-020"] }
                }));
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, "core-010", "combo")
                } else {
                    crate::degrade_number(&mut s, "core-010", "combo")
                };
                assert_eq!(moved, combo);
                // Two plays meet an upgraded Combo 2; three do not meet a degraded Combo 4.
                let plays = if upgrade { 2 } else { 3 };
                for _ in 0..plays {
                    s.play("core-011", json!({}));
                }
                let library = s.pile(PlayerId::P1, "library").len();
                s.play("core-010", json!({}));
                assert_eq!(library - s.pile(PlayerId::P1, "library").len(), drawn);
            }
        }

        #[test]
        fn r386_an_upgrade_draws_4_and_a_degrade_2() {
            for (upgrade, count) in [(true, 4), (false, 2)] {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "core-010-tuned",
                    "p1": { "hand": ["core-011", "core-011", "core-011", "core-010"], "mana": 10, "library": LIBRARY },
                    "p2": { "field": ["core-020"] }
                }));
                let moved = if upgrade {
                    crate::upgrade_number(&mut s, "core-010", "draw")
                } else {
                    crate::degrade_number(&mut s, "core-010", "draw")
                };
                assert_eq!(moved, count);
                for _ in 0..3 {
                    s.play("core-011", json!({}));
                }
                let library = s.pile(PlayerId::P1, "library").len();
                s.play("core-010", json!({}));
                assert_eq!(library - s.pile(PlayerId::P1, "library").len(), count as usize);
            }
        }
    }

    mod s6_2_combo_x_n10_counts_the_cards_played_earlier_at_play_time {
        use super::*;

        #[test]
        fn a_card_cast_during_rapid_replenish_s_own_resolution_fullsend_s_combo_draw_is_not_one_played_earlier_r40_r70()
         {
            crate::register_all();
            let vanilla = "core-008";
            let hinder = "core-021";
            let mut g = scenario(json!({
                "seed": "edge-r8-cbc-rapid",
                "p1": {
                    "hand": ["core-078", vanilla, "core-010"],
                    "library": [vanilla, hinder, vanilla, vanilla, vanilla, vanilla, vanilla, vanilla, vanilla, vanilla]
                },
                "p2": { "hand": [vanilla], "library": [vanilla, vanilla] }
            }));
            g.play("core-078", json!({}));
            let in_hand_vanilla = match g.hand(PlayerId::P1).into_iter().find(|card| card.def_id == vanilla) {
                Some(card) => card,
                None => panic!("p1 holds no {vanilla}"),
            };
            g.play(&in_hand_vanilla, json!({ "zone": 1 }));
            let before = g.pile(PlayerId::P1, "library").len();
            // Two cards were played before Rapid Replenish: /fullsend and Mr. Vanilla. Its Combo 3
            // needs three.
            g.play("core-010", json!({}));
            let drawn_by_self = before - g.pile(PlayerId::P1, "library").len();
            // /fullsend's Combo draw takes the Hinder (cast on draw, R70) and draws again: at most 3
            // cards leave the library, counting a Combo draw of the cast Hinder's own. Rapid
            // Replenish's "draw 3" does not fire, because the Hinder was cast after it was played,
            // not earlier.
            assert!(drawn_by_self <= 3);
        }
    }
}
