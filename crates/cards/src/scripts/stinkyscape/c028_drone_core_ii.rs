//! S #28 Drone Core II (SPEC §8.10 row 28): (1) Spell, Drone Core, Common, a card of the Stinkyscape set.
//!   Base:    "Summon a Drone Interceptor."
//!   Radiant: "Summon a Radiant Drone Interceptor."
//! Engine: a summon of the Drone Interceptor into the caster's leftmost open unit zone (R64), no Cry (R1),
//!   with a full unit row the leftmost open backrow zone (R2248000); with no open zone nothing. The Interceptor draws its own combat keyword as it arrives (R781004).
//!   Radiant: summoned Radiant.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-028";

/// The Drone Interceptor token (§8.10 T-drone-interceptor).
const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |_ctx| {
            vec![summon(json_as(json!({ "defId": INTERCEPTOR, "radiant": radiant })))]
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

// S #28 Drone Core II — SPEC §8.10 row 28, BUILD M12 row S 28 (R2248000).
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::drawn_combat_keywords;
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const CORE: &str = "stinkyscape-028";
    const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";

    fn cast(radiant: bool, field: Value) -> Scenario {
        let mut s = scenario(json!({
            "seed": "drone-core-ii",
            "p1": { "hand": [{ "def": CORE, "radiant": radiant }, FILLER], "field": field, "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.play(CORE, json!({}));
        s
    }

    #[test]
    fn r64_summons_into_the_leftmost_open_zone_with_its_combat_keywords() {
        let s = cast(false, json!([{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 3 }]));
        let drone = s.unit(P1, 2).expect("the Interceptor in lane 2");
        assert_eq!(drone.def_id, INTERCEPTOR);
        assert!(!drone.radiant);
        assert!(s.unit(P1, 4).is_none());
        let drawn = drawn_combat_keywords(s.card(drone.id.as_str()), None);
        assert_eq!(drawn.len(), 1);
        let acting = keywords_of(s.state(), s.card(drone.id.as_str()));
        for keyword in [Keyword::Ranged, Keyword::Flying, drawn[0].clone()] {
            assert!(acting.contains(&keyword), "{keyword:?} in {acting:?}");
        }
    }

    #[test]
    fn r2248000_a_full_unit_row_puts_the_interceptor_in_the_backrow() {
        let mut s = cast(false, json!(vec![VANILLA; 5]));
        for lane in 1..=5 {
            assert_eq!(s.unit(P1, lane).expect("a Vanilla").def_id, VANILLA);
        }
        let drone = s.backrow(P1, 1).expect("the Interceptor in backrow lane 1");
        assert_eq!(drone.def_id, INTERCEPTOR);
        s.expect_in_zone(CORE, "graveyard");
    }

    #[test]
    fn radiant_summons_a_radiant_one() {
        let s = cast(true, json!([]));
        let drone = s.unit(P1, 1).expect("the Interceptor");
        assert_eq!(drone.def_id, INTERCEPTOR);
        assert!(drone.radiant);
        let stats = s.stats(drone.id.as_str());
        assert_eq!((stats.attack, stats.health), (4, 4));
        assert_eq!(drawn_combat_keywords(s.card(drone.id.as_str()), None).len(), 2);
    }
}
