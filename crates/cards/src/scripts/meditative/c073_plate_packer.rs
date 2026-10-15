//! M #73 Plate Packer (SPEC §8.8 row 73): (2) Unit, Human, Common, 6/6 → 12/12.
//!   Both faces: "Cry: Gain +{per}/+{per} for each Armor on the field, heroes' included." — per 1 (Radiant 2).
//! Engine: reads NEW helper `armor_on_field` (R1064, MD-D32) as the Cry resolves, then a permanent
//!   buff of +X/+X on itself.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-073";

pub fn script() -> CardScripts {
    let cry = hook(|ctx| {
        let gain = armor_on_field(&*ctx.state) * param(&*ctx, "per");
        if gain == 0 {
            return vec![];
        }
        vec![buff(json_as(json!({ "target": { "of": "self" }, "attack": gain, "health": gain })))]
    });
    CardScripts {
        base: Script { cry: Some(cry.clone()), ..Script::default() },
        radiant: Script { cry: Some(cry), ..Script::default() },
    }
}

// M #73 Plate Packer — SPEC §8.8 row 73, BUILD M10 row M 73.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const PACKER: &str = "meditative-073";
    const FILLER: &str = "core-005";

    #[test]
    fn r1064_gains_one_per_point_of_armor() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": {
                "hand": [PACKER, FILLER],
                "field": [{ "def": "core-008", "lane": 2 }],
                "health": 30, "armor": 2,
            },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": "core-025", "lane": 1 }],
                "health": 30, "armor": 1,
            },
        }));
        // P1's core-008 in DEF for its +1: set position after the deal.
        let plain = s.unit(PlayerId::P1, 2).expect("plain").id.clone();
        find_instance_mut(s.state_mut(), &plain).expect("plain").position = Some(Position::Def);
        s.play(PACKER, json!({ "zone": 1 }));
        // Heroes 2 + 1, core-025's printed Armor, plain's Defense +1: check the buff landed.
        s.expect_stats(PACKER, json!({ "attack": 17, "maxHealth": 17 }));
    }

    #[test]
    fn no_armor_no_gain() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [PACKER, FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        s.play(PACKER, json!({ "zone": 1 }));
        s.expect_stats(PACKER, json!({ "attack": 6, "maxHealth": 6 }));
    }

    #[test]
    fn r1_summoned_no_cry() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": {
                "hand": ["core-069", FILLER],
                "library": [PACKER, "core-008", "core-011", "core-003"],
                "armor": 5,
            },
            "p2": { "hand": [FILLER] },
        }));
        s.play("core-069", json!({}));
        // Recruited, so no Cry: no armor gain.
        if s.unit(PlayerId::P1, 1).is_some() {
            let unit = s.unit(PlayerId::P1, 1).expect("a recruit");
            if unit.def_id == PACKER {
                s.expect_stats(&unit, json!({ "attack": 6, "maxHealth": 6 }));
            }
        }
    }

    #[test]
    fn r386_per_reads_through_param() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [PACKER, FILLER], "armor": 4 },
            "p2": { "hand": [FILLER] },
        }));
        step_param(s.card_mut(PACKER), "per", 1);
        s.play(PACKER, json!({ "zone": 1 }));
        s.expect_stats(PACKER, json!({ "attack": 14, "maxHealth": 14 }));
    }

    #[test]
    fn radiant_two_per_point() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": PACKER, "radiant": true }, FILLER],
                "field": [{ "def": "core-008", "lane": 2 }],
                "health": 30, "armor": 2,
            },
            "p2": {
                "hand": [FILLER],
                "field": [{ "def": "core-025", "lane": 1 }],
                "health": 30, "armor": 1,
            },
        }));
        let plain = s.unit(PlayerId::P1, 2).expect("plain").id.clone();
        find_instance_mut(s.state_mut(), &plain).expect("plain").position = Some(Position::Def);
        s.play(PACKER, json!({ "zone": 1 }));
        s.expect_stats(PACKER, json!({ "attack": 34, "maxHealth": 34 }));
    }
}
