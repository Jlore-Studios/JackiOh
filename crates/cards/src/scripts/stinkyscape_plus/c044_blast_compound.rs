//! SPEC §8.12 S+ #44 Blast Compound (§3.2 Lock, §6.1 Cleave, §6.3 Buff, Lock; R81, R1420, R3535000,
//! R3547000, R3547001): (1) Spell, Rare, Mindustrial, Mineral, a card of the held Stinkyscape+ set.
//!   Base:    "Give a Unit +{attack} attack and Cleave, lock the tile it is on." — 2.
//!   Radiant: "Give a Unit +{attack} attack and Cleave, lock the tile it is on." — 6.
//!
//! Engine: a declared target, any Unit on either side (R81, R3547000), then in the text's order a
//! `buff` of attack alone, `grant_keyword` Cleave and `lock` on the zone the chosen Unit stands in
//! (§6.3 Lock; R3547001), as Stinkyscape's Meteor locks its target's. The two faces differ by the
//! number alone, `attack`, 2 on the base face and 6 on the Radiant one. The Lock refuses summons
//! into the zone until something unlocks it; its occupant is unaffected and keeps its Cleave.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-044";

/// R81, R3547000: any Unit on either side, picked with the play.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

/// One script for both faces: `attack` is declared per face.
fn blast_compound() -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(|ctx| {
            vec![
                buff(json_as(json!({ "target": { "of": "chosen" }, "attack": param(&*ctx, "attack") }))),
                grant_keyword(json_as(json!({ "target": { "of": "chosen" }, "keyword": { "kind": "Cleave" } }))),
                lock(json_as(json!({ "zone": { "of": "chosen" } }))),
            ]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    let base = blast_compound();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// S+ #44 Blast Compound — SPEC §8.12 row S+ 44 (R81, R3547000, R3547001).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{scenario, upgrade_number};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const BLAST: &str = "stinkyscapeplus-044";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const FLEXIBLE: &str = "stand-in (1) 4/4 Flexible";
    const FILLER: &str = "core-005";

    fn board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": BLAST, "radiant": radiant }, FILLER], "field": mine },
            "p2": { "hand": [FILLER], "field": theirs },
        }))
    }

    fn on(card: &CardInstance) -> Value {
        json!({ "targets": pick::instance(card) })
    }

    fn keyword_count(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> usize {
        s.stats(card).keywords.iter().filter(|keyword| keyword.kind() == kind).count()
    }

    fn locked(s: &Scenario, player: PlayerId, row: Row, lane: i32) -> bool {
        is_locked(s.state(), ZoneSlot { player, row, lane })
    }

    fn locked_zones(s: &Scenario) -> usize {
        [P1, P2]
            .into_iter()
            .flat_map(|player| (1..=UNIT_ZONES).map(move |lane| (player, lane)))
            .filter(|(player, lane)| locked(s, *player, Row::Units, *lane))
            .count()
    }

    fn offered(s: &Scenario) -> Vec<Selection> {
        let blast = s.card(BLAST).id.clone();
        legal_actions(s.state(), P1)
            .into_iter()
            .flat_map(|action| match action {
                ActionBody::Play { instance_id, targets, .. } if instance_id == blast => targets.unwrap_or_default(),
                _ => Vec::new(),
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r3547000_it_offers_any_unit_on_either_side_and_no_hero() {
            let s = board(false, json!([VANILLA]), json!([VANILLA]));
            let offered = offered(&s);
            for unit in [s.unit_at(P1, 1), s.unit_at(P2, 1)] {
                assert!(offered.contains(&Selection::Instance { instance_id: unit.id }));
            }
            assert!(!offered.iter().any(|pick| matches!(pick, Selection::Hero { .. })));
        }

        #[test]
        fn attack_up_2_cleave_and_the_zone_locked() {
            let mut s = board(false, json!([]), json!([{ "def": VANILLA, "lane": 2 }]));
            let target = s.unit_at(P2, 2);
            s.play(BLAST, on(&target));
            s.expect_stats(&target, json!({ "attack": 6, "health": 4 }));
            assert_eq!(s.stats(&target).max_health, 4);
            assert_eq!(keyword_count(&s, &target, KeywordKind::Cleave), 1);
            s.expect_in_zone(&target, "field");
            assert!(locked(&s, P2, Row::Units, 2));
        }

        #[test]
        fn r3547001_the_lock_is_the_zone_the_unit_stands_in_and_no_other() {
            let mut s = board(
                false,
                json!([{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 3 }]),
                json!([{ "def": VANILLA, "lane": 1 }, { "def": VANILLA, "lane": 3 }]),
            );
            let target = s.unit_at(P1, 3);
            s.play(BLAST, on(&target));
            assert!(locked(&s, P1, Row::Units, 3), "its own zone");
            assert_eq!(locked_zones(&s), 1, "the zone beside it, the same lane across and the other side's stay open");
            assert!(s.last_events().iter().any(|event| {
                matches!(event, GameEvent::Locked { player: P1, row: Row::Units, lane: 3 })
            }));
            assert_eq!(s.unit_at(P1, 3).id, target.id, "the occupant is unaffected and stays");
        }

        #[test]
        fn r3547001_a_unit_in_the_backrow_locks_its_backrow_zone() {
            let mut s = board(
                false,
                json!([{ "def": FLEXIBLE, "lane": 2, "row": "backrow" }]),
                json!([]),
            );
            let target = s.backrow(P1, 2).expect("the Flexible Unit stands in the backrow");
            s.play(BLAST, on(&target));
            assert!(locked(&s, P1, Row::Backrow, 2));
            assert_eq!(locked_zones(&s), 0, "no unit zone is locked");
            assert_eq!(keyword_count(&s, &target, KeywordKind::Cleave), 1);
        }

        #[test]
        fn an_enemy_unit_takes_the_attack_and_its_zone_locks_on_their_side() {
            let mut s = board(false, json!([VANILLA]), json!([VANILLA]));
            let (mine, theirs) = (s.unit_at(P1, 1), s.unit_at(P2, 1));
            s.play(BLAST, on(&theirs));
            s.expect_stats(&theirs, json!({ "attack": 6, "health": 4 }));
            s.expect_stats(&mine, json!({ "attack": 4, "health": 4 }));
            assert!(locked(&s, P2, Row::Units, 1));
            assert!(!locked(&s, P1, Row::Units, 1));
        }

        #[test]
        fn cleave_once_on_a_unit_that_has_it() {
            let mut s = board(false, json!(["stand-in (1) 4/4 Cleave"]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(BLAST, on(&unit));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Cleave), 1);
        }

        #[test]
        fn attack_reads_through_param() {
            let mut s = board(false, json!([VANILLA]), json!([]));
            assert_eq!(upgrade_number(&mut s, BLAST, "attack"), 3);
            let unit = s.unit_at(P1, 1);
            s.play(BLAST, on(&unit));
            s.expect_stats(&unit, json!({ "attack": 7, "health": 4 }));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn attack_up_6_cleave_and_the_zone_locked() {
            let mut s = board(true, json!([{ "def": VANILLA, "lane": 4 }]), json!([]));
            let target = s.unit_at(P1, 4);
            s.play(BLAST, on(&target));
            s.expect_stats(&target, json!({ "attack": 10, "health": 4 }));
            assert_eq!(s.stats(&target).max_health, 4);
            assert_eq!(keyword_count(&s, &target, KeywordKind::Cleave), 1);
            assert!(locked(&s, P1, Row::Units, 4));
            assert_eq!(locked_zones(&s), 1);
        }

        #[test]
        fn attack_reads_through_param_on_its_own_face() {
            let mut s = board(true, json!([VANILLA]), json!([]));
            assert_eq!(upgrade_number(&mut s, BLAST, "attack"), 7);
            let unit = s.unit_at(P1, 1);
            s.play(BLAST, on(&unit));
            s.expect_stats(&unit, json!({ "attack": 11, "health": 4 }));
        }
    }
}
