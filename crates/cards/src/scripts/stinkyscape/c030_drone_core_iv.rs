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

    use crate::stinkyscape_family as fam;

    const CORE: &str = "stinkyscape-030";
    const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";

    #[test]
    fn r64_summons_three_left_to_right() {
        let mut s = fam::drone_multi_setup(
            CORE,
            "drone-core-iv",
            false,
            json!([{ "def": VANILLA, "lane": 1 }]),
        );
        s.play(CORE, json!({}));
        let (drone, vanilla) = (
            fam::drone_cell(INTERCEPTOR, false),
            fam::drone_cell(VANILLA, false),
        );
        assert_eq!(
            fam::drone_unit_row(&s),
            vec![vanilla, drone.clone(), drone.clone(), drone, None]
        );
    }

    #[test]
    fn r64_r2248000_what_the_unit_row_cannot_hold_flies_into_the_backrow() {
        let mut s = fam::drone_multi_setup(
            CORE,
            "drone-core-iv",
            false,
            json!([{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 4 }, { "def": VANILLA, "lane": 5 }]),
        );
        s.play(CORE, json!({}));
        let (drone, vanilla) = (
            fam::drone_cell(INTERCEPTOR, false),
            fam::drone_cell(VANILLA, false),
        );
        assert_eq!(
            fam::drone_unit_row(&s),
            vec![
                vanilla.clone(),
                drone.clone(),
                drone.clone(),
                vanilla.clone(),
                vanilla
            ]
        );
        assert_eq!(fam::drone_backrow_row(&s), vec![drone, None, None, None, None]);
    }

    #[test]
    fn interceptors_reads_through_param() {
        let mut s = fam::drone_multi_setup(CORE, "drone-core-iv", false, json!([]));
        set_param(s.card_mut(CORE), "interceptors", 1);
        s.play(CORE, json!({}));
        assert_eq!(
            fam::drone_unit_row(&s),
            vec![fam::drone_cell(INTERCEPTOR, false), None, None, None, None]
        );
    }

    #[test]
    fn radiant_summons_three_radiant_ones() {
        let mut s = fam::drone_multi_setup(CORE, "drone-core-iv", true, json!([]));
        s.play(CORE, json!({}));
        assert_eq!(
            fam::drone_unit_row(&s),
            vec![
                fam::drone_cell(INTERCEPTOR, true),
                fam::drone_cell(INTERCEPTOR, true),
                fam::drone_cell(INTERCEPTOR, true),
                None,
                None
            ]
        );
    }
}
