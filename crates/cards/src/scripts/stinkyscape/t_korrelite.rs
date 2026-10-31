//! SPEC §7 Stinkyscape T-korrelite Korrelite, a Resource token of the Stinkyscape set.
//!   Base:    "Deal {damage} damage to a random enemy." (1)
//!   Radiant: "Echo 1\nDeal {damage} damage to a random enemy." (1)
//!
//! A Resource (R1840000), played from its owner's hand at (0) for its Credits (R2250001). "A random
//! enemy" is B5 E37's `damage_split` of one hit of `damage` among the enemies: the enemy hero and
//! every enemy Unit still standing, drawn from the match rng (R60), raised by Spell Damage as any
//! Spell's hit. The Radiant face prints Echo 1 (`StaticFlags.echo`, §6.3): it resolves a second time,
//! a fresh draw. The number is the declared `damage` (R386).

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

// S T-korrelite — SPEC §7, BUILD M12 row S T-korrelite.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{play_resource, scenario};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;
    const BODY: &str = "core-019"; // 9/9
    const FILLER: &str = "core-005";

    fn board(seed: &str, radiant: bool) -> Scenario {
        scenario(json!({
            "seed": seed,
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER] },
            "p2": { "field": [BODY] },
        }))
    }

    fn hits(seed: &str, radiant: bool) -> (i32, i32) {
        let mut s = board(seed, radiant);
        play_resource(&mut s, P1, ID, Value::Null);
        let body = s.unit_at(P2, 1);
        (30 - s.state().players[P2].hero.health, 9 - s.stats(&body).health)
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
    fn radiant_twice() {
        for seed in (0..10).map(|n| format!("korrelite-radiant-{n}")) {
            let (hero, unit) = hits(&seed, true);
            assert_eq!(hero + unit, 2, "{seed}: two hits of 1 (Echo 1)");
        }
    }
}
