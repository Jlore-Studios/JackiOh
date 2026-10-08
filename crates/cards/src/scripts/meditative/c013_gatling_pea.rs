//! Meditative #13 Gatling Pea (SPEC §8.8 row 13; docs/meditative-set.md M6 #13; R825).
//! (2) Unit, Epic, 2/6 → 4/12.
//!   Base:    "Armor 2 / End of turn: Deal {damage} damage to the enemy hero. Then this damage permanently
//!            goes up by {growth}." (damage 1, growth 1)
//!   Radiant: "Armor 4 / …" (damage 2, growth 2)
//!
//! At its controller's end of turn: one hit on the enemy hero through its Armor and caps (§4.4), then
//! C+ #41 KY's Constant's `set_number` on its own `damage` to the damage plus the growth (R386's
//! `tuning.set`, reported by `numberChanged`). "Permanently" is the card's own tuning, kept in every
//! zone and by a copy (R57, R78), and moved by a Nerf or Buff; the growth comes after the hit, even one
//! Armor took whole, and every extra run grows it (R825). Armor is catalog data. Both faces run one
//! script; `param` reads the face's numbers.

use jackioh_engine::effects::{damage, set_number};
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-013";

pub fn script() -> CardScripts {
    let base = Script {
        end_of_turn: Some(hook(|ctx| {
            let hit = param(&*ctx, "damage");
            let growth = param(&*ctx, "growth");
            vec![
                damage(json_as(json!({ "to": { "of": "enemyHero" }, "amount": hit }))),
                set_number(json_as(json!({
                    "target": { "of": "self" },
                    "which": "param:damage",
                    "value": hit + growth,
                }))),
            ]
        })),
        ..Script::default()
    };
    CardScripts {
        radiant: base.clone(),
        base,
    }
}
