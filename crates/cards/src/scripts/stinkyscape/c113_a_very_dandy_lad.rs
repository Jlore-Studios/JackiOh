//! S #113 A Very Dandy Lad (SPEC §8.10 row 113): (4) Unit, Human, Epic, 9/15 → 18/30, of the
//! Stinkyscape set.
//!   Base:    "Tribute 2 / Aura: All permanents that cost (2) or less are exiled."
//!   Radiant: "Tribute 2 / Aura: All enemy permanents that cost (2) or less are exiled."
//! Engine: Tribute 2 is its play's cost, §6.3's `staticFlags.tribute` (R101), on both faces. The aura is
//! an exile aura (`Script.exile_auras`, R1850002): both rows, `maxCost` 2, the cost each card has now
//! (R66). At every state check each face-up card on the field it reaches is exiled, never this card, a
//! face-down or a Dormant one, and an arrival's Cry resolves first; it ends as the Lad leaves the field.
//! The Radiant face reaches the enemy side only.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscape-113";

/// §6.3 "Tribute 2": two of your Units, or one Sheep Token, worth 2 (R101).
const TRIBUTE_COST: i32 = 2;
/// The aura's printed filter: "cost (2) or less", static data, no param (ST-S17-11).
const MAX_COST: i32 = 2;

fn lad(side: &str) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(TRIBUTE_COST),
            ..StaticFlags::default()
        }),
        exile_auras: vec![ExileAura {
            scope: json_as(json!({ "side": side, "rows": ["units", "backrow"] })),
            cost: Some(json_as(json!({ "maxCost": MAX_COST }))),
            not_keywords: Vec::new(),
        }],
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: lad("any"),
        radiant: lad("enemy"),
    }
}

// S #113 A Very Dandy Lad — SPEC §8.10 row 113, BUILD M12 row S 113 (R1850002).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{filler, scenario};

    const VANILLA: &str = "core-008"; // (1) 4/4, a Human: a tribute.
    const TIMMY: &str = "core-011"; // (1) 3/3.
    const POINTMASTER: &str = "core-020"; // (2) 7/1.
    const MENACE: &str = "core-019"; // (3) 9/9.
    const ARMOR: &str = "core-073"; // (2) Field Spell, face-up.
    const SHEEPISH: &str = "core-041"; // (1) Trap, face-down.
    const FILLER: &str = "core-005";
    const HIT_JOB: &str = "core-016"; // (3) Spell: "Destroy a Unit."

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": [VANILLA, VANILLA, TIMMY, POINTMASTER],
                "backrow": [{ "def": SHEEPISH, "faceUp": false }],
                "library": filler(4),
                "mana": 10,
            },
            "p2": {
                "hand": [FILLER],
                "field": [VANILLA, MENACE],
                "backrow": [ARMOR],
                "library": filler(4),
            },
        }))
    }

    fn exiled(s: &Scenario) -> Vec<String> {
        s.last_events()
            .iter()
            .filter_map(|event| match event {
                GameEvent::Exiled { instance_id, .. } => Some(s.card(instance_id.as_str()).def_id.clone()),
                _ => None,
            })
            .collect()
    }

    fn play_lad(s: &mut Scenario) {
        let tributes = [s.unit_at(P1, 1).id, s.unit_at(P1, 2).id];
        s.play(ID, json!({ "tributes": tributes, "zone": 1 }));
    }

    #[test]
    fn r101_it_needs_its_tribute_2() {
        let mut s = scenario(json!({
            "p1": { "hand": [ID, FILLER], "field": [MENACE], "library": filler(4), "mana": 10 },
            "p2": { "hand": [FILLER], "library": filler(4) },
        }));
        s.expect_refused(|s| s.play(ID, json!({})));
    }

    #[test]
    fn r1850002_every_face_up_permanent_of_cost_2_or_less_is_exiled_on_both_sides() {
        let mut s = board(false);
        play_lad(&mut s);
        // R68's board order: the active player's Units, their backrow, then the opponent's.
        assert_eq!(exiled(&s), vec![TIMMY, POINTMASTER, VANILLA, ARMOR]);
        assert_eq!(s.unit_at(P1, 1).def_id, ID, "never the Lad itself");
        assert_eq!(s.unit_at(P2, 2).def_id, MENACE, "a (3) stays");
        assert!(s.backrow(P1, 1).is_some(), "a face-down Trap stays");
        s.expect_stats(ID, json!({ "attack": 9, "health": 15 }));
    }

    #[test]
    fn r1850002_a_cheap_permanent_played_later_is_exiled() {
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "field": [ID], "library": filler(4) },
            "p2": { "hand": [VANILLA, FILLER], "library": filler(4), "mana": 10 },
            "active": "p2",
        }));
        s.play(VANILLA, json!({}));
        assert_eq!(exiled(&s), vec![VANILLA]);
        assert!(s.unit(P1, 1).is_some(), "the Lad stays");
    }

    #[test]
    fn r1850002_leaving_the_field_ends_it() {
        let mut s = scenario(json!({
            "p1": { "hand": [FILLER], "field": [ID], "library": filler(4) },
            "p2": { "hand": [HIT_JOB, VANILLA, FILLER], "library": filler(4), "mana": 10 },
            "active": "p2",
        }));
        let lad = s.unit_at(P1, 1).id;
        s.play(HIT_JOB, json!({ "targets": pick::id(&lad) }));
        s.expect_in_zone(&lad, "graveyard");
        s.play(VANILLA, json!({ "zone": 1 }));
        assert!(exiled(&s).is_empty());
        assert!(
            s.unit(P2, 1).is_some_and(|unit| unit.def_id == VANILLA),
            "a (1) now stays"
        );
    }

    #[test]
    fn r275_radiant_18_30_exiles_the_enemys_alone() {
        let mut s = board(true);
        play_lad(&mut s);
        assert_eq!(exiled(&s), vec![VANILLA, ARMOR]);
        assert!(s.unit(P1, 3).is_some_and(|unit| unit.def_id == TIMMY));
        assert!(s.unit(P1, 4).is_some_and(|unit| unit.def_id == POINTMASTER));
        s.expect_stats(ID, json!({ "attack": 18, "health": 30 }));
    }
}
