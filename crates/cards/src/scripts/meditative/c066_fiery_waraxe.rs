//! M #66 Fiery Waraxe (SPEC §8.8 row 66): (2) Field Spell, Common, 3/2 → 6/4.
//!   Base:    "Animated on your turn"
//!   Radiant: "Animated on your turn, Pierce"
//! Engine: "Animated on your turn" (R383, R1062) as C+ #12.8 Frostspatula; keywords only.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-066";

pub fn script() -> CardScripts {
    // Keywords only: the faces' printed stats and keywords are catalog data.
    let base = Script::default();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// M #66 Fiery Waraxe — SPEC §8.8 row 66, BUILD M10 row M 66.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const WARAXE: &str = "meditative-066";
    const FILLER: &str = "core-005";
    const VANILLA: &str = "core-008";

    fn played(radiant: bool, lane: i32) -> Scenario {
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": WARAXE, "radiant": radiant }, FILLER, FILLER] },
            "p2": { "hand": [FILLER], "field": [VANILLA] },
        }));
        s.play(WARAXE, json!({ "zone": lane }));
        s
    }

    #[test]
    fn r383_played_it_animates_into_its_lane_sick_that_turn() {
        crate::register_all();
        let mut s = played(false, 1);
        assert_eq!(s.unit(PlayerId::P1, 1).map(|unit| unit.def_id), Some(WARAXE.to_string()));
        s.expect_refused(|s| s.attack(WARAXE, "hero"));
        s.expect_refused(|s| s.attack(WARAXE, VANILLA));
    }

    #[test]
    fn r1062_at_your_next_start_of_turn_it_attacks_the_hero() {
        crate::register_all();
        let _open = preview_sets(&[SetName::Meditative]);
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": WARAXE }, FILLER], "health": 27 },
            "p2": { "hand": [FILLER], "health": 27 },
        }));
        s.play(WARAXE, json!({ "zone": 1 }));
        s.end_turn().end_turn();
        s.attack(WARAXE, "hero");
        s.expect_health(PlayerId::P2, 27 - 3);
    }

    #[test]
    fn r383_it_returns_at_cleanup_and_p2_cannot_attack_it() {
        crate::register_all();
        let mut s = played(false, 2);
        s.end_turn();
        assert!(s.unit(PlayerId::P1, 2).is_none());
        assert!(s.backrow(PlayerId::P1, 2).is_some());
        s.expect_refused(|s| s.attack(VANILLA, WARAXE));
    }

    #[test]
    fn radiant_6_4_pierce_ignores_hero_armor() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": WARAXE, "radiant": true }, FILLER] },
            "p2": { "hand": [FILLER], "armor": 3 },
        }));
        s.play(WARAXE, json!({ "zone": 1 }));
        s.expect_stats(WARAXE, json!({ "attack": 6, "health": 4 }));
        s.end_turn().end_turn();
        s.attack(WARAXE, "hero");
        s.expect_health(PlayerId::P2, 30 - 6);
    }
}
