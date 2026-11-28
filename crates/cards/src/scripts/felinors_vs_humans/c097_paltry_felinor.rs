//! SPEC §8.14 FvH #97 Paltry Felinor ((1) Unit, Felinor, Rare, 1/1 → 2/2), a card of the held Felinors vs
//! Humans set (R5235000).
//!   Base:    "Death: Summon a random (3) Cost Unit. It is Dormant for {dormant|turn|turns}." (dormant 2)
//!   Radiant: "Death: Summon a random (4) Cost Unit. It is Dormant for {dormant|turn|turns}." (dormant 2)
//!
//! The Death is `summon_random_with` of a random non-token Unit printed at (3) (Radiant (4)) with
//! `dormant_turns` the declared `dormant` (R5239001, R3541002): it sleeps through that many of its
//! controller's turns and awakens at the start of the last (R1839001); a summon runs no Cry (R1), and with no
//! open unit zone nothing is drawn (R129). The Unit is not Radiant on the Radiant face, as the text names no
//! Radiant one. The prices are a filter's, no declared number. Built by S07 (#5257).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-097";

/// The printed price of the Unit each face's Death summons.
const UNIT_COST: i32 = 3;
const RADIANT_UNIT_COST: i32 = 4;

fn paltry(cost: i32) -> Script {
    Script {
        death: Some(hook(move |ctx| {
            vec![summon_random_with(json_as(json!({
                "query": { "type": "Unit", "cost": cost },
                "dormantTurns": param(ctx, "dormant"),
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: paltry(UNIT_COST),
        radiant: paltry(RADIANT_UNIT_COST),
    }
}

// FvH #97 Paltry Felinor — SPEC §8.14 row 97.
#[cfg(test)]
mod tests {
    use super::{ID, RADIANT_UNIT_COST, UNIT_COST};
    use jackioh_engine::testkit::PlayerId::P1;
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const HIT_JOB: &str = "core-016"; // Hit Job: the kill
    const FILLER: &str = "core-005"; // Stockpile: a card to hold
    const VANILLA: &str = "stand-in (1) 4/4";

    fn board(radiant: bool, company: usize) -> Scenario {
        let mut field = vec![json!({ "def": ID, "radiant": radiant })];
        field.extend((0..company).map(|_| json!(VANILLA)));
        scenario(json!({
            "p1": { "hand": [HIT_JOB, FILLER], "field": field, "mana": 10, "library": crate::filler(4) },
            "p2": { "hand": [FILLER], "field": [VANILLA], "library": crate::filler(4) },
        }))
    }

    fn kill(s: &mut Scenario) {
        let me = s.unit_at(P1, 1);
        assert_eq!(me.def_id, ID);
        s.play(HIT_JOB, pick::targets(pick::instance(&me)));
    }

    /// The Unit the Death summoned, by its `summoned` event.
    fn summoned(s: &Scenario) -> CardInstance {
        let ids: Vec<String> = s
            .last_events_of(GameEventType::Summoned)
            .into_iter()
            .filter_map(|event| match event {
                GameEvent::Summoned { player: P1, instance_id, .. } => Some(instance_id),
                _ => None,
            })
            .collect();
        assert_eq!(ids.len(), 1, "one Unit summoned: {ids:?}");
        s.card(ids[0].as_str()).clone()
    }

    fn summons_a_dormant_unit_of(radiant: bool, cost: i32) {
        let mut s = board(radiant, 0);
        kill(&mut s);
        let unit = summoned(&s);
        let def = def_of(Some(s.state()), &unit.def_id);
        assert_eq!(def.type_, CardType::Unit);
        assert_eq!(query_cost(def), cost, "{} is printed at ({cost})", def.id);
        assert!(!def.token, "{} is no token", def.id);
        assert!(!unit.radiant, "the text names no Radiant Unit");
        assert!(is_slumbering(s.card(&unit)), "it is Dormant");
        assert_eq!(slumber_turns_of(s.card(&unit)), Some(2), "for 2 turns");
    }

    mod base {
        use super::*;

        #[test]
        fn r5239001_its_death_summons_a_random_3_cost_unit_dormant_for_2_turns() {
            summons_a_dormant_unit_of(false, UNIT_COST);
        }

        #[test]
        fn r1839001_the_unit_sleeps_through_two_of_its_controllers_turns() {
            let mut s = board(false, 0);
            kill(&mut s);
            let unit = summoned(&s);
            s.end_turn();
            assert!(is_slumbering(s.card(&unit)), "the opponent's turn does not count");
            s.end_turn();
            assert!(is_slumbering(s.card(&unit)), "one of its controller's turns has started");
            s.end_turn();
            s.end_turn();
            assert!(!is_slumbering(s.card(&unit)), "it awakens at the start of the second");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5239001_its_death_summons_a_random_4_cost_unit_dormant_for_2_turns_not_radiant() {
            summons_a_dormant_unit_of(true, RADIANT_UNIT_COST);
        }
    }
}
