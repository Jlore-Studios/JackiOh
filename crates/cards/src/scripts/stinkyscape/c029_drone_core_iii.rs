//! S #29 Drone Core III (SPEC §8.10 row 29): (2) Spell, Drone Core, Common, a card of the
//! Stinkyscape set.
//!   Base:    "Summon {interceptors|Drone Interceptor|Drone Interceptors}." — interceptors 2
//!   Radiant: "Summon {interceptors|Radiant Drone Interceptor|Radiant Drone Interceptors}." — interceptors 2
//! Engine: one summon per Interceptor, each into the caster's leftmost open unit zone in turn (R64), no Cry
//!   (R1), and once the unit row is full into the leftmost open backrow zone, where it stands (R2248000),
//!   as many as fit; the count is the declared `interceptors` (R386), C #13 Boots' loop. Each
//!   Interceptor draws its own combat keyword as it arrives (R781004). Radiant: summoned Radiant.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-029";

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

// S #29 Drone Core III — SPEC §8.10 row 29, BUILD M12 row S 29 (R2248000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::stinkyscape_family as fam;

    const CORE: &str = "stinkyscape-029";
    const INTERCEPTOR: &str = "stinkyscape-t-drone-interceptor";
    const VANILLA: &str = "core-008"; // (1) 4/4.

    #[test]
    fn r64_summons_two_left_to_right() {
        let mut s = fam::drone_multi_setup(
            CORE,
            "drone-core-iii",
            false,
            json!([{ "def": VANILLA, "lane": 2 }]),
        );
        s.play(CORE, json!({}));
        let (drone, vanilla) = (
            fam::drone_cell(INTERCEPTOR, false),
            fam::drone_cell(VANILLA, false),
        );
        assert_eq!(
            fam::drone_unit_row(&s),
            vec![drone.clone(), vanilla, drone, None, None]
        );
    }

    #[test]
    fn r64_r2248000_with_one_open_zone_the_second_flies_into_the_backrow() {
        let mut s = fam::drone_multi_setup(CORE, "drone-core-iii", false, json!(vec![VANILLA; 4]));
        s.play(CORE, json!({}));
        let (drone, vanilla) = (
            fam::drone_cell(INTERCEPTOR, false),
            fam::drone_cell(VANILLA, false),
        );
        assert_eq!(
            fam::drone_unit_row(&s),
            vec![
                vanilla.clone(),
                vanilla.clone(),
                vanilla.clone(),
                vanilla,
                drone.clone()
            ]
        );
        assert_eq!(fam::drone_backrow_row(&s), vec![drone, None, None, None, None]);
    }

    #[test]
    fn interceptors_reads_through_param() {
        let mut s = fam::drone_multi_setup(CORE, "drone-core-iii", false, json!([]));
        set_param(s.card_mut(CORE), "interceptors", 3);
        s.play(CORE, json!({}));
        assert_eq!(
            fam::drone_unit_row(&s),
            vec![
                fam::drone_cell(INTERCEPTOR, false),
                fam::drone_cell(INTERCEPTOR, false),
                fam::drone_cell(INTERCEPTOR, false),
                None,
                None
            ]
        );
    }

    #[test]
    fn radiant_summons_two_radiant_ones() {
        let mut s = fam::drone_multi_setup(CORE, "drone-core-iii", true, json!([]));
        s.play(CORE, json!({}));
        assert_eq!(
            fam::drone_unit_row(&s),
            vec![
                fam::drone_cell(INTERCEPTOR, true),
                fam::drone_cell(INTERCEPTOR, true),
                None,
                None,
                None
            ]
        );
    }
}
