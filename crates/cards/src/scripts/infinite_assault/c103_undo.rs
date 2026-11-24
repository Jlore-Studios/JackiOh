//! SPEC §8.13 IA #103 Undo ((2) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Bounce an enemy Unit. It costs (1) more."
//!   Radiant: "Bounce an enemy Unit. It costs (2) more. Draw a card."
//!
//! C #37 Gravedigger's shape: `bounce` the chosen enemy Unit — a bounce off the field, so it says
//! "Bounce" (R692) and the card returns to its controller's hand (R747) — then `set_cost_mod` the
//! tax, which applies only to a card that reaches the hand (R11). The tax is a fixed price, not a
//! param (R482's `PRICE` row). The Radiant face doubles the step and draws a card.

use jackioh_engine::effects::{bounce, draw, set_cost_mod};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-103";

/// "It costs (1) more", the base face's.
const TAX: i32 = 1;
/// "It costs (2) more", the Radiant face's.
const RADIANT_TAX: i32 = 2;
/// "Draw a card", the Radiant face's.
const DRAW: i32 = 1;

fn undo(tax: i32, draws: bool) -> Script {
    Script {
        targets: vec![TargetDecl::target(1, 1, json!({ "side": "enemy", "of": ["unit"] }))],
        cry: Some(hook(move |_ctx| {
            let mut effects =
                vec![bounce(json_as(json!({ "target": { "of": "chosen" }})))];
            effects.push(set_cost_mod(json_as(json!({
                "target": { "of": "chosen" },
                "amount": tax,
                "inHandOnly": true,
            }))));
            if draws {
                effects.push(draw(json_as(json!({ "count": DRAW }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: undo(TAX, false),
        radiant: undo(RADIANT_TAX, true),
    }
}

// IA #103 Undo — SPEC §8.13 row 103 (R4589008).
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const BODY: &str = "stand-in (2) 5/5";

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }], "library": crate::filler(4) },
            "p2": { "field": [BODY], "library": crate::filler(4) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn r4589008_bounces_and_taxes() {
            let mut s = board(false);
            let body = s.unit_at(P2, 1);
            s.play(ID, pick::targets(pick::instance(&body)));
            s.expect_in_zone(body.id.as_str(), "hand");
            let back = s.hand(P2).into_iter().find(|card| card.def_id == BODY).expect("bounced");
            assert_eq!(back.cost_mod, TAX, "it costs (1) more in hand");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4589008_bounces_taxes_2_and_draws() {
            let mut s = board(true);
            let body = s.unit_at(P2, 1);
            s.play(ID, pick::targets(pick::instance(&body)));
            s.expect_in_zone(body.id.as_str(), "hand");
            let back = s.hand(P2).into_iter().find(|card| card.def_id == BODY).expect("bounced");
            assert_eq!(back.cost_mod, RADIANT_TAX, "it costs (2) more in hand");
            assert_eq!(s.hand(P1).len(), 1, "spent, then drew one");
        }
    }
}
