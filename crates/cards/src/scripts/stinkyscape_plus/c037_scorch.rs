//! SPEC §8.12 S+ #37 Scorch (Unit 1/1 → 2/2, Mindustrial, Common, cost 1), a card of the Stinkyscape+ set (R1420).
//!   Base:    "Can't Defend, Ranged, Pierce
//!            Can attack {attacks} times per turn.
//!            Can only attack the Unit directly across from this." — attacks 10
//!   Radiant: "Can't Defend, Ranged, Pierce, Cloaked
//!            Can attack {attacks} times per turn.
//!            Can only attack the Unit directly across from this." — attacks 10
//!
//! Engine: the keywords are catalog data. "Can attack N times per turn" is the static flag
//! `attacksPerTurn` and "can only attack the Unit directly across" is `attacksOnlyAcross` (R3550001): N
//! attacks, the larger of N and Windfury's two, read through the declared number `attacks` where a card
//! declares one (R386), so an Upgrade of it lifts the count; and only the enemy Unit in its own lane,
//! never a hero, whether the attack is declared or forced. Ranged takes it no damage back (R783000) and
//! Pierce ignores Armor (R346).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-037";

/// "Can attack 10 times per turn": the catalog's `attacks`, on both faces (R386).
const ATTACKS: i32 = 10;

/// R3550001: both faces' two flags.
fn scorch() -> Script {
    Script {
        static_flags: Some(StaticFlags {
            attacks_per_turn: Some(ATTACKS),
            attacks_only_across: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: scorch(),
        radiant: scorch(),
    }
}

// S+ #37 Scorch — SPEC §8.12 row 37 (R3550001; R346, R386, R783000).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{upgrade_number, weapon_board};
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const TOUGH: &str = "stand-in (1) 0/40";
    /// A Unit that would strike back hard, and whose Armor would take all of a base hit.
    const ARMORED: &str = "stand-in (1) 5/40 Armor 3";

    /// What each face hits for, and the Radiant face's health.
    const ATTACK: i32 = 1;
    const RADIANT_ATTACK: i32 = 2;
    const RADIANT_HEALTH: i32 = 2;

    /// Scorch in p1's lane 2, `p2` for the enemy field.
    fn board(radiant: bool, seed: &str, p2: Value) -> Scenario {
        weapon_board(
            seed,
            json!({ "field": [{ "def": ID, "radiant": radiant, "lane": 2 }] }),
            json!({ "field": p2 }),
        )
    }

    /// The targets of every attack Scorch may declare now.
    fn targets(s: &Scenario) -> Vec<String> {
        legal_actions(s.state(), P1)
            .into_iter()
            .filter_map(|action| match action {
                ActionBody::Attack {
                    attacker_id,
                    target_id,
                } if attacker_id == s.card(ID).id => Some(target_id),
                _ => None,
            })
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r3550001_it_attacks_only_the_unit_across_never_the_hero() {
            let mut s = board(
                false,
                "s037-across",
                json!([{ "def": TOUGH, "lane": 2 }, { "def": TOUGH, "lane": 3 }]),
            );
            let (across, beside) = (s.unit_at(P2, 2).id, s.unit_at(P2, 3).id);
            assert_eq!(targets(&s), vec![across.clone()], "only lane 2's Unit is offered");
            s.expect_refused(|s| s.attack(ID, "hero"));
            s.expect_refused(|s| s.attack(ID, beside.as_str()));
            s.attack(ID, across.as_str());
            assert_eq!(s.card(across.as_str()).damage, ATTACK);
        }

        #[test]
        fn r3550001_with_no_unit_across_it_attacks_nothing() {
            let s = board(false, "s037-empty", json!([{ "def": TOUGH, "lane": 3 }]));
            assert!(targets(&s).is_empty(), "no hero, no neighbour");
        }

        #[test]
        fn r3550001_it_attacks_ten_times_piercing_and_unharmed() {
            let mut s = board(false, "s037-ten", json!([{ "def": ARMORED, "lane": 2 }]));
            let target = s.unit_at(P2, 2).id;
            for done in 1..=ATTACKS {
                s.attack(ID, target.as_str());
                assert_eq!(s.card(target.as_str()).damage, ATTACK * done, "Pierce: Armor 3 takes nothing");
            }
            assert_eq!(s.card(ID).damage, 0, "Ranged: no strike back");
            s.expect_refused(|s| s.attack(ID, target.as_str()));
            assert_eq!(s.card(target.as_str()).damage, ATTACK * ATTACKS);
        }

        #[test]
        fn r386_an_upgrade_of_its_attacks_gives_it_an_eleventh() {
            let mut s = board(false, "s037-upgrade", json!([{ "def": TOUGH, "lane": 2 }]));
            let target = s.unit_at(P2, 2).id;
            let attacks = upgrade_number(&mut s, ID, "attacks");
            assert_eq!(attacks, ATTACKS + 1);
            for _ in 0..attacks {
                s.attack(ID, target.as_str());
            }
            assert_eq!(s.card(target.as_str()).damage, ATTACK * attacks);
            s.expect_refused(|s| s.attack(ID, target.as_str()));
        }

        #[test]
        fn r3550001_it_attacks_again_next_turn() {
            let mut s = board(false, "s037-turn", json!([{ "def": TOUGH, "lane": 2 }]));
            let target = s.unit_at(P2, 2).id;
            for _ in 0..ATTACKS {
                s.attack(ID, target.as_str());
            }
            s.expect_refused(|s| s.attack(ID, target.as_str()));
            crate::next_turn_of(&mut s, P1);
            s.attack(ID, target.as_str());
            assert_eq!(s.card(target.as_str()).damage, ATTACK * (ATTACKS + 1));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3550001_it_deals_two_each_ten_times_and_is_cloaked() {
            let mut s = board(true, "s037-radiant", json!([{ "def": ARMORED, "lane": 2 }]));
            let target = s.unit_at(P2, 2).id;
            let kinds: Vec<KeywordKind> = s.stats(ID).keywords.iter().map(Keyword::kind).collect();
            assert!(kinds.contains(&KeywordKind::Cloaked), "{kinds:?}");
            for _ in 0..ATTACKS {
                s.attack(ID, target.as_str());
            }
            assert_eq!(s.card(target.as_str()).damage, RADIANT_ATTACK * ATTACKS);
            s.expect_refused(|s| s.attack(ID, target.as_str()));
            s.expect_refused(|s| s.attack(ID, "hero"));
        }

        #[test]
        fn its_health_is_doubled() {
            let s = board(true, "s037-health", json!([]));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (RADIANT_ATTACK, RADIANT_HEALTH));
        }
    }
}
