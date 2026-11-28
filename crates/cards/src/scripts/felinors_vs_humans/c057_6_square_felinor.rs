//! FvH #57.6 Square Felinor (SPEC §8.14 row 57.6; §10.4 layer 5, R1081000): (1) Unit, Felinor,
//! Other-Origins, Token (printed Epic), 2/4 → 4/8, a token FvH #57.3 Felinor Companion and #58 Call of
//! the Shaped Felinors summon.
//!   Base:    "Aura: Your other Units have +{attack} Attack." (attack 1)
//!   Radiant: "Aura: Your other Units have +{attack} Attack." (attack 2)
//!
//! Engine: TG #3 Group Leader's aura, an Attack-only StatMod read on demand (§10.4 layer 5) for each
//! other Unit its controller has on the field, a Unit standing in the backrow included (R1081000). The
//! faces differ only in the declared `attack` and the catalog's stats. Built by S05 (#5255).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-057-6";

fn square() -> Script {
    Script {
        aura: Some(aura_hook(|ctx| {
            let amount = param(&ctx, "attack");
            vec![AuraEntry {
                applies: Box::new(move |unit: &CardInstance| {
                    unit.controller == ctx.self_.controller
                        // R1081000: a Unit standing in the backrow is one of "your Units".
                        && in_units_row_or_standing(ctx.state, unit)
                        && unit.id != ctx.self_.id
                }),
                mod_: StatMod {
                    attack: Some(amount),
                    ..StatMod::default()
                },
            }]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = square();
    // The same script: the Radiant face's 2 is its declared `attack`, which `param` reads.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #57.6 Square Felinor — SPEC §8.14 row 57.6 (§10.4).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BODY: &str = "stand-in (1) 2/2";
    /// A textless (0) Spell: something left to play, so no turn ends by itself.
    const FILLER: &str = "stand-in (0) Spell";

    /// p1 holds the Square Felinor in lane 1 and a 2/2 in lane 2; p2 a 2/2 in lane 1.
    fn squared(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "radiant": radiant }, BODY] },
            "p2": { "hand": [FILLER], "field": [BODY] },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn s10_4_your_other_units_have_1_attack_and_it_and_the_enemy_do_not() {
            let mut s = squared(false);
            let (square, body, enemy) = (s.unit_at(P1, 1), s.unit_at(P1, 2), s.unit_at(P2, 1));
            s.expect_stats(&body, json!({ "attack": 2 + 1, "health": 2 }));
            s.expect_stats(&square, json!({ "attack": 2, "health": 4 }));
            s.expect_stats(&enemy, json!({ "attack": 2, "health": 2 }));
        }

        #[test]
        fn s10_4_a_unit_played_later_has_it_too() {
            let mut s = scenario(json!({
                "p1": { "hand": [BODY, FILLER], "field": [ID] },
                "p2": { "hand": [FILLER] },
            }));
            s.play(BODY, json!({ "zone": 2 }));
            let body = s.unit_at(P1, 2);
            s.expect_stats(&body, json!({ "attack": 2 + 1, "health": 2 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn s10_4_your_other_units_have_2_attack_and_it_and_the_enemy_do_not() {
            let mut s = squared(true);
            let (square, body, enemy) = (s.unit_at(P1, 1), s.unit_at(P1, 2), s.unit_at(P2, 1));
            s.expect_stats(&body, json!({ "attack": 2 + 2, "health": 2 }));
            s.expect_stats(&square, json!({ "attack": 4, "health": 8 }));
            s.expect_stats(&enemy, json!({ "attack": 2, "health": 2 }));
        }
    }
}
