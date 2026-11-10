//! SPEC §8.12 S+ #49 Metaglass (§4.3, §6.1 Ranged; §6.3 Buff; R81, R783000, R1420, R2261001,
//! R3535000, R3547000): (1) Spell, Rare, Mindustrial, Mineral, a card of the held Stinkyscape+ set.
//!   Base:    "Give a Unit Ranged." Declares no numbers.
//!   Radiant: "Give a Unit Ranged and +{attack} attack." — 3.
//!
//! Engine: a declared target, any Unit on either side (R81, R3547000), then `grant_keyword` Ranged
//! (R783000: the Unit takes no strike back from a defender without it) and, on the Radiant face
//! alone, a `buff` of attack alone. `attack` is declared for the Radiant face only (`tunedOn`,
//! R2261001), so the base face never reads it.

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-049";

/// R81, R3547000: any Unit on either side, picked with the play.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

/// The Radiant face adds the attack to the Ranged both faces give.
fn metaglass(radiant: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let mut effects = vec![grant_keyword(json_as(json!({
                "target": { "of": "chosen" },
                "keyword": { "kind": "Ranged" },
            })))];
            if radiant {
                effects.push(buff(json_as(json!({
                    "target": { "of": "chosen" },
                    "attack": param(&*ctx, "attack"),
                }))));
            }
            effects
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: metaglass(false),
        radiant: metaglass(true),
    }
}

// S+ #49 Metaglass — SPEC §8.12 row S+ 49 (R81, R783000, R2261001, R3547000).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::{scenario, upgrade_number};

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const METAGLASS: &str = "stinkyscapeplus-049";
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const STRIKER: &str = "stand-in (1) 3/10";
    const FILLER: &str = "core-005";

    fn board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": METAGLASS, "radiant": radiant }, FILLER], "field": mine },
            "p2": { "hand": [FILLER], "field": theirs },
        }))
    }

    fn on(card: &CardInstance) -> Value {
        json!({ "targets": pick::instance(card) })
    }

    fn keyword_count(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> usize {
        s.stats(card).keywords.iter().filter(|keyword| keyword.kind() == kind).count()
    }

    fn offered(s: &Scenario) -> Vec<Selection> {
        let metaglass = s.card(METAGLASS).id.clone();
        legal_actions(s.state(), P1)
            .into_iter()
            .flat_map(|action| match action {
                ActionBody::Play { instance_id, targets, .. } if instance_id == metaglass => {
                    targets.unwrap_or_default()
                }
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
        fn ranged_and_no_attack_change() {
            let mut s = board(false, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(METAGLASS, on(&unit));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Ranged), 1);
            s.expect_stats(&unit, json!({ "attack": 4, "health": 4 }));
        }

        #[test]
        fn the_unit_takes_no_strike_back_from_a_defender_without_ranged() {
            let mut s = board(false, json!([VANILLA]), json!([STRIKER]));
            let (mine, theirs) = (s.unit_at(P1, 1), s.unit_at(P2, 1));
            s.play(METAGLASS, on(&mine));
            s.attack(&mine, &theirs);
            assert_eq!(s.card(&mine).damage, 0, "R783000");
            assert_eq!(s.card(&theirs).damage, 4);
        }

        #[test]
        fn an_enemy_unit_takes_ranged_too() {
            let mut s = board(false, json!([]), json!([VANILLA]));
            let theirs = s.unit_at(P2, 1);
            s.play(METAGLASS, on(&theirs));
            assert_eq!(keyword_count(&s, &theirs, KeywordKind::Ranged), 1);
        }

        #[test]
        fn ranged_once_on_a_unit_that_has_it() {
            let mut s = board(false, json!(["stand-in (1) 4/4 Ranged"]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(METAGLASS, on(&unit));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Ranged), 1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn ranged_and_plus_3_attack_and_no_health_change() {
            let mut s = board(true, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(METAGLASS, on(&unit));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Ranged), 1);
            s.expect_stats(&unit, json!({ "attack": 7, "health": 4 }));
            assert_eq!(s.stats(&unit).max_health, 4);
        }

        #[test]
        fn the_unit_takes_no_strike_back_and_hits_for_the_new_attack() {
            let mut s = board(true, json!([VANILLA]), json!([STRIKER]));
            let (mine, theirs) = (s.unit_at(P1, 1), s.unit_at(P2, 1));
            s.play(METAGLASS, on(&mine));
            s.attack(&mine, &theirs);
            assert_eq!(s.card(&mine).damage, 0, "R783000");
            assert_eq!(s.card(&theirs).damage, 7);
        }

        #[test]
        fn attack_reads_through_param() {
            let mut s = board(true, json!([VANILLA]), json!([]));
            assert_eq!(upgrade_number(&mut s, METAGLASS, "attack"), 4);
            let unit = s.unit_at(P1, 1);
            s.play(METAGLASS, on(&unit));
            s.expect_stats(&unit, json!({ "attack": 8, "health": 4 }));
        }
    }
}
