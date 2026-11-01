//! SPEC §7 Stinkyscape T-korrelite Korrelite, a Resource token of the Stinkyscape set (§7's token table).
//!   Base:    "Pierce\nDeal {damage} damage to a random enemy." (1)
//!   Radiant: "Echo 1, Pierce\nDeal {damage} damage to a random enemy." (1)
//!
//! A Resource (R1840000), tagged Resource and Token (§5's tags, issue #2521), played from its owner's
//! hand at (0) for its Credits (R2250000, R2250001). "A random enemy" is B5 E37's `damage_split` of one
//! hit of `damage` among the enemies: the enemy hero and every enemy Unit still standing, drawn from the
//! match rng (R60), raised by Spell Damage as any Spell's hit. Pierce is printed on both faces (catalog
//! data, issue #2521), and §4.4 step 2 reads it off the hit's source, the card resolving (R346,
//! `damage::pierces`): the hit skips the Armor of an Armored Unit and of the enemy hero alike, and
//! nothing else. The Radiant face prints Echo 1 (`StaticFlags.echo`, §6.3): it resolves a second time,
//! a fresh draw, the same instance still in the resolving zone (R98), so the repeat pierces too. The
//! number is the declared `damage` (R386).

use jackioh_engine::effects::damage_split;
use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-t-korrelite";

/// "Echo 1", the Radiant face's.
const RADIANT_ECHO: i32 = 1;

fn face(radiant: bool) -> Script {
    Script {
        static_flags: radiant.then(|| StaticFlags {
            echo: Some(RADIANT_ECHO),
            ..StaticFlags::default()
        }),
        cry: Some(hook(|ctx| {
            let damage = param(&*ctx, "damage");
            vec![damage_split(json_as(json!({
                "amount": damage,
                "perHit": damage,
                "among": "enemies",
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: face(false),
        radiant: face(true),
    }
}

// S T-korrelite — SPEC §7, BUILD M12 row S T-korrelite (issue #2521): played from the hand, 1 damage
// to a random enemy (the hero or a Unit, each seen over the seeds) with Pierce, so an Armored hero or
// Unit takes it whole; damage reads through `param()`; radiant it resolves twice (Echo 1), both hits
// piercing. Tagged Resource and Token.
#[cfg(test)]
mod tests {
    use jackioh_engine::layers::unit_view;
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, play_resource, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const BODY: &str = "core-019"; // 9/9
    const ARMORED: &str = "core-025"; // 7/7, Armor 7
    const FILLER: &str = "core-005";
    /// The enemy hero's Armor in the Pierce tests: more than any hit here, so a hit it took would show.
    const HERO_ARMOR: i32 = 5;

    fn board(seed: &str, radiant: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER] },
            "p2": { "field": [BODY] },
        }))
    }

    /// The damage the enemy hero and the enemy Unit took from one play on `seed`.
    fn hits(seed: &str, radiant: bool) -> (i32, i32) {
        let mut s = board(seed, radiant);
        play_resource(&mut s, P1, ID, Value::Null);
        let body = s.unit_at(P2, 1);
        (30 - s.state().players[P2].hero.health, 9 - s.stats(&body).health)
    }

    /// p2's hero at 30 with `HERO_ARMOR`, beside `field`: one play of the Korrelite on `seed`.
    fn armored_play(seed: &str, radiant: bool, field: Value) -> Scenario {
        let mut s = scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER] },
            "p2": { "field": field, "health": 30, "armor": HERO_ARMOR },
        }));
        play_resource(&mut s, P1, ID, Value::Null);
        s
    }

    /// The damage the Armored hero and the Armored Unit took from one play on `seed`.
    fn armored_hits(seed: &str, radiant: bool) -> (i32, i32) {
        let s = armored_play(seed, radiant, json!([ARMORED]));
        let wall = s.unit_at(P2, 1);
        (30 - s.state().players[P2].hero.health, 7 - s.stats(&wall).health)
    }

    #[test]
    fn tagged_resource_and_token() {
        assert_eq!(card_def(ID).tags, vec![Tag::Resource, Tag::Token]);
    }

    #[test]
    fn prints_pierce_on_both_faces() {
        let def = card_def(ID);
        assert_eq!(serde_json::to_value(&def.base.keywords).unwrap(), json!([{ "kind": "Pierce" }]));
        assert_eq!(serde_json::to_value(&def.radiant.keywords).unwrap(), json!([{ "kind": "Pierce" }]));
        // The card in a hand reads it, as §4.4 step 2 reads its source (`damage::pierces`).
        let s = board("korrelite-keyword", false);
        let card = s.card(ID).clone();
        let keywords = serde_json::to_value(&unit_view(s.state(), &card).keywords).unwrap();
        assert!(keywords.as_array().unwrap().contains(&json!({ "kind": "Pierce" })));
    }

    #[test]
    fn hero_or_unit_over_the_seeds() {
        let hero = seed_where("korrelite-hero", |seed| hits(seed, false) == (1, 0));
        let unit = seed_where("korrelite-unit", |seed| hits(seed, false) == (0, 1));
        assert_ne!(hero, unit);
        for seed in (0..20).map(|n| format!("korrelite-{n}")) {
            let (hero, unit) = hits(&seed, false);
            assert_eq!(hero + unit, 1, "{seed}: one hit of 1, on one enemy");
        }
    }

    #[test]
    fn pierce_skips_an_armored_heros_armor() {
        // No enemy Unit: the hero is the one enemy, and its Armor 5 would have taken the whole hit.
        let mut s = armored_play("korrelite-pierce-hero", false, json!([]));
        s.expect_health(P2, 29);
        assert_eq!(s.state().players[P2].hero.armor, HERO_ARMOR, "the Armor is passed by, not spent");
    }

    #[test]
    fn pierce_skips_an_armored_units_armor() {
        let unit = seed_where("korrelite-pierce-unit", |seed| armored_hits(seed, false) == (0, 1));
        let hero = seed_where("korrelite-pierce-wall-hero", |seed| armored_hits(seed, false) == (1, 0));
        assert_ne!(unit, hero);
        for seed in (0..20).map(|n| format!("korrelite-pierce-{n}")) {
            let (hero, wall) = armored_hits(&seed, false);
            assert_eq!(hero + wall, 1, "{seed}: the hit lands whole through Armor 7 or Armor 5");
        }
    }

    #[test]
    fn damage_reads_through_param() {
        let mut s = board("korrelite-param", false);
        let id = crate::resources_in_hand(&s, P1)[0].id.clone();
        set_param(s.card_mut(id.as_str()), "damage", 3);
        let body = s.unit_at(P2, 1);
        play_resource(&mut s, P1, ID, Value::Null);
        let (hero, unit) = (30 - s.state().players[P2].hero.health, 9 - s.stats(&body).health);
        assert!((hero, unit) == (3, 0) || (hero, unit) == (0, 3), "one hit of 3");
    }

    #[test]
    fn damage_through_param_pierces() {
        let mut s = scenario(json!({
            "seed": "korrelite-param-pierce",
            "p1": { "hand": [{ "def": ID }, FILLER] },
            "p2": { "health": 30, "armor": HERO_ARMOR },
        }));
        let id = crate::resources_in_hand(&s, P1)[0].id.clone();
        set_param(s.card_mut(id.as_str()), "damage", 3);
        play_resource(&mut s, P1, ID, Value::Null);
        s.expect_health(P2, 27);
        assert_eq!(s.state().players[P2].hero.armor, HERO_ARMOR);
    }

    mod radiant {
        use super::*;

        #[test]
        fn twice() {
            for seed in (0..10).map(|n| format!("korrelite-radiant-{n}")) {
                let (hero, unit) = hits(&seed, true);
                assert_eq!(hero + unit, 2, "{seed}: two hits of 1 (Echo 1)");
            }
        }

        #[test]
        fn both_hits_pierce_an_armored_hero() {
            // The hero alone: Echo 1's repeat hits it again, Pierce on both (R346), so 2 through Armor 5.
            let mut s = armored_play("korrelite-radiant-pierce-hero", true, json!([]));
            s.expect_health(P2, 28);
            assert_eq!(s.state().players[P2].hero.armor, HERO_ARMOR, "the Armor is passed by twice");
        }

        #[test]
        fn both_hits_pierce_an_armored_unit() {
            for seed in (0..10).map(|n| format!("korrelite-radiant-pierce-{n}")) {
                let (hero, wall) = armored_hits(&seed, true);
                assert_eq!(hero + wall, 2, "{seed}: two hits of 1, each whole through Armor");
            }
            // Some seed sends both hits into the Armor 7 Unit: 2 off its 7 health.
            seed_where("korrelite-radiant-wall", |seed| armored_hits(seed, true) == (0, 2));
        }
    }
}
