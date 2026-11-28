//! FvH #104 Dark Skies (SPEC §8.14 row 104): (2) Spell, Epic, of the Felinors vs Humans set.
//!   Base:    "Pierce / Deal {damage} damage to a random Unit. Repeat this {repeat|time|times} for each card
//!            in your hand. / Combo: Draw {draw|card|cards}." (damage 1, repeat 2, draw 1)
//!   Radiant: the same, damage 2.
//!
//! Pierce is catalog data, read off the source as it resolves (R346). One `damage_split` among
//! `SplitAmong::Units` (R5239003): the first hit and `repeat` more for each card in the caster's hand as it
//! resolves (this Spell already out of it), each of the declared `damage` (R386), every hit drawn among the
//! Units of both sides still standing, never a hero (R60), Spell Damage added to each. Combo (§6.2, X = 1):
//! with a card played before it this turn, `draw` the declared `draw`; the count is `played_earlier`, this
//! play's own place in the turn log, as Core #10 Rapid Replenish reads it. Both faces run one script: the
//! Radiant face's 2 is its declared `damage`.

use jackioh_engine::effects::draw;
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-104";

/// "Deal … to a random Unit": the hit before any repeat.
const FIRST_HIT: i32 = 1;

/// §6.2's Combo with no number: one card played earlier this turn.
const COMBO: i32 = 1;

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            let damage = param(&*ctx, "damage");
            let held = zone_count(&*ctx.state, ctx.controller, OffFieldZone::Hand);
            let hits = FIRST_HIT + param(&*ctx, "repeat") * held;
            let mut effects = vec![damage_split(json_as(json!({
                "amount": hits * damage,
                "perHit": damage,
                "among": "units",
            })))];
            if played_earlier(&*ctx.state, ctx.controller, ctx.live_self()) >= COMBO {
                effects.push(draw(json_as(json!({ "count": param(&*ctx, "draw") }))));
            }
            effects
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 2 a hit is its declared `damage`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #104 Dark Skies — SPEC §8.14 row 104 (R346, R5239003).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{filler, hits, scenario};

    /// Stockpile: a Spell each hand keeps.
    const FILLER: &str = "core-005";
    /// A Unit no hit here can kill.
    const BIG: &str = "stand-in (1) 1/30";
    /// A Unit whose Armor a hit without Pierce would spend first.
    const ARMORED: &str = "stand-in (1) 1/30 Armor 7";
    /// A free Spell to play first, for the Combo.
    const FREE: &str = "stand-in (0) Spell";

    /// p1 holds Dark Skies, `extra` and one filler; each side has a `unit` in lane 1.
    fn board(radiant: bool, extra: &[&str], unit: &str) -> Scenario {
        let mut hand = vec![json!({ "def": ID, "radiant": radiant })];
        hand.extend(extra.iter().map(|card| json!(card)));
        hand.push(json!(FILLER));
        scenario(json!({
            "seed": "dark-skies",
            "p1": { "hand": hand, "field": [unit], "library": filler(4) },
            "p2": { "hand": [FILLER], "field": [unit], "library": filler(4) },
        }))
    }

    /// The hits Dark Skies dealt: each on a Unit of either side, never a hero.
    fn unit_hits(s: &Scenario) -> Vec<i32> {
        let units = [s.unit_at(P1, 1).id, s.unit_at(P2, 1).id];
        nonempty(hits(s.last_events()), "Dark Skies' hits")
            .into_iter()
            .map(|(target, amount)| {
                assert!(units.contains(&target), "{target} is a Unit on the field, no hero");
                amount
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r5239003_one_hit_and_two_more_for_each_card_in_hand_one_damage_each_on_units_only() {
            let mut s = board(false, &[FILLER], BIG);
            s.play(ID, json!({}));
            // Two fillers left in hand: 1 + 2 × 2 hits of 1.
            assert_eq!(unit_hits(&s), vec![1; 5]);
            assert_eq!(s.state().players.p1.hero.health, s.state().players.p2.hero.health, "no hero hit");
        }

        #[test]
        fn r346_pierce_puts_every_hit_past_armor() {
            let mut s = board(false, &[], ARMORED);
            s.play(ID, json!({}));
            // One filler left in hand: three hits of 1, all on Health.
            assert_eq!(unit_hits(&s), vec![1; 3]);
            let health = s.stats(&s.unit_at(P1, 1).id).health + s.stats(&s.unit_at(P2, 1).id).health;
            assert_eq!(health, 30 + 30 - 3, "Armor took none of it");
        }

        #[test]
        fn combo_draws_1_after_another_card_this_turn() {
            let mut s = board(false, &[FREE], BIG);
            s.play(FREE, json!({}));
            s.play(ID, json!({}));
            assert_eq!(s.last_count("drawn"), 1);
        }

        #[test]
        fn without_combo_it_draws_nothing() {
            let mut s = board(false, &[], BIG);
            s.play(ID, json!({}));
            assert_eq!(s.last_count("drawn"), 0);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5239003_each_hit_deals_2() {
            let mut s = board(true, &[], BIG);
            s.play(ID, json!({}));
            assert_eq!(unit_hits(&s), vec![2; 3]);
        }

        #[test]
        fn combo_draws_1_after_another_card_this_turn() {
            let mut s = board(true, &[FREE], BIG);
            s.play(FREE, json!({}));
            s.play(ID, json!({}));
            assert_eq!(s.last_count("drawn"), 1);
        }
    }
}
