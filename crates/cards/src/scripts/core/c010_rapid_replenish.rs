//! #10 Rapid Replenish (SPEC §8.1): a 0-cost Spell, "Combo 3: draw 3; otherwise nothing", radiant
//! "Combo 3: draw 6" — a Radiant cell that changes only a number changes only that number (§8
//! Conventions), so the two faces differ only in how many cards the Combo draws.
//!
//! §8.1's Engine cell is "Checks `turnLog.cardsPlayed >= 3`; always playable". Always playable needs
//! nothing: the cost is 0 and the card declares no target, so a Combo that is not met simply draws
//! nothing and the spell still counts as played and still goes to the graveyard (§8 Conventions,
//! R40, R70).
//!
//! THE OFF-BY-ONE, which is the whole subtlety of this card: §6.2 defines "Combo X" as "X or more
//! cards were played EARLIER this turn", and §10.5 step 4 counts the card before step 5 runs this
//! script, so Rapid Replenish is already inside `turnLog.cardsPlayed` when its own hook asks. And
//! step 5 runs /fullsend's granted Combo draw before this script, so a cast-on-draw card that draw
//! casts (R70) is in the count too, though it was played after Rapid Replenish. §6.2 checks the
//! count "at play time", so the script reads `played_earlier`, this play's own place in the turn's
//! log. BUILD M4-T4's must-pass row says the same in numbers: two prior plays draw
//! nothing, three prior plays draw 3.
//!
//! It is read per player: `ctx.controller`'s own turn log, which is the only per-turn record there
//! is, so a card the opponent cast during this turn counts on their log and not on this one.
//!
//! R195, the yellow glow: `condition_met` answers the same question from the hand, before the card is
//! played. `played_earlier` answers it there too: a card still in hand has not been played, so every
//! play this turn is earlier than the one it would be. One reader for both, so the glow and the draw
//! cannot disagree.

use jackioh_engine::effects::draw;
use jackioh_engine::prelude::*;

pub const ID: &str = "core-010";

/// "Combo 3" (§6.2): three or more cards played earlier this turn.
const COMBO: i32 = 3;
const BASE_DRAW: i32 = 3;
const RADIANT_DRAW: i32 = 6;

/// The only difference between the two faces is how many cards the met Combo draws.
fn rapid_replenish(count: i32) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            // The plays before this one, at play time: not this spell, and not a card its step 5 cast.
            let earlier = played_earlier(ctx.state, ctx.controller, ctx.live_self());
            if earlier >= COMBO {
                vec![draw(json_as(json!({ "count": count })))]
            } else {
                vec![]
            }
        })),
        // R195: hand only. The condition is about a play; a Spell never sits on the field. In hand,
        // `played_earlier` is every play this turn.
        condition_met: Some(condition_hook(|ctx: ConditionContext<'_>| {
            ctx.zone == ConditionZone::Hand && played_earlier(ctx.state, ctx.controller, ctx.self_) >= COMBO
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = rapid_replenish(BASE_DRAW);
    let radiant = rapid_replenish(RADIANT_DRAW);
    CardScripts { base, radiant }
}

// #10 Rapid Replenish — SPEC §8.1 row 10, BUILD M4-T4 must-pass: "2 prior plays → no draw; 3 →
// draw 3; radiant 6; counts as played either way".
//
// The boundary is the point of this card. §6.2 counts the cards played EARLIER this turn, and
// §10.5 step 4 has already counted this spell by the time its script runs, so "2 prior plays" and
// "3 prior plays" are `turnLog.cardsPlayed` of 3 and 4. The tests below never read that counter:
// they play real cards first and then assert the draw, which is the only thing a player can see.
//
// "Counts as played either way" (R40, R70) is proved the same way — with a second copy. A Rapid
// Replenish that drew nothing still raised the count, so the copy played right after it is the one
// that finds three earlier plays.
//
// Nothing in the libraries here is cast-on-draw (#21, #27, #90.1 are), so a draw is just a draw.
// The last case is the exception, on purpose (hunt round 8): a cast-on-draw Hinder that /fullsend's
// Combo draw casts at §10.5 step 5, before this script, is played after Rapid Replenish, not
// earlier, since §6.2 checks the count at play time.
//
// R195's yellow glow (`conditionMet`): both answers of this card's hook, checked against the branch
// its resolution then takes, are in condition_active.rs with the other hooked cards (README §5).
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
