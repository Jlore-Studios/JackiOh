//! S #33 Drone Core VII (SPEC §8.10 row 33): (4) Spell, Drone Core, Epic, a card of the Stinkyscape set.
//!   Base:    "Summon a Drone Corvette and {interceptors|Drone Interceptor|Drone Interceptors}."
//!            — interceptors 2
//!   Radiant: "Summon a Radiant Drone Corvette and {interceptors|Radiant Drone Interceptor|Radiant Drone
//!            Interceptors}." — interceptors 2
//! Engine: two kinds of summon, the Corvette first, then one per Interceptor, each into the caster's
//!   leftmost open unit zone in turn (R64), then the backrow's once the unit row is full, where each stands
//!   (R2248000), no Cry (R1), as many as fit; the Interceptors' count is the
//!   declared `interceptors` (R386). Each drone draws its own combat keywords as it arrives (R781004).
//!   Radiant: all Radiant.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-033";

/// The Drone Interceptor token (§8.10 T-drone-interceptor).
const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
/// The Drone Corvette token (§8.10 T-drone-corvette).
const CORVETTE: &str = "stinkyscape-t-drone-corvette";

fn face(radiant: bool) -> Script {
    Script {
        cry: Some(hook(move |ctx| {
            let mut effects = vec![summon(json_as(json!({ "defId": CORVETTE, "radiant": radiant })))];
            effects.extend(
                (0..param(&*ctx, "interceptors"))
                    .map(|_| summon(json_as(json!({ "defId": INTERCEPTOR, "radiant": radiant })))),
            );
            effects
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

// S #33 Drone Core VII — SPEC §8.10 row 33, BUILD M12 row S 33 (R2248000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const P1: PlayerId = PlayerId::P1;

    const CORE: &str = "stinkyscape-033";
    const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
    const CORVETTE: &str = "stinkyscape-t-drone-corvette";
    const VANILLA: &str = "core-008"; // (1) 4/4.
    const FILLER: &str = "core-005";

    fn setup(radiant: bool, field: Value) -> Scenario {
        scenario(json!({
            "seed": "drone-core-vii",
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

    fn of(def_id: &str, radiant: bool) -> Option<(String, bool)> {
        Some((def_id.to_string(), radiant))
    }

    #[test]
    fn r64_summons_the_corvette_first_then_two_interceptors() {
        let mut s = setup(false, json!([{ "def": VANILLA, "lane": 1 }]));
        s.play(CORE, json!({}));
        assert_eq!(
            row(&s),
            vec![
                of(VANILLA, false),
                of(CORVETTE, false),
                of(INTERCEPTOR, false),
                of(INTERCEPTOR, false),
                None
            ]
        );
    }

    #[test]
    fn r64_r2248000_with_one_open_zone_the_corvette_takes_it_and_the_interceptors_fly_into_the_backrow() {
        let mut s = setup(false, json!(vec![VANILLA; 4]));
        s.play(CORE, json!({}));
        assert_eq!(row(&s)[4], of(CORVETTE, false));
        assert_eq!(
            backrow(&s),
            vec![of(INTERCEPTOR, false), of(INTERCEPTOR, false), None, None, None]
        );
    }

    #[test]
    fn interceptors_reads_through_param() {
        let mut s = setup(false, json!([]));
        set_param(s.card_mut(CORE), "interceptors", 3);
        s.play(CORE, json!({}));
        assert_eq!(
            row(&s),
            vec![
                of(CORVETTE, false),
                of(INTERCEPTOR, false),
                of(INTERCEPTOR, false),
                of(INTERCEPTOR, false),
                None
            ]
        );
    }

    #[test]
    fn radiant_all_three_radiant() {
        let mut s = setup(true, json!([]));
        s.play(CORE, json!({}));
        assert_eq!(
            row(&s),
            vec![
                of(CORVETTE, true),
                of(INTERCEPTOR, true),
                of(INTERCEPTOR, true),
                None,
                None
            ]
        );
    }
}
