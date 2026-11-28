//! SPEC §8.14 FvH #10 Shadow Judge ((2) Unit, Human, Epic, 4/3 → 6/4), a card of the held Felinors vs Humans
//! set (R5235000). Built by S09 (#5259).
//!   Base:    "Cry: Make a permanent Dormant for {turns|turn|turns}." (2)
//!   Radiant: "Cry: Make a permanent and the permanents adjacent to it Dormant for {turns|turn|turns}." (2)
//!
//! Engine: the Cry (R1) takes one Unit of either side picked with the play, "a permanent" read as a Unit
//! (R3557002, FV2 #10; R81), and `dormant_target` puts it to sleep for the declared `turns` of its
//! controller's turns (R1839001), summoning sick the turn it awakens (R2259000). A Unit already Dormant is
//! passed by. The Radiant face (the designer's ", Cleave") first sleeps the Units beside the pick in its row
//! and on its side, read while it stands awake (`adjacent_to`, Cleave's reading, R2198003), then the pick.
//! The turns are the Cry's number (R1748000).

use jackioh_engine::effects::dormant_target;
use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-010";

/// R81, R3557002: one Unit of either side, picked with the play.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

fn judge(radiant: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let turns = param(ctx, "turns");
            let mut effects = Vec::new();
            if radiant {
                // Read once as the clause begins, while the pick still stands awake (a Dormant Unit is no
                // neighbour), so the neighbours sleep before the pick does.
                effects.push(for_each_card(ForEachCardArgs {
                    cards: Arc::new(|ctx: &mut EffectContext<'_>| {
                        adjacent_to(ctx, &json_as(json!({ "of": "chosen" })), &BoardScope::default())
                            .into_iter()
                            .map(|card| card.id)
                            .collect()
                    }),
                    each: Arc::new(move |instance_id: &str| {
                        dormant_target(json_as(json!({
                            "target": { "of": "instance", "instanceId": instance_id },
                            "turns": turns,
                        })))
                    }),
                }));
            }
            effects.push(dormant_target(json_as(json!({ "target": { "of": "chosen" }, "turns": turns }))));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: judge(false),
        radiant: judge(true),
    }
}

// FvH #10 Shadow Judge — SPEC §8.14 row 10 (R3557002, R1839001, R2198003, R1748000).
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::{filler, scenario};

    const BODY: &str = "stand-in (1) 2/2";
    const FILLER: &str = "core-005"; // Stockpile: a card kept in hand, so no turn ends by itself

    /// p1 holds the Judge; each side has a Unit in lanes 1 to 4.
    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "shadow-judge",
            "p1": {
                "hand": [{ "def": ID, "radiant": radiant }, FILLER],
                "field": [{ "def": BODY, "lane": 1 }, { "def": BODY, "lane": 2 }],
                "library": filler(6),
                "mana": 10,
            },
            "p2": {
                "hand": [FILLER],
                "field": [BODY, BODY, BODY, BODY],
                "library": filler(6),
            },
        }))
    }

    fn slumber(s: &Scenario, id: &str) -> Option<i32> {
        jackioh_engine::slumber::slumber_turns_of(s.card(id))
    }

    #[test]
    fn r81_r3557002_a_unit_of_either_side_is_a_legal_pick() {
        for radiant in [false, true] {
            let s = board(radiant);
            let picks = pick::instance_ids(&s.play_targets(ID));
            assert!(picks.contains(&s.unit_at(P1, 1).id), "radiant {radiant}: a friendly Unit");
            assert!(picks.contains(&s.unit_at(P2, 3).id), "radiant {radiant}: an enemy Unit");
        }
    }

    mod base {
        use super::*;

        #[test]
        fn r1839001_the_pick_sleeps_two_of_its_controllers_turns_and_its_neighbours_stay_awake() {
            let mut s = board(false);
            let target = s.unit_at(P2, 3).id;
            let (left, right) = (s.unit_at(P2, 2).id, s.unit_at(P2, 4).id);
            s.play(ID, pick::targets(pick::id(&target)));
            assert_eq!(slumber(&s, &target), Some(2));
            assert_eq!(slumber(&s, &left), None);
            assert_eq!(slumber(&s, &right), None);
            s.end_turn();
            assert_eq!(slumber(&s, &target), Some(1), "its controller's first turn");
            s.end_turn();
            s.end_turn();
            assert_eq!(slumber(&s, &target), None, "awake at the start of its controller's second turn");
        }

        #[test]
        fn r3557002_a_friendly_unit_may_be_made_dormant_too() {
            let mut s = board(false);
            let own = s.unit_at(P1, 1).id;
            s.play(ID, pick::targets(pick::id(&own)));
            assert_eq!(slumber(&s, &own), Some(2));
            assert_eq!(slumber(&s, &s.unit_at(P1, 2).id), None);
        }

        #[test]
        fn r1748000_the_turns_read_through_param() {
            let mut s = board(false);
            let turns = crate::upgrade_number(&mut s, ID, "turns");
            assert_eq!(turns, 3);
            let target = s.unit_at(P2, 1).id;
            s.play(ID, pick::targets(pick::id(&target)));
            assert_eq!(slumber(&s, &target), Some(turns));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r2198003_the_pick_and_the_units_beside_it_on_its_side_sleep_two_turns() {
            let mut s = board(true);
            let target = s.unit_at(P2, 2).id;
            let (left, right, far) = (s.unit_at(P2, 1).id, s.unit_at(P2, 3).id, s.unit_at(P2, 4).id);
            s.play(ID, pick::targets(pick::id(&target)));
            for id in [&target, &left, &right] {
                assert_eq!(slumber(&s, id), Some(2), "{id}");
            }
            assert_eq!(slumber(&s, &far), None, "two lanes off is no neighbour");
            assert_eq!(slumber(&s, &s.unit_at(P1, 2).id), None, "the Unit across the lane is no neighbour");
        }

        #[test]
        fn r2198003_an_edge_pick_sleeps_its_one_neighbour() {
            let mut s = board(true);
            let own = s.unit_at(P1, 1).id;
            let beside = s.unit_at(P1, 2).id;
            s.play(ID, pick::targets(pick::id(&own)));
            assert_eq!(slumber(&s, &own), Some(2));
            assert_eq!(slumber(&s, &beside), Some(2));
            assert_eq!(slumber(&s, &s.unit_at(P2, 1).id), None);
        }
    }
}
