//! M #56 House Party (SPEC §8.8 row 56): (3) Spell, Common.
//!   Base:    "Fill your board with Right-house defenders."
//!   Radiant: "Fill your board with Radiant Right-house defenders."
//! Engine: §7's fill with core-003 Right-house defender (R64), no Cry; radiant defenders are
//!   Radiant and carry Death summoning a base defender.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-056";

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|_ctx| {
                vec![fill_board(json_as(json!({ "defId": "core-003" })))]
            })),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|_ctx| {
                vec![fill_board(json_as(json!({ "defId": "core-003", "radiant": true })))]
            })),
            ..Script::default()
        },
    }
}

// M #56 House Party — SPEC §8.8 row 56, BUILD M10 row M 56.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const PARTY: &str = "meditative-056";
    const DEFENDER: &str = "core-003";
    const TIMMY: &str = "core-011";
    const FILLER: &str = "core-005";

    fn defs_of(s: &Scenario, player: PlayerId) -> Vec<Option<String>> {
        (1..=5)
            .map(|lane| s.unit(player, lane).map(|unit| unit.def_id.clone()))
            .collect()
    }

    #[test]
    fn r64_fills_every_empty_unit_zone_left_to_right_with_base_defenders() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [PARTY], "field": [{ "def": TIMMY, "lane": 2 }], "mana": 4 },
            "p2": { "hand": [FILLER] },
        }));
        s.play(PARTY, json!({}));
        let row = defs_of(&s, PlayerId::P1);
        assert_eq!(row[1], Some(TIMMY.to_string()));
        for lane in [0, 2, 3, 4] {
            assert_eq!(row[lane], Some(DEFENDER.to_string()));
        }
        let defender = s.unit(PlayerId::P1, 1).expect("a defender");
        assert!(!defender.radiant);
    }

    #[test]
    fn r64_a_full_board_takes_none() {
        crate::register_all();
        let full = [TIMMY, "core-t-rush", "core-t-sheep", "core-019", "core-020"];
        let mut s = scenario(json!({ "p1": { "hand": [PARTY], "field": full, "mana": 4 } }));
        s.play(PARTY, json!({}));
        assert_eq!(
            defs_of(&s, PlayerId::P1),
            full.iter().map(|id| Some(id.to_string())).collect::<Vec<_>>()
        );
        s.expect_in_zone(PARTY, "graveyard");
    }

    #[test]
    fn radiant_the_defenders_are_radiant_and_one_dying_summons_a_base_defender() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "hand": [{ "def": PARTY, "radiant": true }, "core-016", FILLER], "mana": 10 },
            "p2": { "hand": [FILLER] },
        }));
        s.play(PARTY, json!({}));
        let first = s.unit(PlayerId::P1, 1).expect("a radiant defender");
        assert_eq!(first.def_id, DEFENDER);
        assert!(first.radiant);
        let victim = first.id.clone();
        s.play("core-016", json!({ "targets": [{ "pick": "instance", "instanceId": victim }] }));
        let back = s.unit(PlayerId::P1, 1).expect("the replacement defender");
        assert_eq!(back.def_id, DEFENDER);
        assert!(!back.radiant);
    }
}
