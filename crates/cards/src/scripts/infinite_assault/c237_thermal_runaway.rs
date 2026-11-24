//! IA #237 Thermal Runaway (SPEC §8.13 row 237), a card of the held Infinite Assault set
//! (R4577000).
//!   Base:    "Charge. Aura: Your cards cost ({discount}) less. End of turn: Deal {damage} damage to
//!            each hero." (1, 3)
//!   Radiant: "Charge. Aura: Your cards cost ({discount}) less. End of turn: Deal {damage} damage to
//!            the enemy hero." (1, 3)
//!
//! Charge is catalog data. The aura discounts its controller's cards, with no floor and no Created
//! gate. The End of turn hits its controller's hero then the enemy's — Radiant only the enemy's —
//! so both heroes at 0 is a draw.
//!
//! Engine: `costAura`, `damage` on `endOfTurn`. Built by C10 (#4594).

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-237";

fn discount_aura() -> CostAuraHook {
    read_hook(|args| {
        vec![CostAura {
            rule: CostRule {
                amount: Some(-param(&args, "discount")),
                ..CostRule::default()
            },
            whose: CostAuraWhose::Yours,
            ban: None,
        }]
    })
}

fn runaway(radiant: bool) -> Script {
    Script {
        cost_aura: Some(discount_aura()),
        end_of_turn: Some(hook(move |ctx| {
            let amount = param(&*ctx, "damage");
            let mut effects = Vec::new();
            if !radiant {
                effects.push(damage(json_as(json!({
                    "to": { "of": "selfHero" },
                    "amount": amount,
                }))));
            }
            effects.push(damage(json_as(json!({
                "to": { "of": "enemyHero" },
                "amount": amount,
            }))));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts { base: runaway(false), radiant: runaway(true) }
}

// IA #237 Thermal Runaway — SPEC §8.13 row 237.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::scenario;

    const STOCKPILE: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    mod base {
        use super::*;

        #[test]
        fn own_cards_cost_1_less_the_enemys_not() {
            let s = scenario(json!({
                "seed": "runaway-discount",
                "p1": { "field": [{ "def": ID }], "hand": [STOCKPILE], "library": crate::filler(4) },
                "p2": { "hand": [STOCKPILE], "library": crate::filler(4) },
            }));
            let mine = s.hand(P1).into_iter().find(|card| card.def_id == STOCKPILE).expect("a Stockpile");
            let theirs = s.hand(P2).into_iter().find(|card| card.def_id == STOCKPILE).expect("a Stockpile");
            assert_eq!(cost_now(s.state(), &mine), 1 - 1);
            assert_eq!(cost_now(s.state(), &theirs), 1);
        }

        #[test]
        fn end_of_turn_hits_both_heroes_a_double_ko_is_a_draw() {
            let mut s = scenario(json!({
                "seed": "runaway-draw",
                "active": "p1",
                "p1": { "field": [{ "def": ID }], "hand": [STOCKPILE], "library": crate::filler(4), "health": 3 },
                "p2": { "hand": [STOCKPILE], "library": crate::filler(4), "health": 3 },
            }));
            s.end_turn();
            let over = s.events().iter().find_map(|event| match event {
                GameEvent::GameOver { winner, .. } => Some(winner.clone()),
                _ => None,
            });
            assert_eq!(over, Some(Winner::Draw), "both heroes at 0 is a draw");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn end_of_turn_hits_only_the_enemy_hero() {
            let mut s = scenario(json!({
                "seed": "runaway-radiant",
                "active": "p1",
                "p1": { "field": [{ "def": ID, "radiant": true }], "hand": [STOCKPILE], "library": crate::filler(4), "health": 10 },
                "p2": { "hand": [STOCKPILE], "library": crate::filler(4), "health": 10 },
            }));
            s.end_turn();
            assert_eq!(s.state().players[P1].hero.health, 10, "its own hero spared");
            assert_eq!(s.state().players[P2].hero.health, 10 - 3, "the enemy hero hit");
        }
    }
}
