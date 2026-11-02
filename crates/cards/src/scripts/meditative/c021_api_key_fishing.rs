//! M #21 API Key Fishing (SPEC §8.8 row 21, BUILD M10 row M 21). (0, embiggen 1) Spell, Rare.
//!   Base:    "{chance}% chance: Steal your opponent's hand.
//!            Embiggened (1): {paidChance}% instead.
//!            End of turn: Add an API Key Fishing to your hand." (2; 5 step 2)
//!   Radiant: the same, but "End of turn: Add a Radiant API Key Fishing to your hand." (4; 10 step 2)
//!
//! Embiggen per §2.3. A success gives the opponent's whole hand: the cards become their taker's
//! (E2/E16, R12), the hand cap burns the overflow into the taker's graveyard, and `stolen` is hidden
//! per zone (R97). The `end_of_turn` hook works from the graveyard on the turn the card was played
//! (the return flag, R153, R155) and adds a fresh copy while this one stays: a base one from the base
//! face (a generated card isn't Radiant unless said, §5.2; R805), a Radiant one from the Radiant face
//! (#1828, R1828000). Both numbers are read through `param` (R386).

use jackioh_engine::effects::{add_to_hand, give_from_hand};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-021";

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

/// The script of a face whose end-of-turn copy is Radiant when `adds_radiant` is.
fn fishing(adds_radiant: bool) -> Script {
    Script {
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
        end_of_turn: Some(hook(move |ctx| {
            if returns_this_turn(ctx) {
                vec![add_to_hand(json_as(json!({ "defId": ID, "radiant": adds_radiant })))]
            } else {
                vec![]
            }
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: fishing(false),
        radiant: fishing(true),
    }
}

/// R767000: through v0.4.9 the Radiant face added a base copy; `KEPT_SCRIPTS` keeps it.
pub fn script_v0_4_9() -> CardScripts {
    CardScripts {
        base: fishing(false),
        radiant: fishing(false),
    }
}

// M #21 API Key Fishing — SPEC §8.8 row 21, BUILD M10 row M 21 (MD-A25).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::def_ids;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FISHING: &str = "meditative-021";
    const FILLER: &str = "core-005"; // (1) Spell.
    const VANILLA: &str = "core-008"; // (1) Unit.
    const TIMMY: &str = "core-011"; // (1) Unit.

    /// p1 holds a Radiant API Key Fishing, against a hand to take.
    fn radiant_return() -> Scenario {
        crate::register_all();
        scenario(json!({
            "seed": "meditative-021-radiant-return",
            "p1": { "hand": [{ "def": FISHING, "radiant": true }, TIMMY], "field": [TIMMY] },
            "p2": { "hand": [FILLER], "field": [TIMMY] },
        }))
    }

    /// p1's hand defs of `def`, with radiance.
    fn hand_copies(s: &Scenario, def: &str) -> Vec<bool> {
        s.hand(P1).iter().filter(|card| card.def_id == def).map(|card| card.radiant).collect()
    }

    mod meditative_021 {
        use super::*;

        /// The seeds the tests below use came from this sweep: on every seed the paid roll takes the
        /// hand whenever the base roll does, the same draw read against higher odds.
        #[test]
        fn paid_odds_take_every_hand_the_base_odds_take() {
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
                assert!(!base_took || paid_took, "seed {seed}: the base roll took the hand, the paid one did not");
            }
        }

        #[test]
        fn a_success_takes_their_whole_hand() {
            crate::register_all();
            let mut s = scenario(json!({
                // This seed passes the base 2%.
                "seed": "probe-32",
                "p1": { "hand": [FISHING, TIMMY], "field": [TIMMY] },
                "p2": { "hand": [FILLER, VANILLA], "field": [TIMMY] },
            }));

            s.play(FISHING, json!({}));

            assert!(s.hand(P2).is_empty());
            let taken = def_ids(&s.hand(P1));
            assert!(taken.contains(&FILLER.to_string()));
            assert!(taken.contains(&VANILLA.to_string()));
        }

        #[test]
        fn a_failure_takes_nothing() {
            crate::register_all();
            let mut s = scenario(json!({
                // This seed fails the base 2% (and the paid 5%).
                "seed": "probe-0",
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
                "seed": "probe-5",
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
            let played = s.card(FISHING).id.clone();
            s.play(FISHING, json!({}));
            s.expect_in_zone(&played, "graveyard");

            s.end_turn();

            // A fresh card in hand, while the played one, named by its id (by its definition the
            // new one in hand would answer), stays in the graveyard.
            assert_eq!(hand_copies(&s, FISHING), vec![false]);
            assert!(s.hand(P1).iter().all(|card| card.id != played));
            s.expect_in_zone(&played, "graveyard");
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
        fn r1828000_the_radiant_face_adds_a_radiant_copy_and_stays_in_the_graveyard() {
            let mut s = radiant_return();
            let played = s.card(FISHING).id.clone();
            s.play(FISHING, json!({}));
            s.end_turn();

            assert_eq!(hand_copies(&s, FISHING), vec![true]);
            assert!(s.hand(P1).iter().all(|card| card.id != played));
            s.expect_in_zone(&played, "graveyard");
        }

        /// R1828000: the Radiant face rolls 4%, and 10% Embiggened. Each roll is pinned to a cursor whose
        /// draw lands in the band named, `[lo, hi)`, the play drawing nothing before the roll.
        #[test]
        fn r1828000_the_radiant_face_rolls_4_and_10_embiggened() {
            const SEED: &str = "meditative-021-radiant-odds";
            let band = |lo: f64, hi: f64| {
                cursor_where(SEED, |cursor| {
                    Rng::new(SEED, cursor).chance(hi) && !Rng::new(SEED, cursor).chance(lo)
                })
            };
            let takes = |cursor: u32, embiggen: bool| {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": SEED,
                    "p1": { "hand": [{ "def": FISHING, "radiant": true }, TIMMY], "mana": 9, "field": [TIMMY] },
                    "p2": { "hand": [FILLER, VANILLA], "field": [TIMMY] },
                }));
                s.state_mut().rng_cursor = cursor;
                s.play(FISHING, json!({ "embiggen": embiggen }));
                s.hand(P2).is_empty()
            };
            assert!(takes(band(0.0, 0.04), false));
            assert!(!takes(band(0.04, 0.05), false), "v0.4.9's 5% is gone");
            assert!(takes(band(0.05, 0.10), true));
            assert!(!takes(band(0.10, 0.12), true), "v0.4.9's 12% is gone");
        }

        /// R767000: a game on v0.4.9 folds under the kept script, whose Radiant face added a base copy.
        #[test]
        fn r767000_r1828000_a_game_on_v0_4_9_folds_under_the_kept_script_the_radiant_face_adds_a_base_copy() {
            crate::register_all();
            jackioh_engine::with_shipped(crate::catalog_at("v0.4.9"), || {
                let mut s = radiant_return();
                s.play(FISHING, json!({}));
                s.end_turn();

                assert_eq!(hand_copies(&s, FISHING), vec![false]);
            });
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
