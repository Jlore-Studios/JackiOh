//! S #31 Drone Core V (SPEC §8.10 row 31): (3) Spell, Drone Core, Rare, a card of the Stinkyscape set.
//!   Base:    "Summon a Drone Corvette."
//!   Radiant: "Summon a Radiant Drone Corvette."
//! Engine: a summon of the Drone Corvette into the caster's leftmost open unit zone (R64), no Cry (R1),
//!   with a full unit row the leftmost open backrow zone (R2248000); with no open zone nothing. The Corvette draws its own two combat keywords as it arrives (R781004).
//!   Radiant: summoned Radiant.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-031";

/// The Drone Corvette token (§8.10 T-drone-corvette).
const CORVETTE: &str = "stinkyscape-t-drone-corvette";

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![summon(json_as(json!({ "defId": CORVETTE, "radiant": radiant })))]
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

// S #31 Drone Core V — SPEC §8.10 row 31, BUILD M12 row S 31.
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::drawn_combat_keywords;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const CORE: &str = "stinkyscape-031";
    const CORVETTE: &str = "stinkyscape-t-drone-corvette";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";

    fn cast(radiant: bool, field: Value) -> Scenario {
        let mut s = scenario(json!({
            "seed": "drone-core-v",
            "p1": { "hand": [{ "def": CORE, "radiant": radiant }, FILLER], "field": field, "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(CORE, json!({}));
        s
    }

    #[test]
    fn r64_summons_an_8_8_corvette_into_the_leftmost_open_zone_with_two_combat_keywords() {
        let s = cast(false, json!([{ "def": VANILLA, "lane": 1 }]));
        let drone = s.unit(P1, 2).expect("the Corvette in lane 2");
        assert_eq!(drone.def_id, CORVETTE);
        assert!(!drone.radiant);
        let stats = s.stats(drone.id.as_str());
        assert_eq!((stats.attack, stats.health), (8, 8));
        let drawn = drawn_combat_keywords(s.card(drone.id.as_str()), None);
        assert_eq!(drawn.len(), 2);
        for keyword in [Keyword::Ranged, Keyword::Flying, drawn[0].clone(), drawn[1].clone()] {
            assert!(stats.keywords.contains(&keyword), "{keyword:?} in {:?}", stats.keywords);
        }
    }

    #[test]
    fn r2248000_a_full_unit_row_puts_the_corvette_in_the_backrow() {
        let mut s = cast(false, json!(vec![VANILLA; 5]));
        let drone = s.backrow(P1, 1).expect("the Corvette in backrow lane 1");
        assert_eq!(drone.def_id, CORVETTE);
        s.expect_in_zone(CORE, "graveyard");
    }

    #[test]
    fn radiant_summons_a_radiant_16_16_with_four_keywords() {
        let s = cast(true, json!([]));
        let drone = s.unit(P1, 1).expect("the Corvette");
        assert_eq!(drone.def_id, CORVETTE);
        assert!(drone.radiant);
        let stats = s.stats(drone.id.as_str());
        assert_eq!((stats.attack, stats.health), (16, 16));
        assert_eq!(drawn_combat_keywords(s.card(drone.id.as_str()), None).len(), 4);
    }
}
