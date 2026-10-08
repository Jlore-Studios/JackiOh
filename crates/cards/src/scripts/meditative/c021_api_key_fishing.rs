//! M #21 API Key Fishing (SPEC §8.8 row 21, BUILD M10 row M 21). (0, embiggen 1) Spell, Rare.
//!   Base and Radiant: "{chance}% chance: Steal your opponent's hand.
//!   Paid (1): {paidChance}% instead.
//!   End of turn: Add an API Key Fishing to your hand." (2/5; 5/12 step 2)
//!
//! `cost: { base: 0, embiggen: 1 }` (§2.3; the script reads `ctx.embiggened`). At resolution the hook
//! draws one `ctx.rng.chance(p / 100)`; on success it runs `give_from_hand { from: enemy, cards:
//! all }`: the cards become their taker's (E2/E16, R12), the hand cap burns the overflow into the
//! taker's graveyard, and `stolen` is hidden per zone (R97). Against an empty hand there is nothing
//! to take, so it rolls nothing (R129).
//!
//! The `end_of_turn` hook works from the graveyard on the turn the card was played (the return flag,
//! R153, R155): `add_to_hand { defId: "meditative-021" }` adds a fresh card, while this one stays in
//! the graveyard. The added card is a new, base API Key Fishing, even from the Radiant face (a
//! generated card isn't Radiant unless said, §5.2; R805). Both numbers are declared and read
//! through `param` (R386).

use jackioh_engine::effects::{add_to_hand, give_from_hand};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-021";

/// A percent roll is out of a hundred.
const PERCENT: i32 = 100;

/// §5.1 and R68, as Core #23 Reoccurring Dream reads them: at the end of the turn it was played on,
/// the spell adds its copy from the graveyard — flagged for the return, or in this turn's play log.
fn returns_this_turn(ctx: &EffectContext<'_>) -> bool {
    let Some(self_) = ctx.live_self() else {
        return false;
    };
    if self_.return_to_hand_at_end_of_turn == Some(true) {
        return true;
    }
    was_played_this_turn(&*ctx.state, self_.controller, self_)
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            // R129: with no hand to steal, there is nothing to roll for.
            if zone_count(&*ctx.state, opponent_of(ctx.controller), OffFieldZone::Hand) == 0 {
                return vec![];
            }
            let key = if ctx.embiggened { "paidChance" } else { "chance" };
            let odds = f64::from(param(&*ctx, key)) / f64::from(PERCENT);
            if ctx.rng.chance(odds) {
                vec![give_from_hand(json_as(json!({ "from": "enemy", "cards": "all" })))]
            } else {
                vec![]
            }
        })),
        end_of_turn: Some(hook(|ctx| {
            if returns_this_turn(ctx) {
                vec![add_to_hand(json_as(json!({ "defId": ID })))]
            } else {
                vec![]
            }
        })),
        ..Script::default()
    };

    // The same script: the Radiant face differs only in its declared numbers, read through `param`.
    let radiant = base.clone();

    CardScripts { base, radiant }
}

// M #21 API Key Fishing — SPEC §8.8 row 21, BUILD M10 row M 21: "A {chance}% roll steals the whole
// hand (paid (1): {paidChance}%); at end of turn a new base copy is added and the card stays in the
// graveyard (R805, R153); nothing on later turns; the Radiant still adds a base copy; an empty hand
// rolls nothing (R129); its tuned numbers read through `param()` (R386)".
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FISHING: &str = "meditative-021";
    const FILLER: &str = "core-005"; // (1) Spell.
    const VANILLA: &str = "core-008"; // (1) Unit.
    const TIMMY: &str = "core-011"; // (1) Unit.

    /// p1's hand defs of `def`, with radiance.
    fn hand_copies(s: &Scenario, def: &str) -> Vec<bool> {
        s.hand(P1).iter().filter(|card| card.def_id == def).map(|card| card.radiant).collect()
    }

    mod meditative_021 {
        use super::*;

        #[test]
        fn probe_fishing_seeds_temporary() {
            crate::register_all();
            for n in 0..300 {
                let seed = format!("probe-{n}");
                let mut s = scenario(json!({
                    "seed": seed,
                    "p1": { "hand": [FISHING, TIMMY], "field": [TIMMY] },
                    "p2": { "hand": [FILLER, VANILLA], "field": [TIMMY] },
                }));
                s.play(FISHING, json!({}));
                let base_took = s.hand(P2).is_empty();
                let mut t = scenario(json!({
                    "seed": seed,
                    "p1": { "hand": [FISHING, TIMMY], "mana": 9, "field": [TIMMY] },
                    "p2": { "hand": [FILLER, VANILLA], "field": [TIMMY] },
                }));
                t.play(FISHING, json!({ "embiggen": true }));
                let paid_took = t.hand(P2).is_empty();
                if base_took || paid_took {
                    println!("seed {seed}: base_took={base_took} paid_took={paid_took}");
                }
            }
        }

        #[test]
        fn a_success_takes_their_whole_hand() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "TBD-success",
                "p1": { "hand": [FISHING, TIMMY], "field": [TIMMY] },
                "p2": { "hand": [FILLER, VANILLA], "field": [TIMMY] },
            }));

            s.play(FISHING, json!({}));

            assert!(s.hand(P2).is_empty());
            let taken: Vec<String> = s.hand(P1).iter().map(|card| card.def_id.clone()).collect();
            assert!(taken.contains(&FILLER.to_string()));
            assert!(taken.contains(&VANILLA.to_string()));
        }

        #[test]
        fn a_failure_takes_nothing() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "TBD-failure",
                "p1": { "hand": [FISHING, TIMMY], "field": [TIMMY] },
                "p2": { "hand": [FILLER, VANILLA], "field": [TIMMY] },
            }));

            s.play(FISHING, json!({}));

            assert_eq!(s.hand(P2).len(), 2);
            assert_eq!(s.hand(P1).len(), 1);
        }

        #[test]
        fn paid_1_uses_paid_chance() {
            crate::register_all();
            // This seed fails the base 2% and passes the paid 5%: the same roll, two prices.
            let mut s = scenario(json!({
                "seed": "TBD-paid",
                "p1": { "hand": [FISHING, TIMMY], "mana": 9, "field": [TIMMY] },
                "p2": { "hand": [FILLER, VANILLA], "field": [TIMMY] },
            }));

            s.play(FISHING, json!({ "embiggen": true }));

            assert!(s.hand(P2).is_empty());
            s.expect_mana(P1, 8);
        }

        #[test]
        fn r805_at_end_of_turn_adds_a_base_copy_and_stays_in_the_graveyard() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "meditative-021-return",
                "p1": { "hand": [FISHING, TIMMY], "field": [TIMMY] },
                "p2": { "hand": [FILLER], "field": [TIMMY] },
            }));
            s.play(FISHING, json!({}));
            s.expect_in_zone(FISHING, "graveyard");

            s.end_turn();

            assert_eq!(hand_copies(&s, FISHING), vec![false]);
            s.expect_in_zone(FISHING, "graveyard");
        }

        #[test]
        fn r805_not_on_later_turns() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "meditative-021-once",
                "p1": { "hand": [FISHING, TIMMY], "field": [TIMMY] },
                "p2": { "hand": [FILLER], "field": [TIMMY] },
            }));
            s.play(FISHING, json!({}));
            s.end_turn();
            assert_eq!(hand_copies(&s, FISHING).len(), 1);

            s.end_turn();
            s.end_turn();

            assert_eq!(hand_copies(&s, FISHING).len(), 1);
        }

        #[test]
        fn r805_radiant_still_adds_a_base_copy() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "meditative-021-radiant-return",
                "p1": { "hand": [{ "def": FISHING, "radiant": true }, TIMMY], "field": [TIMMY] },
                "p2": { "hand": [FILLER], "field": [TIMMY] },
            }));
            s.play(FISHING, json!({}));
            s.end_turn();

            assert_eq!(hand_copies(&s, FISHING), vec![false]);
        }

        #[test]
        fn r805_an_empty_hand_rolls_nothing() {
            crate::register_all();
            let mut s = scenario(json!({
                "seed": "meditative-021-empty",
                "p1": { "hand": [FISHING, TIMMY], "field": [TIMMY] },
                "p2": { "hand": [], "field": [TIMMY] },
            }));
            let cursor = s.state().rng_cursor;

            s.play(FISHING, json!({}));

            assert_eq!(s.state().rng_cursor, cursor);
            assert_eq!(s.hand(P1).len(), 1);
        }
    }
}
