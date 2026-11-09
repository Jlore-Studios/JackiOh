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

    use crate::stinkyscape_family as fam;

    const CORE: &str = "stinkyscape-032";
    const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
    const VANILLA: &str = "core-008"; // (1) 4/4.

    #[test]
    fn r64_summons_five_into_an_empty_unit_row() {
        let mut s = fam::drone_multi_setup(CORE, "drone-core-vi", false, json!([]));
        s.play(CORE, json!({}));
        assert_eq!(
            fam::drone_unit_row(&s),
            vec![fam::drone_cell(INTERCEPTOR, false); 5]
        );
    }

    #[test]
    fn r64_r2248000_from_a_fuller_one_the_rest_fly_into_the_backrow() {
        let mut s = fam::drone_multi_setup(
            CORE,
            "drone-core-vi",
            false,
            json!([{ "def": VANILLA, "lane": 3 }]),
        );
        s.play(CORE, json!({}));
        let vanilla = fam::drone_cell(VANILLA, false);
        let drone = fam::drone_cell(INTERCEPTOR, false);
        assert_eq!(
            fam::drone_unit_row(&s),
            vec![drone.clone(), drone.clone(), vanilla, drone.clone(), drone.clone()]
        );
        assert_eq!(fam::drone_backrow_row(&s), vec![drone.clone(), None, None, None, None]);
    }

    #[test]
    fn interceptors_reads_through_param() {
        let mut s = fam::drone_multi_setup(CORE, "drone-core-vi", false, json!([]));
        set_param(s.card_mut(CORE), "interceptors", 2);
        s.play(CORE, json!({}));
        assert_eq!(
            fam::drone_unit_row(&s),
            vec![
                fam::drone_cell(INTERCEPTOR, false),
                fam::drone_cell(INTERCEPTOR, false),
                None,
                None,
                None
            ]
        );
    }

    #[test]
    fn radiant_summons_five_radiant_ones() {
        let mut s = fam::drone_multi_setup(CORE, "drone-core-vi", true, json!([]));
        s.play(CORE, json!({}));
        assert_eq!(
            fam::drone_unit_row(&s),
            vec![fam::drone_cell(INTERCEPTOR, true); 5]
        );
    }
}
