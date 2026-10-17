//! M #61 Joon Jorker (SPEC §8.8 row 61): (3) Unit, Human, Common, 7/5 → 14/10.
//!   Both faces: "End of turn: Buff the Unit to the right of this {times|time|times}." — times 5
//!   (Radiant 10).
//! Engine: at its controller's end of turn, separate Buffs (Upgrade, R386) on the acting Unit in the
//!   next lane on its own side (MD-D16, R1060); none or Immutable neighbour: nothing (R129).

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-061";

fn joon() -> Script {
    Script {
        end_of_turn: Some(hook(|ctx| {
            let Some(me) = ctx.live_self() else {
                return vec![];
            };
            let Some(at) = slot_of(&*ctx.state, me) else {
                return vec![];
            };
            // R1081000: standing in the backrow, the Unit to its right is the one standing in the next
            // backrow zone (`is_adjacent_card`), never a backrow card.
            if !in_units_row_or_standing(&*ctx.state, me) {
                return vec![];
            }
            let zones = if at.row == Row::Units { UNIT_ZONES } else { BACKROW_ZONES };
            if at.lane >= zones {
                return vec![];
            }
            let next = ZoneSlot { player: at.player, row: at.row, lane: at.lane + 1 };
            let Some(id) = card_at(&*ctx.state, next)
                .filter(|card| is_adjacent_card(&*ctx.state, me, card))
                .map(|card| card.id.clone())
            else {
                return vec![];
            };
            vec![upgrade(json_as(json!({ "instanceId": id, "times": param(&*ctx, "times") })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = joon();
    // The Radiant face differs only in stats and its `times` value, both catalog data.
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// M #61 Joon Jorker — SPEC §8.8 row 61, BUILD M10 row M 61.
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const JOON: &str = "meditative-061";
    const VANILLA: &str = "core-008";
    const IMMUTABLE: &str = "core-066";
    const FILLER: &str = "core-005";

    fn upgraded_for(events: &[GameEvent], id: &str) -> usize {
        events
            .iter()
            .filter(|event| matches!(event, GameEvent::Upgraded { instance_id, .. } if instance_id == id))
            .count()
    }

    #[test]
    fn r1060_at_your_end_of_turn_five_upgrades_on_the_unit_in_the_next_lane() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [{ "def": JOON, "lane": 2 }, { "def": VANILLA, "lane": 3 }, { "def": VANILLA, "lane": 1 }], "hand": [FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        let right = s.unit(PlayerId::P1, 3).expect("the right neighbour").id.clone();
        let left = s.unit(PlayerId::P1, 1).expect("the left unit").id.clone();
        let before = s.state().rng_cursor;
        s.end_turn();
        let _ = before;
        let lane3 = upgraded_for(s.events(), &right);
        assert_eq!(lane3, 5);
        assert_eq!(upgraded_for(s.events(), &left), 0);
    }

    #[test]
    fn r1060_in_lane_5_nothing_and_no_random_number() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [{ "def": JOON, "lane": 5 }, { "def": VANILLA, "lane": 4 }], "hand": [FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        let cursor = s.state().rng_cursor;
        s.end_turn();
        assert_eq!(s.state().rng_cursor, cursor);
        assert_eq!(
            s.events().iter().filter(|event| matches!(event, GameEvent::Upgraded { .. })).count(),
            0
        );
    }

    #[test]
    fn r1060_an_empty_or_immutable_neighbour_takes_nothing() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [{ "def": JOON, "lane": 1 }], "hand": [FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        s.end_turn();
        assert_eq!(
            s.events().iter().filter(|event| matches!(event, GameEvent::Upgraded { .. })).count(),
            0
        );

        crate::register_all();
        let mut t = scenario(json!({
            "p1": { "field": [{ "def": JOON, "lane": 1 }, { "def": IMMUTABLE, "lane": 2, "radiant": true }], "hand": [FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        t.end_turn();
        assert_eq!(
            t.events().iter().filter(|event| matches!(event, GameEvent::Upgraded { .. })).count(),
            0
        );
    }

    #[test]
    fn r1060_nothing_at_the_opponent_s_end_of_turn() {
        crate::register_all();
        let mut s = scenario(json!({
            "active": "p2",
            "p1": { "field": [{ "def": JOON, "lane": 2 }, { "def": VANILLA, "lane": 3 }], "hand": [FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        s.end_turn();
        assert_eq!(
            s.events().iter().filter(|event| matches!(event, GameEvent::Upgraded { .. })).count(),
            0
        );
    }

    #[test]
    fn r386_times_reads_through_param() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [{ "def": JOON, "lane": 2 }, { "def": VANILLA, "lane": 3 }], "hand": [FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        let right = s.unit(PlayerId::P1, 3).expect("the neighbour").id.clone();
        step_param(s.card_mut(JOON), "times", 1);
        s.end_turn();
        assert_eq!(upgraded_for(s.events(), &right), 6);
    }

    #[test]
    fn radiant_ten_upgrades() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": { "field": [{ "def": JOON, "lane": 2, "radiant": true }, { "def": VANILLA, "lane": 3 }], "hand": [FILLER] },
            "p2": { "hand": [FILLER] },
        }));
        let right = s.unit(PlayerId::P1, 3).expect("the neighbour").id.clone();
        s.end_turn();
        assert_eq!(upgraded_for(s.events(), &right), 10);
    }

    /// R1081000: standing in the backrow, the Unit to its right is the one standing in the next backrow
    /// zone; a backrow card there is no Unit and takes nothing.
    #[test]
    fn r1081000_standing_in_the_backrow_it_buffs_the_unit_standing_to_its_right() {
        crate::register_all();
        let mut s = scenario(json!({
            "p1": {
                "field": [{ "def": JOON, "lane": 2 }, { "def": VANILLA, "lane": 3 }],
                "hand": [FILLER],
            },
            "p2": { "hand": [FILLER] },
        }));
        let joon = s.unit(PlayerId::P1, 2).expect("Joon").id.clone();
        let right = s.unit(PlayerId::P1, 3).expect("the right neighbour").id.clone();
        crate::stand_in_backrow(&mut s, &joon, 2);
        crate::stand_in_backrow(&mut s, &right, 3);
        s.end_turn();
        assert_eq!(upgraded_for(s.events(), &right), 5);

        let mut beside_a_spell = scenario(json!({
            "p1": {
                "field": [{ "def": JOON, "lane": 4 }],
                "backrow": [{ "def": "core-006", "lane": 5 }],
                "hand": [FILLER],
            },
            "p2": { "hand": [FILLER] },
        }));
        let joon = beside_a_spell.unit(PlayerId::P1, 4).expect("Joon").id.clone();
        let well = beside_a_spell.backrow(PlayerId::P1, 5).expect("the Field Spell").id.clone();
        crate::stand_in_backrow(&mut beside_a_spell, &joon, 4);
        beside_a_spell.end_turn();
        assert_eq!(upgraded_for(beside_a_spell.events(), &well), 0);
    }
}
