//! S #11.3 Flame Strike (SPEC §8.10 row S 11.3; §4.4): (2) Spell, Other-Origins, Token
//! (printed Epic).
//!   Base and Radiant: "Deal {damage} damage to all enemy Units." — 5, 10 on the Radiant face.
//!
//! Engine: `damage_all` over the enemy's Units, one hit each, before one state check (R59); no hero
//! (§6.3's `heroes` stays off). Spell Damage raises each hit (§4.4 step 0). Both faces run this one
//! script: the Radiant face's 10 is its declared `damage` (R386).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-011-3";

pub fn script() -> CardScripts {
    let base = Script {
        cry: Some(hook(|ctx| {
            vec![damage_all(json_as(json!({
                "amount": param(&*ctx, "damage"),
                "side": "enemy",
            })))]
        })),
        ..Script::default()
    };
    // The same script: the Radiant face's 10 is its declared `damage` (R386).
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S #11.3 Flame Strike — SPEC §8.10 row S 11.3, BUILD M12 row S 11.3.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const STRIKE: &str = "stinkyscape-011-3";
    const VANILLA: &str = "core-008"; // (1) Unit 4/4.
    const MENACE: &str = "core-019"; // (3) Unit 9/9.
    const SHIELDED: &str = "core-056"; // (2) Unit 3/2, Divine Shield.
    const FILLER: &str = "core-005";

    mod base {
        use super::*;

        #[test]
        fn deals_5_to_each_enemy_unit_none_to_the_hero_or_to_your_own() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [STRIKE, FILLER], "field": [MENACE] },
                "p2": { "hand": [FILLER], "field": [MENACE, MENACE] },
            }));
            let first = s.unit_at(P2, 1);
            let second = s.unit_at(P2, 2);
            let mine = s.unit_at(P1, 1);

            s.play(STRIKE, json!({}));

            s.expect_stats(&first, json!({ "health": 4 }));
            s.expect_stats(&second, json!({ "health": 4 }));
            s.expect_stats(&mine, json!({ "health": 9 }));
            s.expect_health(P2, HERO_HEALTH);
            s.expect_health(P1, HERO_HEALTH);
        }

        #[test]
        fn a_divine_shield_stops_its_hit() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [STRIKE, FILLER] },
                "p2": { "hand": [FILLER], "field": [SHIELDED, VANILLA] },
            }));
            let shielded = s.unit_at(P2, 1);
            let plain = s.unit_at(P2, 2);

            s.play(STRIKE, json!({}));

            s.expect_stats(&shielded, json!({ "attack": 3, "health": 2 }));
            s.expect_in_zone(&plain, "graveyard");
        }

        #[test]
        fn r386_an_upgrade_makes_each_hit_6() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [STRIKE, FILLER] },
                "p2": { "hand": [FILLER], "field": [MENACE] },
            }));
            step_param(s.card_mut(STRIKE), "damage", 1);
            let unit = s.unit_at(P2, 1);

            s.play(STRIKE, json!({}));

            s.expect_stats(&unit, json!({ "health": 3 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn deals_10_to_each_enemy_unit() {
            crate::register_all();
            let mut s = scenario(json!({
                "p1": { "hand": [{ "def": STRIKE, "radiant": true }, FILLER], "field": [MENACE] },
                "p2": { "hand": [FILLER], "field": [MENACE, MENACE] },
            }));
            let first = s.unit_at(P2, 1);
            let second = s.unit_at(P2, 2);

            s.play(STRIKE, json!({}));

            s.expect_in_zone(&first, "graveyard");
            s.expect_in_zone(&second, "graveyard");
            s.expect_stats(s.unit_at(P1, 1), json!({ "health": 9 }));
            s.expect_health(P2, HERO_HEALTH);
        }
    }
}
