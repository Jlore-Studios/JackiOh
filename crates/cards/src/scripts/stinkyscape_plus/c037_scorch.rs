//! SPEC §8.12 S+ #37 Scorch (Unit 1/5 → 2/10, Mindustrial, Common, cost 1), a card of the Stinkyscape+ set (R1420).
//!   Base:    "Can't Defend, Ranged, Pierce
//!            Can only attack the Unit directly across from this.
//!            Damage also Burns {burns}." — burns 5
//!   Radiant: the same, burns 10.
//!
//! Engine: the keywords are catalog data. "Can only attack the Unit directly across" is the static flag
//! `attacksOnlyAcross` (R3550001): only the enemy Unit in its own lane, never a hero, whether the attack is
//! declared or forced. "Damage also Burns N" is the rider `burnsOnDamage` (R3540002), N the declared
//! `burns` per face (R386). Ranged takes it no damage back (R783000) and Pierce ignores Armor (R346).
//! Issue #3951 made it 1/5 (Radiant 2/10), took away its ten attacks a turn (`attacksPerTurn`) and the
//! Radiant face's Cloaked, and gave it the Burn; `script_v0_6_4b` keeps the ten attacks for the replays
//! of v0.6.4b and before (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-037";

/// The printed Burns of each face, the catalog's `burns` (R386).
const BASE_BURNS: i32 = 5;
const RADIANT_BURNS: i32 = 10;
/// Through v0.6.4b: "Can attack 10 times per turn", the catalog's `attacks` then, on both faces (R386).
const ATTACKS_V0_6_4B: i32 = 10;

/// R3550001, R3540002: a face's two flags.
fn scorch(burns: i32) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            attacks_only_across: Some(true),
            burns_on_damage: Some(burns),
            ..StaticFlags::default()
        }),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: scorch(BASE_BURNS),
        radiant: scorch(RADIANT_BURNS),
    }
}

/// R767000: both faces through v0.6.4b, ten attacks a turn and no Burn (issue #3951). `KEPT_SCRIPTS`
/// folds those replays under it.
pub fn script_v0_6_4b() -> CardScripts {
    let face = Script {
        static_flags: Some(StaticFlags {
            attacks_per_turn: Some(ATTACKS_V0_6_4B),
            attacks_only_across: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    CardScripts {
        base: face.clone(),
        radiant: face,
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

    /// What each face hits for, and its health.
    const ATTACK: i32 = 1;
    const HEALTH: i32 = 5;
    const RADIANT_ATTACK: i32 = 2;
    const RADIANT_HEALTH: i32 = 10;

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

    fn burn_of(s: &Scenario, id: &str) -> i32 {
        query::burn_on(s.card(id))
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

        /// Issue #3951: one attack a turn, piercing, unharmed, and its hit Burns 5.
        #[test]
        fn r3540002_it_attacks_once_piercing_unharmed_and_burns_5() {
            let mut s = board(false, "s037-once", json!([{ "def": ARMORED, "lane": 2 }]));
            let target = s.unit_at(P2, 2).id;
            s.attack(ID, target.as_str());
            assert_eq!(s.card(target.as_str()).damage, ATTACK, "Pierce: Armor 3 takes nothing");
            assert_eq!(burn_of(&s, target.as_str()), BASE_BURNS);
            assert_eq!(s.card(ID).damage, 0, "Ranged: no strike back");
            s.expect_refused(|s| s.attack(ID, target.as_str()));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (ATTACK, HEALTH));
        }

        #[test]
        fn r386_an_upgrade_of_its_burns_burns_one_more() {
            let mut s = board(false, "s037-upgrade", json!([{ "def": TOUGH, "lane": 2 }]));
            let target = s.unit_at(P2, 2).id;
            let burns = upgrade_number(&mut s, ID, "burns");
            assert_eq!(burns, BASE_BURNS + 1);
            s.attack(ID, target.as_str());
            assert_eq!(burn_of(&s, target.as_str()), burns);
        }

        #[test]
        fn r3550001_it_attacks_again_next_turn() {
            let mut s = board(false, "s037-turn", json!([{ "def": TOUGH, "lane": 2 }]));
            let target = s.unit_at(P2, 2).id;
            s.attack(ID, target.as_str());
            s.expect_refused(|s| s.attack(ID, target.as_str()));
            crate::next_turn_of(&mut s, P1);
            s.attack(ID, target.as_str());
            // Two hits, and the Burn 5 the first left ticked at the start of p2's turn (R3540000).
            assert_eq!(s.card(target.as_str()).damage, 2 * ATTACK + BASE_BURNS);
        }

        /// R767000: through v0.6.4b it attacked ten times a turn and burned nothing (issue #3951).
        #[test]
        fn r767000_the_script_kept_for_v0_6_4b_attacks_ten_times_and_burns_nothing() {
            crate::under_kept(ID, script_v0_6_4b(), || {
                let mut s = board(false, "s037-kept", json!([{ "def": TOUGH, "lane": 2 }]));
                let target = s.unit_at(P2, 2).id;
                for _ in 0..ATTACKS_V0_6_4B {
                    s.attack(ID, target.as_str());
                }
                s.expect_refused(|s| s.attack(ID, target.as_str()));
                assert_eq!(burn_of(&s, target.as_str()), 0);
            });
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3540002_it_deals_two_burns_10_and_is_not_cloaked() {
            let mut s = board(true, "s037-radiant", json!([{ "def": ARMORED, "lane": 2 }]));
            let target = s.unit_at(P2, 2).id;
            let kinds: Vec<KeywordKind> = s.stats(ID).keywords.iter().map(Keyword::kind).collect();
            assert!(!kinds.contains(&KeywordKind::Cloaked), "{kinds:?}");
            s.attack(ID, target.as_str());
            assert_eq!(s.card(target.as_str()).damage, RADIANT_ATTACK);
            assert_eq!(burn_of(&s, target.as_str()), RADIANT_BURNS);
            s.expect_refused(|s| s.attack(ID, target.as_str()));
            s.expect_refused(|s| s.attack(ID, "hero"));
        }

        #[test]
        fn its_stats_are_doubled() {
            let s = board(true, "s037-health", json!([]));
            let view = s.stats(ID);
            assert_eq!((view.attack, view.health), (RADIANT_ATTACK, RADIANT_HEALTH));
        }
    }
}
