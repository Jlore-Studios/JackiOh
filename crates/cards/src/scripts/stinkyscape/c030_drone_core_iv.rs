//! S #30 Drone Core IV (SPEC §8.10 row 30): (3) Spell, Drone Core, Rare, a card of the Stinkyscape set.
//!   Base:    "Summon {interceptors|Drone Interceptor|Drone Interceptors}." — interceptors 3
//!   Radiant: "Summon {interceptors|Radiant Drone Interceptor|Radiant Drone Interceptors}." — interceptors 3
//! Engine: as S #29, one summon per Interceptor, each into the caster's leftmost open unit zone in turn
//!   (R64), no Cry (R1), then the backrow's once the unit row is full (R2248000), as many as fit; the count
//!   is the declared `interceptors` (R386). Each Interceptor
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

// S #30 Drone Core IV — SPEC §8.10 row 30, BUILD M12 row S 30 (R2248000).
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
    fn r64_r2248000_what_the_unit_row_cannot_hold_flies_into_the_backrow() {
        let mut s = setup(false, json!([{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 4 }, { "def": VANILLA, "lane": 5 }]));
        s.play(CORE, json!({}));
        assert_eq!(row(&s), vec![vanilla(), drone(false), drone(false), vanilla(), vanilla()]);
        assert_eq!(backrow(&s), vec![drone(false), None, None, None, None]);
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
