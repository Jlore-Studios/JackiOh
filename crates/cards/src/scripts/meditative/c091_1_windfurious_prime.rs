//! M #91.1 Windfurious Prime (SPEC §8.8 row 91.1, §7 row 1b): (3) Unit Token (printed Epic),
//! Prime, 5/10 → 10/20, Rush and Windfury on both faces, First Strike added on the Radiant face.
//!
//! Base:    "Rush, Windfury\nWhenever this attacks: Summon {joiners|random Unit|random Units} from
//!            your hand or deck. They attack its target first."
//! Radiant: "Rush, Windfury, First Strike\nWhenever this attacks: Summon {joiners|random Unit|random
//!            Units} from your hand or deck. They attack its target first."
//! Engine: `attack_joiners` (ME-ATTACKSUMMON, R1203); the count is the catalog `joiners` param (2
//! and 2). Keywords are catalog data; there is nothing to run.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-091-1";

pub fn script() -> CardScripts {
    // Both faces summon their joiners; the Radiant face's First Strike is catalog data beside it.
    let base = Script {
        static_flags: Some(StaticFlags {
            attack_joiners: Some(true),
            ..Default::default()
        }),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// M #91.1 Windfurious Prime — SPEC §8.8 row 91.1, BUILD M10 row M 91-1: "its controller's
// hand-or-deck pool is Uniform; the joiners attack first (in lane order); a declared attack whose
// target has left the field before its combat is cancelled (exertion spent), attackCancelled is
// emitted, and the others' window, too; a subset attacks if there are fewer zones, none if there
// are no Units, and only those cast; they never return".
//
// The rider is proved in `crates/engine/tests/rules/mb22.rs` (R1203); these tests prove the card's
// own faces carry it.
#[cfg(test)]
mod tests {
    use super::*;

    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const PRIME: &str = "meditative-091-1";
    const UNIT: &str = "core-011"; // A plain 3/3 Unit for the pool.
    const OTHER: &str = "classic-086"; // A plain 14/14 Unit.
    const FILLER: &str = "core-005";
    const SCARAB: &str = "core-007"; // (1) 1/1, Cry: Discover a (2) Cost card.
    const MOTHS: &str = "core-009"; // (2) 1/14, Start of turn: Every enemy Unit attacks this.
    const VANILLA: &str = "core-008"; // (1) 4/4.

    mod m91_1_windfurious_prime {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r1203_joiners_attack_first_in_lane_order_then_the_prime() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "prime-joiners",
                    "p1": { "hand": [UNIT, OTHER, FILLER], "library": [], "field": [PRIME] },
                    "p2": { "hand": [FILLER] },
                }));
                s.attack(PRIME, "hero");
                // The Prime declares; before its combat both joiners strike as forced attacks (R1203) …
                let kinds: Vec<(String, bool)> = s
                    .events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::AttackDeclared {
                            attacker_id, forced, ..
                        } => Some((attacker_id.clone(), *forced)),
                        _ => None,
                    })
                    .collect();
                assert_eq!(kinds.len(), 3, "the Prime, then its two joiners: {kinds:?}");
                assert!(!kinds[0].1, "the Prime's own attack is declared");
                let prime = s.card(PRIME).id.clone();
                assert_eq!(kinds[0].0, prime);
                assert!(kinds[1].1 && kinds[2].1, "the joiners are forced");
                // … and all three hit: the hero took the joiners' 3 + 14 and the Prime's 5.
                s.expect_health(P2, HERO_HEALTH - 3 - 14 - 5);
                // The joiners stay; nothing returns.
                assert_eq!(
                    s.state().players[P1].units.iter().flatten().count(),
                    3,
                    "the Prime and its two joiners hold the field"
                );
            }

            #[test]
            fn r1203_fewer_zones_fewer_joiners() {
                crate::register_all();
                const AURA: &str = "classic-049";
                const TAUNT: &str = "core-055";
                let mut s = scenario(json!({
                    "seed": "prime-one-zone",
                    "p1": { "hand": [UNIT, OTHER, FILLER], "library": [UNIT], "field": [PRIME, UNIT, OTHER, AURA, TAUNT] },
                    "p2": { "hand": [FILLER] },
                }));
                // Four units beside the Prime leave no open zone: the Prime fights alone.
                s.attack(PRIME, "hero");
                let forced = s
                    .events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::AttackDeclared { forced: true, .. }))
                    .count();
                assert_eq!(forced, 0, "no open zone, no joiners");
                s.expect_health(P2, HERO_HEALTH - 5);
            }
            #[test]
            fn rush_and_windfury_it_attacks_twice_the_turn_it_is_played() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "prime-windfury",
                    "p1": { "hand": [PRIME, FILLER], "library": [] },
                    "p2": { "hand": [FILLER], "field": [VANILLA, VANILLA] },
                }));
                s.play(PRIME, json!({ "zone": 1 }));
                let first = s.unit(P2, 1).unwrap().id;
                let second = s.unit(P2, 2).unwrap().id;
                // Rush: it attacks a Unit at once; Windfury: a second time.
                s.attack(PRIME, &first);
                s.expect_in_zone(&first, "graveyard");
                s.attack(PRIME, &second);
                s.expect_in_zone(&second, "graveyard");
                s.expect_refused(|s| s.attack(PRIME, "hero"));
            }

            #[test]
            fn r1203_two_different_units_from_hand_and_deck_together_join_with_no_cry() {
                // Jewelosco Scarab's Cry would Discover; summoned as a joiner it runs nothing.
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "prime-no-cry",
                    "p1": { "hand": [SCARAB, FILLER], "library": [OTHER], "field": [PRIME] },
                    "p2": { "hand": [FILLER] },
                }));
                let scarab = s.card(SCARAB).id.clone();
                let other = s.card(OTHER).id.clone();
                s.attack(PRIME, "hero");
                assert!(s.state().pending.is_none(), "no Cry prompt opened");
                // One from the hand and one from the deck: both pools together.
                s.expect_in_zone(&scarab, "field");
                s.expect_in_zone(&other, "field");
                assert_ne!(scarab, other);
                s.expect_health(P2, HERO_HEALTH - 1 - 14 - 5);
            }

            #[test]
            fn r1203_a_forced_attack_brings_its_joiners_too() {
                // Moths to the Flame: "Start of turn: Every enemy Unit attacks this."
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "prime-forced",
                    "p1": { "hand": [UNIT, FILLER], "library": [OTHER], "field": [PRIME] },
                    "p2": { "hand": [FILLER], "field": [MOTHS] },
                }));
                s.end_turn();
                let forced: Vec<String> = s
                    .events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::AttackDeclared {
                            attacker_id,
                            forced: true,
                            ..
                        } => Some(attacker_id.clone()),
                        _ => None,
                    })
                    .collect();
                let prime = s.card(PRIME).id.clone();
                assert!(forced.contains(&prime), "the Prime is forced: {forced:?}");
                assert_eq!(forced.len(), 3, "the Prime and its two joiners: {forced:?}");
            }

            #[test]
            fn r1203_with_the_target_gone_the_remaining_attacks_are_cancelled_exertion_spent() {
                // The first joiner kills the 1/1 Scarab: the second joiner and the Prime find no
                // target and are cancelled, the Prime's exertion spent.
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "prime-gone",
                    "p1": { "hand": [UNIT, FILLER], "library": [OTHER], "field": [PRIME] },
                    "p2": { "hand": [FILLER], "field": [SCARAB] },
                }));
                let prime = s.card(PRIME).id.clone();
                let scarab = s.unit(P2, 1).unwrap().id;
                s.attack(&prime, &scarab);
                s.expect_in_zone(&scarab, "graveyard");
                let cancelled: Vec<String> = s
                    .events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::AttackCancelled { attacker_id, .. } => Some(attacker_id.clone()),
                        _ => None,
                    })
                    .collect();
                assert_eq!(cancelled, vec![prime.clone()], "the Prime's attack is cancelled");
                // Only the first joiner fought: the target was gone before the second's turn.
                let fights = s
                    .events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::Damage { .. }))
                    .count();
                assert!(fights <= 2, "one combat, its hit and its strike back");
                assert_eq!(s.card(prime.as_str()).damage, 0, "the Prime fought no combat");
                assert!(s.card(prime.as_str()).exertion.attacked, "its exertion is spent");
                s.expect_health(P2, HERO_HEALTH);
            }

            #[test]
            fn r1203_joiners_stay() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "prime-stay",
                    "p1": { "hand": [UNIT, FILLER], "library": [OTHER, FILLER, FILLER], "field": [PRIME] },
                    "p2": { "hand": [FILLER], "library": [FILLER, FILLER] },
                }));
                let unit = s.card(UNIT).id.clone();
                let other = s.card(OTHER).id.clone();
                s.attack(PRIME, "hero");
                s.end_turn();
                s.end_turn();
                s.expect_in_zone(&unit, "field");
                s.expect_in_zone(&other, "field");
            }

            #[test]
            fn r1203_fewer_units_fewer_joiners() {
                // One Unit in hand and deck together: one joiner.
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "prime-one-unit",
                    "p1": { "hand": [UNIT, FILLER], "library": [], "field": [PRIME] },
                    "p2": { "hand": [FILLER] },
                }));
                s.attack(PRIME, "hero");
                let forced = s
                    .events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::AttackDeclared { forced: true, .. }))
                    .count();
                assert_eq!(forced, 1);
                s.expect_health(P2, HERO_HEALTH - 3 - 5);
            }

            #[test]
            fn r1203_joiners_reads_through_param() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "prime-tuned",
                    "p1": { "hand": [UNIT, OTHER, FILLER], "library": [UNIT], "field": [PRIME] },
                    "p2": { "hand": [FILLER] },
                }));
                assert_eq!(crate::upgrade_number(&mut s, PRIME, "joiners"), 3);
                s.attack(PRIME, "hero");
                let forced = s
                    .events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::AttackDeclared { forced: true, .. }))
                    .count();
                assert_eq!(forced, 3, "a tuned joiners brings three");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r1203_radiant_adds_first_strike_and_keeps_the_rider() {
                crate::register_all();
                let def = crate::card_def(ID);
                assert_eq!(
                    def.radiant.text,
                    "Rush, Windfury, First Strike\nWhenever this attacks: Summon {joiners|random Unit|random Units} from your hand or deck. They attack its target first."
                );
                assert!(def.radiant.keywords.contains(&Keyword::FirstStrike));
                assert!(
                    registered_scripts()[ID]
                        .radiant
                        .static_flags
                        .as_ref()
                        .and_then(|flags| flags.attack_joiners)
                        == Some(true)
                );
                let mut s = scenario(json!({
                    "seed": "prime-radiant",
                    "p1": { "hand": [UNIT, OTHER, FILLER], "library": [UNIT], "field": [{ "def": PRIME, "radiant": true }] },
                    "p2": { "hand": [FILLER] },
                }));
                s.attack(PRIME, "hero");
                let forced = s
                    .events()
                    .iter()
                    .filter(|event| matches!(event, GameEvent::AttackDeclared { forced: true, .. }))
                    .count();
                assert_eq!(forced, 2, "the Radiant Prime's joiners attack first too");
            }

            #[test]
            fn radiant_is_10_20_with_first_strike() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "field": [{ "def": PRIME, "radiant": true }] },
                }));
                s.expect_stats(PRIME, json!({ "attack": 10, "health": 20 }));
                assert!(
                    s.stats(PRIME)
                        .keywords
                        .iter()
                        .any(|keyword| keyword.kind() == KeywordKind::FirstStrike)
                );
            }
        }
    }
}
