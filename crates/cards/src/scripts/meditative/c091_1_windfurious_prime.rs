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

    mod m91_1_windfurious_prime {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn is_a_token_printed_epic_with_rush_windfury_and_joiners_2() {
                crate::register_all();
                let def = crate::card_def(ID);
                assert_eq!(def.cost, CardCost::Fixed(3));
                assert_eq!(def.rarity, Rarity::Token);
                assert_eq!(def.printed_rarity, Some(PrintedRarity::Epic));
                assert!(def.token);
                assert!(def.tags.contains(&Tag::Prime));
                assert!(def.tags.contains(&Tag::Token));
                assert_eq!(
                    [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
                    [Some(5), Some(10), Some(10), Some(20)]
                );
                assert!(def.base.keywords.contains(&Keyword::Rush));
                assert!(def.base.keywords.contains(&Keyword::Windfury));
                assert!(!def.base.keywords.contains(&Keyword::FirstStrike));
                assert_eq!(
                    def.base.text,
                    "Rush, Windfury\nWhenever this attacks: Summon {joiners|random Unit|random Units} from your hand or deck. They attack its target first."
                );
                assert!(
                    registered_scripts()[ID]
                        .base
                        .static_flags
                        .as_ref()
                        .and_then(|flags| flags.attack_joiners)
                        == Some(true)
                );
            }

            #[test]
            fn joiners_attack_first_in_lane_order_then_the_prime() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "prime-joiners",
                    "p1": { "hand": [UNIT, OTHER, FILLER], "library": [], "field": [PRIME] },
                    "p2": { "hand": [FILLER] },
                }));
                s.attack(PRIME, "hero");
                // Both joiners struck as forced attacks before the Prime's own declaration …
                let kinds: Vec<(String, bool)> = s
                    .events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::AttackDeclared {
                            attacker_id,
                            forced,
                            ..
                        } => Some((attacker_id.clone(), *forced)),
                        _ => None,
                    })
                    .collect();
                assert_eq!(kinds.len(), 3, "two joiners, then the Prime: {kinds:?}");
                assert!(kinds[0].1 && kinds[1].1, "the joiners are forced");
                assert!(!kinds[2].1, "the Prime's own attack is declared");
                let prime = s.card(PRIME).id.clone();
                assert_eq!(kinds[2].0, prime);
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
            fn fewer_zones_fewer_joiners() {
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
                    .filter(|event| {
                        matches!(event, GameEvent::AttackDeclared { forced: true, .. })
                    })
                    .count();
                assert_eq!(forced, 0, "no open zone, no joiners");
                s.expect_health(P2, HERO_HEALTH - 5);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn radiant_adds_first_strike_and_keeps_the_rider() {
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
                    .filter(|event| {
                        matches!(event, GameEvent::AttackDeclared { forced: true, .. })
                    })
                    .count();
                assert_eq!(forced, 2, "the Radiant Prime's joiners attack first too");
            }
        }
    }
}
