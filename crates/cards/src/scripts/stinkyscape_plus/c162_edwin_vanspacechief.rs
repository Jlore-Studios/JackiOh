//! SPEC §8.12 S+ #162 Edwin Van’Spacechief (§6.2 Combo, §10.4; R1420): (1) Unit, Human, Rare, a
//! card of the Stinkyscape+ set. Built by S20 (#3568).
//!   Base:    "Combo X: Gain +2X/2X."  (2/2)
//!   Radiant: "Combo X: Gain +4X/4X."  (4/4)
//!
//! X is the number of cards its controller played before this one this turn (SP6's reading, Hearthstone's
//! Edwin): `played_earlier`, this play's own place in the turn's log, a cast counted (R70) and this card
//! itself not, as Core #10 Rapid Replenish reads it. With nothing played earlier the Cry gains nothing.
//! The gain is a permanent buff of attack and health by the same number (§10.4 layer 4), on the card
//! itself. The yellow glow of R195 is not declared: it needs a `conditionMet` hook, which `tests/cross/
//! condition_active.rs` pins to a closed list of cards.
//! The multiples are printed in the words, so they are named constants here, not declared numbers
//! (R1748003), as Core #38's are.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-162";

/// §6.2: a Combo is met by one card played earlier this turn.
const COMBO: i32 = 1;
/// "Gain +2X/2X" on the base face.
const BASE_MULTIPLE: i32 = 2;
/// "Gain +4X/4X" on the Radiant face.
const RADIANT_MULTIPLE: i32 = 4;

/// `multiple`: how much of each card played earlier this unit gains, in attack and in health.
fn edwin(multiple: i32) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let x = played_earlier(ctx.state, ctx.controller, ctx.live_self());
            if x < COMBO {
                return vec![];
            }
            let gain = x * multiple;
            vec![buff(json_as(json!({
                "target": { "of": "self" },
                "attack": gain,
                "health": gain,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: edwin(BASE_MULTIPLE),
        radiant: edwin(RADIANT_MULTIPLE),
    }
}

// S+ #162 Edwin Van’Spacechief — SPEC §8.12 row 162.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const CHEAP: &str = "stand-in (0) Spell"; // a play that does nothing: only the turn's count
    const FILLER: &str = "core-005"; // Stockpile: a card in hand, so no turn ends by itself

    /// p1 holds Edwin of `radiant`'s face behind `plays` free plays; p2 has a card so its turns never end alone.
    fn board(radiant: bool, plays: usize) -> Scenario {
        let mut hand = vec![json!(CHEAP); plays];
        hand.push(json!({ "def": ID, "radiant": radiant }));
        hand.push(json!(FILLER));
        scenario(json!({
            "p1": { "hand": hand, "mana": 10, "library": [FILLER, FILLER, FILLER] },
            "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
        }))
    }

    #[test]
    fn combo_x_gains_twice_x() {
        crate::register_all();
        let mut s = board(false, 2);
        s.play(CHEAP, json!({}))
            .play(CHEAP, json!({}))
            .play(ID, json!({}));
        // Two cards played earlier: X is 2, +4/+4 on the printed 2/2.
        s.expect_stats(ID, json!({ "attack": 2 + 4, "health": 2 + 4 }));
    }

    #[test]
    fn x_zero_gains_nothing() {
        crate::register_all();
        let mut s = board(false, 0);
        s.play(ID, json!({}));
        s.expect_stats(ID, json!({ "attack": 2, "health": 2 }));
        assert!(
            !s.events()
                .iter()
                .any(|event| matches!(event, GameEvent::Buffed { .. }))
        );
    }

    #[test]
    fn one_card_played_earlier_gains_two() {
        crate::register_all();
        let mut s = board(false, 1);
        s.play(CHEAP, json!({})).play(ID, json!({}));
        s.expect_stats(ID, json!({ "attack": 2 + 2, "health": 2 + 2 }));
    }

    #[test]
    fn radiant_gains_four_times_x() {
        crate::register_all();
        let mut s = board(true, 3);
        s.play(CHEAP, json!({}))
            .play(CHEAP, json!({}))
            .play(CHEAP, json!({}))
            .play(ID, json!({}));
        // Three cards played earlier: X is 3, +12/+12 on the Radiant 4/4.
        s.expect_stats(ID, json!({ "attack": 4 + 12, "health": 4 + 12 }));
    }

    #[test]
    fn radiant_with_nothing_played_earlier_gains_nothing() {
        crate::register_all();
        let mut s = board(true, 0);
        s.play(ID, json!({}));
        s.expect_stats(ID, json!({ "attack": 4, "health": 4 }));
    }

    #[test]
    fn x_counts_the_turn_it_is_played_not_an_earlier_one() {
        crate::register_all();
        let mut s = board(false, 2);
        s.play(CHEAP, json!({})).play(CHEAP, json!({}));
        s.end_turn().end_turn();
        // A new turn: the two plays of the last one no longer count.
        s.play(ID, json!({}));
        s.expect_stats(ID, json!({ "attack": 2, "health": 2 }));
    }
}
