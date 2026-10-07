//! C #71 Lane Eater (SPEC §8.6 row 71). (3) Unit, Common, 4/4 → 8/8.
//!   Base:    "Cry: Destroy every other card in this lane. Lock this lane."
//!   Radiant: "Cry: Destroy the enemy cards in this lane. Lock the enemy side of this lane."
//!   Engine:  "The lane's four zones (§3.1): Lane Eater's own, your backrow zone and both of the
//!            opponent's. The top card of each other zone is destroyed (a dormant card under a Stack is
//!            not on the field, R13, and resumes); then all four are Locked (Lock / Unlock, §6.3). Lock
//!            evicts nothing, so Lane Eater stays and its zone stays Locked after it leaves. Radiant: the
//!            opponent's two zones only. Unlabelled one-time text on a Unit is its Cry. Tunes: none."
//!
//! "This lane" is the column Lane Eater stands in (§3.1: your lane N faces the opponent's lane N), read
//! as the Cry resolves. The destroy marks the card acting in each other zone — the top of a unit pile or
//! of a backrow pile (R13), a face-down trap included — and the state check after the Cry collects them
//! together (R59); Indestructible ones stay (R46), and a card beneath a destroyed top resumes there.
//! Then `lockLane` Locks the zones (both sides on the base face, the enemy side on the Radiant face)
//! before that check, so a destroyed Reborn Unit finds its zone Locked and does not return (R47). A
//! Lock evicts nothing: Lane Eater, the survivors and the resumed cards stay, later summons and plays
//! into those zones are refused (§3.2), and the zones stay Locked after Lane Eater leaves.
//!
//! Rulings: R13, R46, R47, R59; §3.1, §3.2. Its proof: `test/classic/071-lane-eater.test.ts`.

use jackioh_engine::effects::{destroy, lock_lane};
use jackioh_engine::prelude::*;

pub const ID: &str = "classic-071";

/// The zones of this lane the face destroys in: the base face's three others, the Radiant's two enemy ones.
fn lane_zones(me: PlayerId, enemy_only: bool) -> Vec<(PlayerId, Row)> {
    let enemy = opponent_of(me);
    let theirs = vec![(enemy, Row::Units), (enemy, Row::Backrow)];
    if enemy_only {
        theirs
    } else {
        let mut zones = vec![(me, Row::Backrow)];
        zones.extend(theirs);
        zones
    }
}

/// Destroy the card acting in each of those zones of Lane Eater's lane, as the Cry resolves.
fn destroy_in_lane(ctx: &EffectContext, enemy_only: bool) -> Vec<Effect> {
    let at = ctx.live_self().and_then(|me| slot_of(&*ctx.state, me));
    let Some(at) = at else {
        return Vec::new();
    };
    if at.row != Row::Units {
        return Vec::new();
    }
    lane_zones(ctx.controller, enemy_only)
        .into_iter()
        .filter_map(|(player, row)| {
            card_at(&*ctx.state, ZoneSlot { player, row, lane: at.lane }).map(|card| {
                destroy(json_as(json!({ "target": { "of": "instance", "instanceId": card.id } })))
            })
        })
        .collect()
}

pub fn script() -> CardScripts {
    CardScripts {
        base: Script {
            cry: Some(hook(|ctx| {
                let mut effects = destroy_in_lane(ctx, false);
                effects.push(lock_lane(json_as(json!({}))));
                effects
            })),
            ..Script::default()
        },
        radiant: Script {
            cry: Some(hook(|ctx| {
                let mut effects = destroy_in_lane(ctx, true);
                effects.push(lock_lane(json_as(json!({ "side": "enemy" }))));
                effects
            })),
            ..Script::default()
        },
    }
}

// C #71 Lane Eater (SPEC §8.6 row 71; BUILD M9 row C 71). (3) Unit, Common, 4/4 → 8/8: Cry: destroy
// every other card in this lane, then Lock this lane. Radiant: the enemy cards and the enemy side only.

/// `describe("C #71 Lane Eater")`.
#[cfg(test)]
mod tests {
    use super::*;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const LANE_EATER: &str = "classic-071";
    const VANILLA: &str = "core-008"; // 4/4
    const TIMMY: &str = "core-011"; // 3/3
    const FIENDER: &str = "core-092"; // Felinor Fiender: Stack
    const DEFENDER: &str = "core-003"; // Right-house defender: Taunt, Divine Shield, Reborn
    const THE_ROCK: &str = "core-066"; // Tribute 1, Indestructible
    const HIT_JOB: &str = "core-016";
    const MANA_WELL: &str = "core-006"; // Field Spell
    const GOING_LONG: &str = "core-084"; // Field Spell
    const SHEEPISH: &str = "core-041"; // Trap
    const GIFTED: &str = "core-064"; // Field Spell
    const TOKEN_MAKER: &str = "core-015"; // Me and Mr Token: Cry: summon a Rush Token
    const STOCKPILE: &str = "core-005";

    use crate::scenario;

    use crate::merged;

    /// TS `SPARE: SideSetup`.
    fn spare() -> Value {
        json!({ "hand": [STOCKPILE], "library": [VANILLA, VANILLA] })
    }

    fn locked(s: &Scenario, player: PlayerId, row: Row, lane: i32) -> bool {
        is_locked(s.state(), ZoneSlot { player, row, lane })
    }

    /// The four zones of lane 2, and whether each is Locked: p1 units, p1 backrow, p2 units, p2 backrow.
    fn lane_two_locks(s: &Scenario) -> Vec<bool> {
        vec![
            locked(s, P1, Row::Units, 2),
            locked(s, P1, Row::Backrow, 2),
            locked(s, P2, Row::Units, 2),
            locked(s, P2, Row::Backrow, 2),
        ]
    }

    /// Lane 2 full on both sides, and cards in lanes 1 and 3 that the Cry must not touch.
    fn board(radiant: bool) -> Scenario {
        scenario(json!({
            "p1": {
                "hand": [{ "def": LANE_EATER, "radiant": radiant }, STOCKPILE],
                "field": [{ "def": TIMMY, "lane": 1 }],
                "backrow": [{ "def": MANA_WELL, "lane": 2 }, { "def": GIFTED, "lane": 1 }],
                "library": spare()["library"],
                "mana": 10,
            },
            "p2": merged(
                json!({
                    "field": [{ "def": VANILLA, "lane": 2 }, { "def": TIMMY, "lane": 3 }],
                    "backrow": [{ "def": SHEEPISH, "lane": 2, "faceUp": false }, { "def": GOING_LONG, "lane": 3 }],
                }),
                spare(),
            ),
        }))
    }

    fn def_ids(cards: Vec<Option<CardInstance>>) -> Vec<Option<String>> {
        cards.into_iter().map(|card| card.map(|card| card.def_id)).collect()
    }

    /// `describe("base")`.
    mod base {
        use super::*;

        /// §8.6 Cry: the top card of every other zone in its lane is destroyed — your backrow zone and both of the opponent's
        #[test]
        fn s8_6_cry_the_top_card_of_every_other_zone_in_its_lane_is_destroyed_your_backrow_zone_and_both_of_the_opponents() {
            let mut s = board(false);
            let vanilla = s.unit(P2, 2).expect("p2's lane-2 unit");
            let trap = s.backrow(P2, 2).expect("p2's lane-2 trap");
            let well = s.backrow(P1, 2).expect("p1's lane-2 Field Spell");
            s.play(LANE_EATER, json!({ "zone": 2 }));
            s.expect_in_zone(&vanilla, "graveyard")
                .expect_in_zone(&trap, "graveyard")
                .expect_in_zone(&well, "graveyard");
            // Lane Eater stays; lanes 1 and 3 are untouched.
            assert_eq!(s.unit(P1, 2).map(|card| card.def_id).as_deref(), Some(LANE_EATER));
            assert_eq!(
                def_ids(vec![s.unit(P1, 1), s.backrow(P1, 1), s.unit(P2, 3), s.backrow(P2, 3)]),
                vec![
                    Some(TIMMY.to_string()),
                    Some(GIFTED.to_string()),
                    Some(TIMMY.to_string()),
                    Some(GOING_LONG.to_string()),
                ]
            );
        }

        /// §3.2 then all four zones of the lane are Locked, and a Lock evicts nothing: Lane Eater stays in its Locked zone
        #[test]
        fn s3_2_then_all_four_zones_of_the_lane_are_locked_and_a_lock_evicts_nothing_lane_eater_stays_in_its_locked_zone() {
            let mut s = board(false);
            s.play(LANE_EATER, json!({ "zone": 2 }));
            assert_eq!(lane_two_locks(&s), vec![true, true, true, true]);
            assert_eq!(
                s.last_events()
                    .iter()
                    .filter(|event| event.event_type() == GameEventType::Locked)
                    .count(),
                4
            );
            assert_eq!(s.unit(P1, 2).map(|card| card.def_id).as_deref(), Some(LANE_EATER));
            assert_eq!(vec![locked(&s, P1, Row::Units, 1), locked(&s, P2, Row::Units, 3)], vec![false, false]);
        }

        /// R13 a dormant card beneath a destroyed top resumes, and stays in its Locked zone
        #[test]
        fn r13_a_dormant_card_beneath_a_destroyed_top_resumes_and_stays_in_its_locked_zone() {
            let mut s = scenario(json!({
                "p1": { "hand": [LANE_EATER, STOCKPILE], "library": spare()["library"], "mana": 10 },
                "p2": merged(json!({ "field": [{ "def": VANILLA, "lane": 2 }, { "def": FIENDER, "lane": 2, "stack": true }] }), spare()),
            }));
            let top = s.unit(P2, 2).expect("p2's lane-2 top");
            s.play(LANE_EATER, json!({ "zone": 2 }));
            s.expect_in_zone(&top, "graveyard");
            assert_eq!(s.unit(P2, 2).map(|card| card.def_id).as_deref(), Some(VANILLA));
            assert!(locked(&s, P2, Row::Units, 2));
        }

        /// R46 an Indestructible card stays, in a Locked zone
        #[test]
        fn r46_an_indestructible_card_stays_in_a_locked_zone() {
            let mut s = scenario(json!({
                "p1": { "hand": [LANE_EATER, STOCKPILE], "library": spare()["library"], "mana": 10 },
                "p2": merged(json!({ "field": [{ "def": THE_ROCK, "lane": 2 }] }), spare()),
            }));
            s.play(LANE_EATER, json!({ "zone": 2 }));
            assert_eq!(s.unit(P2, 2).map(|card| card.def_id).as_deref(), Some(THE_ROCK));
            assert!(locked(&s, P2, Row::Units, 2));
        }

        /// R47 R688 a destroyed Reborn Unit returns to its now-Locked zone: a Lock refuses plays, not the return
        #[test]
        fn r47_r688_a_destroyed_reborn_unit_returns_to_its_now_locked_zone_a_lock_refuses_plays_not_the_return() {
            let mut s = scenario(json!({
                "p1": { "hand": [LANE_EATER, STOCKPILE], "library": spare()["library"], "mana": 10 },
                "p2": merged(json!({ "field": [{ "def": DEFENDER, "lane": 2 }] }), spare()),
            }));
            let defender_id = s.unit(P2, 2).expect("p2's defender").id;
            s.play(LANE_EATER, json!({ "zone": 2 }));
            assert_eq!(s.unit(P2, 2).map(|card| card.id), Some(defender_id));
            assert_eq!(s.unit(P2, 2).map(|card| card.def_id).as_deref(), Some(DEFENDER));
            assert!(locked(&s, P2, Row::Units, 2));
        }

        /// R13 a backrow pile: only its top is destroyed, and the card beneath resumes in the Locked zone
        #[test]
        fn r13_a_backrow_pile_only_its_top_is_destroyed_and_the_card_beneath_resumes_in_the_locked_zone() {
            let mut s = scenario(json!({
                "p1": { "hand": [LANE_EATER, STOCKPILE], "library": spare()["library"], "mana": 10 },
                "p2": merged(
                    json!({ "backrow": [{ "def": GOING_LONG, "lane": 2 }, { "def": MANA_WELL, "lane": 2, "stack": true }] }),
                    spare(),
                ),
            }));
            let top = s.backrow(P2, 2).expect("p2's lane-2 backrow top");
            s.play(LANE_EATER, json!({ "zone": 2 }));
            s.expect_in_zone(&top, "graveyard");
            assert_eq!(s.backrow(P2, 2).map(|card| card.def_id).as_deref(), Some(GOING_LONG));
            assert!(locked(&s, P2, Row::Backrow, 2));
        }

        /// §3.2 once it has left, `legalActions` never offers its empty Locked zone, and a summon passes it over
        #[test]
        fn s3_2_once_it_has_left_legal_actions_never_offers_its_empty_locked_zone_and_a_summon_passes_it_over() {
            let mut s = scenario(json!({
                "p1": {
                    "hand": [LANE_EATER, HIT_JOB, TOKEN_MAKER, VANILLA, STOCKPILE],
                    "field": [{ "def": TIMMY, "lane": 1 }],
                    "library": spare()["library"],
                    "mana": 20,
                },
                "p2": spare(),
            }));
            s.play(LANE_EATER, json!({ "zone": 2 }));
            let eater = s.card(LANE_EATER).id.clone();
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": eater }] }));
            assert!(s.unit(P1, 2).is_none());
            let vanilla = s.card(VANILLA).clone();
            let zones: Vec<i32> = legal_actions(s.state(), P1)
                .into_iter()
                .filter_map(|action| match action {
                    ActionBody::Play {
                        instance_id,
                        zone: Some(zone),
                        ..
                    } if instance_id == vanilla.id => Some(zone.lane),
                    _ => None,
                })
                .collect();
            assert_eq!(zones, vec![3, 4, 5]);
            // Me and Mr Token takes lane 3; the Rush Token it summons passes empty, Locked lane 2 for lane 4.
            s.play(TOKEN_MAKER, json!({ "zone": 3 }));
            assert!(s.unit(P1, 2).is_none());
            assert_eq!(s.unit(P1, 4).map(|card| card.def_id).as_deref(), Some("core-t-rush"));
        }

        /// §3.2 later plays into those zones are refused, and its zone stays Locked after it leaves
        #[test]
        fn s3_2_later_plays_into_those_zones_are_refused_and_its_zone_stays_locked_after_it_leaves() {
            let mut s = scenario(json!({
                "p1": { "hand": [LANE_EATER, HIT_JOB, MANA_WELL, VANILLA, STOCKPILE], "library": spare()["library"], "mana": 20 },
                "p2": spare(),
            }));
            s.play(LANE_EATER, json!({ "zone": 2 }));
            s.expect_refused(|s| s.play(MANA_WELL, json!({ "zone": 2 })));
            let eater = s.card(LANE_EATER).id.clone();
            s.play(HIT_JOB, json!({ "targets": [{ "pick": "instance", "instanceId": eater }] }));
            s.expect_in_zone(LANE_EATER, "graveyard");
            assert!(locked(&s, P1, Row::Units, 2));
            s.expect_refused(|s| s.play(VANILLA, json!({ "zone": 2 })));
            // The lanes beside it still take plays.
            s.play(VANILLA, json!({ "zone": 3 }));
            assert_eq!(s.unit(P1, 3).map(|card| card.def_id).as_deref(), Some(VANILLA));
        }

        /// §8.6 a 4/4
        #[test]
        fn s8_6_a_4_4() {
            let mut s = board(false);
            s.play(LANE_EATER, json!({ "zone": 2 }));
            s.expect_stats(LANE_EATER, json!({ "attack": 4, "health": 4 }));
        }
    }

    /// `describe("radiant")`.
    mod radiant {
        use super::*;

        /// §8.6 8/8: it destroys only the enemy cards in its lane, and Locks only the enemy's two zones
        #[test]
        fn s8_6_8_8_it_destroys_only_the_enemy_cards_in_its_lane_and_locks_only_the_enemys_two_zones() {
            let mut s = board(true);
            let vanilla = s.unit(P2, 2).expect("p2's lane-2 unit");
            let trap = s.backrow(P2, 2).expect("p2's lane-2 trap");
            s.play(LANE_EATER, json!({ "zone": 2 }));
            s.expect_stats(LANE_EATER, json!({ "attack": 8, "health": 8 }));
            s.expect_in_zone(&vanilla, "graveyard").expect_in_zone(&trap, "graveyard");
            assert_eq!(s.backrow(P1, 2).map(|card| card.def_id).as_deref(), Some(MANA_WELL));
            assert_eq!(lane_two_locks(&s), vec![false, false, true, true]);
            assert_eq!(
                s.last_events()
                    .iter()
                    .filter(|event| event.event_type() == GameEventType::Locked)
                    .count(),
                2
            );
        }
    }
}
