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
    const HIT_JOB: &str = "core-016"; // (3) Spell: destroy a Unit.
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
        // Timmy holds lane 5, so the Party fills lanes 1–4; a Hit Job then clears lane 5, the one
        // zone a defender's Death can summon into.
        let mut s = scenario(json!({
            "p1": {
                "hand": [{ "def": PARTY, "radiant": true }, HIT_JOB, HIT_JOB, FILLER],
                "field": [{ "def": TIMMY, "lane": 5 }],
                "mana": 10,
            },
            "p2": { "hand": [FILLER] },
        }));
        s.play(PARTY, json!({}));
        for lane in 1..=4 {
            let defender = s.unit(PlayerId::P1, lane).expect("a radiant defender");
            assert_eq!(defender.def_id, DEFENDER);
            assert!(defender.radiant);
        }
        let timmy = s.unit(PlayerId::P1, 5).expect("Timmy").id.clone();
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": timmy }] }));
        assert!(s.unit(PlayerId::P1, 5).is_none());
        let victim = s.unit(PlayerId::P1, 1).expect("a radiant defender").id.clone();
        s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": victim }] }));
        // Reborn brings the Radiant one back into its own zone, held for it (R64, core #3); its
        // Death summons a base defender into the open lane 5.
        let reborn = s.unit(PlayerId::P1, 1).expect("the reborn defender");
        assert_eq!(reborn.id, victim);
        assert!(reborn.radiant);
        let summoned = s.unit(PlayerId::P1, 5).expect("the summoned defender");
        assert_eq!(summoned.def_id, DEFENDER);
        assert!(!summoned.radiant);
    }
}
