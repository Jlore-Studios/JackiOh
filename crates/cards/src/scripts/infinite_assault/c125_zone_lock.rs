//! SPEC §8.13 IA #125 Zone Lock ((1) Spell, Common), a card of the held Infinite Assault set (R4577000).
//!   Base:    "Lock an empty enemy unit zone. Draw a card."
//!   Radiant: "Lock two empty enemy unit zones. Draw a card."
//!
//! Declared zone picks on the enemy's side (R81): only an open unit zone is offered, so an occupied
//! zone and a backrow zone never are (§3.2 Lock, R15). Each pick locks where it stands, then a draw.

use jackioh_engine::effects::{LockArgs, ZoneSpec, draw, lock};
use jackioh_engine::prelude::*;

pub const ID: &str = "infiniteassault-125";

/// "An empty enemy unit zone": the check is data the script's `target_checks` reads — only an open
/// unit zone is offered at all, so the check keeps the unit row's.
const UNIT_ZONE: &str = "unitZone";

/// "Draw a card" on both faces.
const DRAWS: i32 = 1;

fn targets(count: i32) -> Vec<TargetDecl> {
    vec![TargetDecl::zone(count, count, json!({ "side": "enemy", "of": ["zone"], "check": UNIT_ZONE }))]
}

fn zone_lock(count: i32) -> Script {
    Script {
        targets: targets(count),
        target_checks: IndexMap::from([(
            UNIT_ZONE,
            target_check(|args| matches!(args.selection, Selection::Zone { row: Row::Units, .. })),
        )]),
        cry: Some(hook(|ctx| {
            let mut effects: Vec<Effect> = ctx
                .targets
                .iter()
                .filter_map(|selection| match selection {
                    Selection::Zone { player, row, lane } => {
                        let side = if *player == ctx.controller {
                            PlayerSpec::SelfSide
                        } else {
                            PlayerSpec::Enemy
                        };
                        Some(lock(LockArgs {
                            zone: ZoneSpec::Lane { row: *row, lane: *lane, player: Some(side) },
                        }))
                    }
                    _ => None,
                })
                .collect();
            effects.push(draw(json_as(json!({ "count": DRAWS }))));
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: zone_lock(1),
        radiant: zone_lock(2),
    }
}

// IA #125 Zone Lock — SPEC §8.13 row 125.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FILLER: &str = "core-005"; // Stockpile: Draw 2. Heal your hero 2.

    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "seed": "zone-lock",
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "library": crate::filler(3) },
            "p2": { "hand": [FILLER], "field": [VANILLA], "library": crate::filler(3) },
        }))
    }

    /// The zone picks the play is offered, as `(row, lane)` pairs.
    fn offered(s: &Scenario) -> Vec<(Row, i32)> {
        s.play_targets(ID)
            .iter()
            .filter_map(|selection| match selection {
                Selection::Zone { row, lane, .. } => Some((*row, *lane)),
                _ => None,
            })
            .collect()
    }

    fn locked(s: &Scenario, lane: i32) -> bool {
        jackioh_engine::zones::is_locked(s.state(), ZoneSlot { player: P2, row: Row::Units, lane })
    }

    mod base {
        use super::*;

        #[test]
        fn locks_an_empty_enemy_unit_zone_and_draws_a_card() {
            let mut s = board(false);
            // Lane 1 holds a Unit; the backrow is never offered.
            assert!(!offered(&s).contains(&(Row::Units, 1)));
            assert!(offered(&s).iter().all(|(row, _)| *row == Row::Units));
            s.play(ID, json!({ "targets": [{ "pick": "zone", "player": "p2", "row": "units", "lane": 2 }] }));
            assert!(locked(&s, 2));
            assert!(!locked(&s, 3));
            assert_eq!(s.hand(P1).len(), 2, "the played Lock drew one");
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn locks_two_empty_enemy_unit_zones_and_draws_a_card() {
            let mut s = board(true);
            s.play(
                ID,
                json!({ "targets": [
                    { "pick": "zone", "player": "p2", "row": "units", "lane": 2 },
                    { "pick": "zone", "player": "p2", "row": "units", "lane": 3 },
                ] }),
            );
            assert!(locked(&s, 2));
            assert!(locked(&s, 3));
            assert_eq!(s.hand(P1).len(), 2, "the played Lock drew one");
        }

        #[test]
        fn with_one_open_zone_it_locks_that_one() {
            let mut s = scenario(json!({
                "seed": "zone-lock-one",
                "p1": { "hand": [{ "def": ID, "radiant": true }, FILLER], "library": crate::filler(3) },
                "p2": {
                    "hand": [FILLER],
                    "field": [VANILLA, VANILLA, VANILLA, VANILLA],
                    "library": crate::filler(3),
                },
            }));
            assert_eq!(offered(&s), vec![(Row::Units, 5)]);
            s.play(ID, json!({ "targets": [{ "pick": "zone", "player": "p2", "row": "units", "lane": 5 }] }));
            assert!(locked(&s, 5));
        }
    }
}
