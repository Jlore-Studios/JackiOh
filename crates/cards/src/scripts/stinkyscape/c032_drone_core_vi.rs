//! S #32 Drone Core VI (SPEC §8.10 row 32): (4) Spell, Drone Core, Epic, a card of the Stinkyscape set.
//!   Base:    "Summon {interceptors|Drone Interceptor|Drone Interceptors}." — interceptors 5
//!   Radiant: "Summon {interceptors|Radiant Drone Interceptor|Radiant Drone Interceptors}." — interceptors 5
//! Engine: as S #29, one summon per Interceptor, each into the caster's leftmost open unit zone in turn
//!   (R64), no Cry (R1): a whole unit row when it is empty, and from a fuller one the rest into the
//!   leftmost open backrow zones, where each stands (R2248000); the count is the
//!   declared `interceptors` (R386). Each Interceptor draws its own combat keyword as it arrives
//!   (R781004). Radiant: summoned Radiant.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-032";

/// The Drone Interceptor token (§8.10 T-drone-interceptor).
const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            (0..param(&*ctx, "interceptors"))
                .map(|_| summon(json_as(json!({ "defId": INTERCEPTOR, "radiant": radiant }))))
                .collect()
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

// S #32 Drone Core VI — SPEC §8.10 row 32, BUILD M12 row S 32 (R2248000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const CORE: &str = "stinkyscape-032";
    const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";

    fn setup(radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "seed": "drone-core-vi",
            "p1": { "hand": [{ "def": CORE, "radiant": radiant }, FILLER], "field": field, "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    fn row(s: &Scenario) -> Vec<Option<(String, bool)>> {
        (1..=5)
            .map(|lane| s.unit(P1, lane).map(|unit| (unit.def_id, unit.radiant)))
            .collect()
    }

    fn backrow(s: &Scenario) -> Vec<Option<(String, bool)>> {
        (1..=5)
            .map(|lane| s.backrow(P1, lane).map(|card| (card.def_id, card.radiant)))
            .collect()
    }

    fn drone(radiant: bool) -> Option<(String, bool)> {
        Some((INTERCEPTOR.to_string(), radiant))
    }

    #[test]
    fn r64_summons_five_into_an_empty_unit_row() {
        let mut s = setup(false, json!([]));
        s.play(CORE, json!({}));
        assert_eq!(row(&s), vec![drone(false); 5]);
    }

    #[test]
    fn r64_r2248000_from_a_fuller_one_the_rest_fly_into_the_backrow() {
        let mut s = setup(false, json!([{ "def": VANILLA, "lane": 3 }]));
        s.play(CORE, json!({}));
        let vanilla = Some((VANILLA.to_string(), false));
        assert_eq!(row(&s), vec![drone(false), drone(false), vanilla, drone(false), drone(false)]);
        assert_eq!(backrow(&s), vec![drone(false), None, None, None, None]);
    }

    #[test]
    fn interceptors_reads_through_param() {
        let mut s = setup(false, json!([]));
        set_param(s.card_mut(CORE), "interceptors", 2);
        s.play(CORE, json!({}));
        assert_eq!(row(&s), vec![drone(false), drone(false), None, None, None]);
    }

    #[test]
    fn radiant_summons_five_radiant_ones() {
        let mut s = setup(true, json!([]));
        s.play(CORE, json!({}));
        assert_eq!(row(&s), vec![drone(true); 5]);
    }
}
