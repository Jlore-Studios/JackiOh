//! SPEC §8.13 IA #161 Defense in Depth ((2) Spell, Epic), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Your hero gains {armor} Armor. If you control a Taunt Unit, your hero gains {extra} more Armor." (8, 4)
//!   Radiant: "Your hero gains {armor} Armor. If you control a Taunt Unit, your hero gains {extra} more Armor." (16, 8)
//!
//! Engine: the declared {armor} Armor, then the declared {extra} more when any of the controller's
//! Units has Taunt — read through the layers (R4591006), so a Unit in Defense Position counts
//! (§4.1). One script: the Radiant face's numbers are its declared params.

use jackioh_engine::effects::gain_hero_armor;
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-161";

fn defense_in_depth() -> Script {
    Script {
        cry: Some(hook(|ctx| {
            let mut effects = vec![gain_hero_armor(json_as(json!({
                "amount": param(&*ctx, "armor"),
            })))];
            // R4591006: "a Taunt Unit" reads layered keywords, so a Unit in Defense counts.
            let taunt = active_units_of(&*ctx.state, ctx.controller)
                .iter()
                .any(|unit| unit_has(&*ctx.state, unit, KeywordKind::Taunt));
            if taunt {
                effects.push(gain_hero_armor(json_as(json!({
                    "amount": param(&*ctx, "extra"),
                }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // One script: the Radiant face's 16 and 8 are its declared params, read off the running face.
    let base = defense_in_depth();
    CardScripts {
        base: base.clone(),
        radiant: base,
    }
}

// IA #161 Defense in Depth — SPEC §8.13 row 161 (R4591006).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const UNIT: &str = "stand-in (1) 2/2";
    const TAUNT: &str = "stand-in (1) 2/2 Taunt";

    fn board(radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }],
                "field": field,
                "library": crate::filler(3),
                "mana": 8,
            },
            "p2": { "library": crate::filler(3) },
        }))
    }

    mod base {
        use super::*;

        #[test]
        fn with_no_taunt_your_hero_gains_8_armor() {
            let mut s = board(false, json!([UNIT]));
            s.play(ID, json!({}));
            assert_eq!(s.state().players[P1].hero.armor, 8);
        }

        #[test]
        fn r4591006_with_taunt_your_hero_gains_12_armor() {
            let mut s = board(false, json!([TAUNT]));
            s.play(ID, json!({}));
            assert_eq!(s.state().players[P1].hero.armor, 12);
        }

        #[test]
        fn r4591006_a_defense_position_unit_counts_as_taunt() {
            let mut s = board(false, json!([{ "def": UNIT, "position": "DEF" }]));
            s.play(ID, json!({}));
            assert_eq!(s.state().players[P1].hero.armor, 12);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn with_no_taunt_your_hero_gains_16_armor() {
            let mut s = board(true, json!([UNIT]));
            s.play(ID, json!({}));
            assert_eq!(s.state().players[P1].hero.armor, 16);
        }

        #[test]
        fn r4591006_with_taunt_your_hero_gains_24_armor() {
            let mut s = board(true, json!([TAUNT]));
            s.play(ID, json!({}));
            assert_eq!(s.state().players[P1].hero.armor, 24);
        }
    }
}
