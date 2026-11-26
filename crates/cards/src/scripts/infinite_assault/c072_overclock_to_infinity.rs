//! SPEC §8.13 IA #72 Overclock to Infinity ((4) Spell, Mythic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Your cards cost (0) this turn. End of turn: Deal {damage} damage to your hero." (damage 10)
//!   Radiant: "Your cards cost (0) this turn. End of turn: Deal {damage} damage to your hero." (damage 5)
//!
//! Classic #2 The Trickster's price rule set to (0) (`addCostRule` with `setTo`, lasting this
//! turn), reaching every card its controller plays — the (0) is the script's, never a declared
//! number. Like IA #65 Overdrive's, a `delay` to its controller's end of turn (R62) carries the
//! declared `damage` in its data (§10.6), and the resume deals it to its controller's hero.

use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-072";

/// "Your cards cost (0)": the price the rule sets, a structure of the text, never a declared
/// number (R1748003's price).
const SET_COST: i32 = 0;

/// The delayed end-of-turn step the Cry names; it labels the pause.
const BLOWBACK_STEP: &str = "overclocked";

/// §10.6's data carried across the boundary, or the printed value when it is missing.
fn carried(ctx: &EffectContext<'_>, key: &str) -> i32 {
    match ctx.data.get(key).and_then(Value::as_i64) {
        Some(value) => value as i32,
        None => param(ctx, key),
    }
}

/// The blowback, at its R62 end-of-turn point: the carried `damage` to its controller's hero.
fn blowback(ctx: &mut EffectContext<'_>) -> Vec<Effect> {
    vec![damage(json_as(
        json!({ "to": { "of": "selfHero" }, "amount": carried(ctx, "damage") }),
    ))]
}

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![
                add_cost_rule(json_as(json!({
                    "rule": { "setTo": SET_COST },
                    "lasts": "thisTurn",
                }))),
                delay(json_as(json!({
                    "at": { "phase": "end", "player": "self" },
                    "step": BLOWBACK_STEP,
                    "hook": RESUME_HOOK,
                    "data": { "damage": param(&*ctx, "damage") },
                }))),
            ]
        })),
        resume: IndexMap::from([(BLOWBACK_STEP, hook(blowback))]),
        ..Script::default()
    };
    // The same script: the Radiant face's 5 is its declared `damage`.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// IA #72 Overclock to Infinity — SPEC §8.13 row 72.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const TWO: &str = "stand-in (2) 2/2";
    const FILLER: &str = "core-005"; // Stockpile: a card in hand, so no turn ends by itself

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, TWO], "library": crate::filler(4) },
            "p2": { "hand": [FILLER], "library": crate::filler(4) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn your_cards_cost_0_and_your_end_of_turn_deals_10_to_your_hero() {
            let mut s = board(false);
            s.play(ID, json!({}));
            s.expect_mana(P1, 0);
            // The (2) Unit pays (0).
            s.play(TWO, json!({ "zone": 1 }));
            s.expect_mana(P1, 0);
            s.end_turn();
            s.expect_health(P1, HERO_HEALTH - 10);
            s.expect_health(P2, HERO_HEALTH);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn your_cards_cost_0_and_your_end_of_turn_deals_5_to_your_hero() {
            let mut s = board(true);
            s.play(ID, json!({}));
            s.play(TWO, json!({ "zone": 1 }));
            s.expect_mana(P1, 0);
            s.end_turn();
            s.expect_health(P1, HERO_HEALTH - 5);
        }
    }
}
