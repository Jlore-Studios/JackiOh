//! S #30 Drone Core IV (SPEC §8.10 row 30): (3) Spell, Drone Core, Rare, a card of the held Stinkyscape
//! set (R1420).
//!   Base:    "Summon {interceptors|Drone Interceptor|Drone Interceptors}." — interceptors 3
//!   Radiant: "Summon {interceptors|Radiant Drone Interceptor|Radiant Drone Interceptors}." — interceptors 3
//! Engine: as S #29, one summon per Interceptor, each into the caster's leftmost open unit zone in turn
//!   (R64), no Cry (R1), as many as fit; the count is the declared `interceptors` (R386). Each Interceptor
//!   draws its own combat keyword as it arrives (R781004). Radiant: summoned Radiant.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-030";

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

// S #30 Drone Core IV — SPEC §8.10 row 30, BUILD M12 row S 30: "summons 3 Drone Interceptors, as many as
// the unit row holds; interceptors reads through `param()`; radiant 3 Radiant ones".
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const CORE: &str = "stinkyscape-030";
    const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";

    fn setup(radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "seed": "drone-core-iv",
            "p1": { "hand": [{ "def": CORE, "radiant": radiant }, FILLER], "field": field, "library": filler(4) },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }))
    }

    /// p1's unit row, lane 1 to 5: each zone's definition and face, or `None` when it is empty.
    fn row(s: &Scenario) -> Vec<Option<(String, bool)>> {
        (1..=5)
            .map(|lane| s.unit(P1, lane).map(|unit| (unit.def_id, unit.radiant)))
            .collect()
    }

    fn drone(radiant: bool) -> Option<(String, bool)> {
        Some((INTERCEPTOR.to_string(), radiant))
    }

    fn vanilla() -> Option<(String, bool)> {
        Some((VANILLA.to_string(), false))
    }

    #[test]
    fn r64_summons_three_left_to_right() {
        let mut s = setup(false, json!([{ "def": VANILLA, "lane": 1 }]));
        s.play(CORE, json!({}));
        assert_eq!(row(&s), vec![vanilla(), drone(false), drone(false), drone(false), None]);
    }

    #[test]
    fn r64_as_many_as_the_unit_row_holds() {
        let mut s = setup(false, json!([{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 4 }, { "def": VANILLA, "lane": 5 }]));
        s.play(CORE, json!({}));
        assert_eq!(row(&s), vec![vanilla(), drone(false), drone(false), vanilla(), vanilla()]);
    }

    #[test]
    fn interceptors_reads_through_param() {
        let mut s = setup(false, json!([]));
        set_param(s.card_mut(CORE), "interceptors", 1);
        s.play(CORE, json!({}));
        assert_eq!(row(&s), vec![drone(false), None, None, None, None]);
    }

    #[test]
    fn radiant_summons_three_radiant_ones() {
        let mut s = setup(true, json!([]));
        s.play(CORE, json!({}));
        assert_eq!(row(&s), vec![drone(true), drone(true), drone(true), None, None]);
    }
}
