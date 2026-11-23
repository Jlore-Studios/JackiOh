//! SPEC §8.13 IA #18 Prompt Engineer ((2) Unit, Rare, 2/4 → 4/8), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Aura: Your Created cards cost ({discount}) less, but not less than (1)." (discount 1)
//!   Radiant: "Aura: Your Created cards cost ({discount}) less, but not less than (1), and your Created Units
//!             have +{buff}/+{buff}." (discount 1, buff 1)
//!
//! C #77 Anti-Magic Monkey's `costAura` over its controller's cards, as a rule of Created cards (R943)
//! with a floor of (1): the discount stops at (1) and never lifts a card already below it, and two
//! Engineers stop at the same floor (R4577003). The Radiant face adds IA #22 Assault Mainframe's aura
//! over its controller's Created Units on the field.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-018";

/// "but not less than (1)": the floor the discount stops at (R4577003).
const PRICE_FLOOR: i32 = 1;

fn created_discount() -> CostAuraHook {
    read_hook(|args| {
        vec![CostAura {
            rule: CostRule {
                amount: Some(-param(&args, "discount")),
                floor: Some(PRICE_FLOOR),
                created: Some(true),
                ..CostRule::default()
            },
            whose: CostAuraWhose::Yours,
            ban: None,
        }]
    })
}

/// "your Created Units have +N/+N": its controller's Created Units on the field.
fn created_units_aura() -> AuraHook {
    aura_hook(move |ctx| {
        let amount = param(&ctx, "buff");
        vec![AuraEntry {
            applies: Box::new(move |unit: &CardInstance| {
                unit.controller == ctx.self_.controller && in_units_row_or_standing(ctx.state, unit) && is_created(unit)
            }),
            mod_: StatMod {
                attack: Some(amount),
                max_health: Some(amount),
                ..StatMod::default()
            },
        }]
    })
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cost_aura: Some(created_discount()),
            ..Script::default()
        },
        radiant: Script {
            cost_aura: Some(created_discount()),
            aura: Some(created_units_aura()),
            ..Script::default()
        },
    }
}

// IA #18 Prompt Engineer — SPEC §8.13 row 18 (R4577003).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const THREE: &str = "stand-in (3) Spell";
    const TWO: &str = "stand-in (2) Spell";
    const ONE: &str = "stand-in (1) Spell";
    const ZERO: &str = "stand-in (0) Spell";
    const UNIT: &str = "stand-in (1) 2/2";

    fn made(def: &str) -> Value {
        json!({ "def": def, "created": true })
    }

    fn board(engineers: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [made(THREE), made(TWO), made(ONE), made(ZERO), THREE],
                "field": engineers,
            },
            "p2": { "hand": [made(THREE)] },
        }))
    }

    /// p1's hand prices, in hand order.
    fn prices(s: &Scenario, player: PlayerId) -> Vec<i32> {
        s.hand(player).iter().map(|card| cost_now(s.state(), card)).collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4577003_your_created_cards_cost_1_less_never_below_1_and_a_dealt_card_full_price() {
            let s = board(json!([ID]));
            assert_eq!(prices(&s, P1), [2, 1, 1, 0, 3]);
            assert_eq!(prices(&s, P2), [3], "the opponent's Created card is not yours");
        }

        #[test]
        fn r4577003_two_engineers_stop_at_the_same_floor() {
            let s = board(json!([ID, ID]));
            assert_eq!(prices(&s, P1), [1, 1, 1, 0, 3]);
        }

        #[test]
        fn the_discount_ends_when_it_leaves_and_its_units_get_no_buff() {
            let s = scenario(json!({
                "p1": { "hand": [made(THREE)], "field": [{ "def": UNIT, "created": true }] },
            }));
            assert_eq!(prices(&s, P1), [3], "no Engineer on the field");
            let mut s = scenario(json!({
                "p1": { "hand": [made(THREE)], "field": [ID, { "def": UNIT, "created": true }] },
            }));
            assert_eq!(prices(&s, P1), [2]);
            let unit = s.unit_at(P1, 2);
            s.expect_stats(&unit, json!({ "attack": 2, "health": 2 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r4577003_the_same_floored_discount() {
            let s = board(json!([{ "def": ID, "radiant": true }]));
            assert_eq!(prices(&s, P1), [2, 1, 1, 0, 3]);
        }

        #[test]
        fn your_created_units_have_1_1_and_no_other_unit() {
            let mut s = scenario(json!({
                "p1": { "field": [{ "def": ID, "radiant": true }, { "def": UNIT, "created": true }, UNIT] },
                "p2": { "field": [{ "def": UNIT, "created": true }] },
            }));
            let (made, dealt, enemy) = (s.unit_at(P1, 2), s.unit_at(P1, 3), s.unit_at(P2, 1));
            s.expect_stats(&made, json!({ "attack": 3, "health": 3 }));
            s.expect_stats(&dealt, json!({ "attack": 2, "health": 2 }));
            s.expect_stats(&enemy, json!({ "attack": 2, "health": 2 }));
            let engineer = s.unit_at(P1, 1);
            s.expect_stats(&engineer, json!({ "attack": 4, "health": 8 }));
        }
    }
}
