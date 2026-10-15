//! M #91 Windfast (SPEC §8.8 row 91): (1) Unit, Epic, Catalyst, 1/1 → 2/2, Windfury on both faces.
//!
//! Base:    "Tribute 1, Windfury\nWhen this would attack, summon a Unit of your choice from your hand
//!            to make that attack instead. Bounce it afterwards if it survives.\nDeath: Shuffle a
//!            Windfurious Prime into your deck."
//! Radiant: "Tribute 1, Windfury\nWhen this would attack, summon a Unit of your choice from your hand
//!            to make that attack instead.\nDeath: Shuffle a Radiant Windfurious Prime into your deck."
//! Engine: `tribute` 1, `attack_from_hand` on both faces, `bounce_attacker` on the base face only
//! (ME-ATTACKSUMMON, R1202); a `death` hook shuffling in `meditative-091-1` — Radiant on the Radiant
//! face. Tribute and Windfury are catalog data; the aura it brings ("a declared attack keeps its
//! window; a forced one stays forced") is the engine's.

use jackioh_engine::effects::shuffle_into;
use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-091";
pub const PRIME: &str = "meditative-091-1";

fn windfast(radiant_prime: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            tribute: Some(1),
            attack_from_hand: Some(true),
            // The base face bounces the substitute afterwards if it survives; the Radiant face keeps it.
            bounce_attacker: if radiant_prime { None } else { Some(true) },
            ..Default::default()
        }),
        death: Some(hook(move |_| {
            vec![shuffle_into(json_as(json!({
                "defId": PRIME,
                "count": 1,
                "radiant": radiant_prime,
            })))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: windfast(false),
        radiant: windfast(true),
    }
}

// M #91 Windfast — SPEC §8.8 row 91, BUILD M10 row M 91: "Tribute 1 and melee only, Windfury on both
// faces; whenever it would attack, declared or forced, its controller summons a Unit from their
// hand that makes that attack on the same target as a forced attack (Covers R1202), with its own
// trap window (a declared attack keeps its window; a forced one stays forced); the base face
// bounces the summoned Unit if it survives the attack; with no Unit in hand or no open zone
// Windfast attacks itself; each of Windfury's two attacks summons its own; Death shuffles a
// Windfurious Prime (Radiant on the Radiant face) into its owner's deck".
//
// The replacement is proved in `crates/engine/tests/rules/mb22.rs` (R1202); these tests prove the
// card's own faces carry it.
#[cfg(test)]
mod tests {
    use super::*;

    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const WINDFAST: &str = "meditative-091";
    const UNIT: &str = "core-011"; // A plain 3/3 Unit for the hand.
    const OTHER: &str = "classic-086"; // A plain 14/14 Unit, so two different Units can be held.
    const FILLER: &str = "core-005";
    const GRUNT: &str = "core-011";

    mod m91_windfast {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn is_an_epic_catalyst_1_cost_1_1_with_tribute_1_and_windfury() {
                crate::register_all();
                let def = crate::card_def(ID);
                assert_eq!(def.cost, CardCost::Fixed(1));
                assert_eq!(def.rarity, Rarity::Epic);
                assert!(def.tags.contains(&Tag::Catalyst));
                assert_eq!(
                    [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
                    [Some(1), Some(1), Some(2), Some(2)]
                );
                assert!(def.base.keywords.contains(&Keyword::Windfury));
                assert!(def.radiant.keywords.contains(&Keyword::Windfury));
                assert_eq!(
                    def.base.text,
                    "Tribute 1, Windfury\nWhen this would attack, summon a Unit of your choice from your hand to make that attack instead. Bounce it afterwards if it survives.\nDeath: Shuffle a Windfurious Prime into your deck."
                );
                assert_eq!(def.refs, Some(vec![PRIME.to_string()]));
                let all = registered_scripts();
                let flags = all[ID].base.static_flags.as_ref().expect("flags");
                assert_eq!(flags.tribute, Some(1));
                assert_eq!(flags.attack_from_hand, Some(true));
                assert_eq!(flags.bounce_attacker, Some(true));
            }

            #[test]
            fn r1202_one_unit_in_hand_makes_the_attack_and_is_bounced() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [UNIT, FILLER], "field": [WINDFAST] },
                    "p2": { "hand": [FILLER] },
                }));
                s.attack(WINDFAST, "hero");
                // The 3/3 fought, not the 1/1: the hero took 3.
                s.expect_health(P2, HERO_HEALTH - 3);
                // …and went home afterwards, if it survived (it did: heroes strike nothing back).
                s.expect_in_zone(UNIT, "hand");
                // Windfast spent its exertion without emitting its own declaration.
                let windfast = s.card(WINDFAST).id.clone();
                assert!(
                    find_instance(s.state(), &windfast)
                        .expect("Windfast")
                        .exertion
                        .attacked
                );
                let declarations: Vec<(String, Option<String>)> = s
                    .events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::AttackDeclared {
                            attacker_id,
                            instead_of,
                            ..
                        } => Some((attacker_id.clone(), instead_of.clone())),
                        _ => None,
                    })
                    .collect();
                assert_eq!(declarations.len(), 1);
                assert_eq!(declarations[0].1.as_deref(), Some(windfast.as_str()));
                assert_ne!(declarations[0].0, windfast);
            }

            #[test]
            fn r1202_windfury_summons_its_own_for_each_attack() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [UNIT, FILLER], "field": [WINDFAST] },
                    "p2": { "hand": [FILLER] },
                }));
                s.attack(WINDFAST, "hero");
                s.attack(WINDFAST, "hero");
                s.expect_health(P2, HERO_HEALTH - 6);
                s.expect_in_zone(UNIT, "hand");
            }

            #[test]
            fn r1202_several_units_ask_which_one() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [UNIT, OTHER, FILLER], "field": [WINDFAST] },
                    "p2": { "hand": [FILLER] },
                }));
                s.attack(WINDFAST, "hero");
                let pending = s.state().pending.clone().expect("the hand pick");
                assert_eq!(pending.kind, PromptKind::Hand);
                s.answer(json!([UNIT]));
                s.expect_health(P2, HERO_HEALTH - 3);
                s.expect_in_zone(UNIT, "hand");
                s.expect_in_zone(OTHER, "hand");
            }

            #[test]
            fn r1202_with_no_unit_in_hand_it_attacks_itself() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [WINDFAST] },
                    "p2": { "hand": [FILLER] },
                }));
                s.attack(WINDFAST, "hero");
                s.expect_health(P2, HERO_HEALTH - 1);
            }

            #[test]
            fn death_shuffles_a_windfurious_prime_into_the_deck() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [WINDFAST] },
                    "p2": { "hand": [FILLER], "field": [GRUNT] },
                }));
                s.end_turn();
                s.attack(GRUNT, WINDFAST);
                s.expect_in_zone(WINDFAST, "graveyard");
                let prime = s.state().players[P1]
                    .library
                    .iter()
                    .find(|card| card.def_id == PRIME)
                    .expect("the Prime in the deck");
                assert!(!prime.radiant, "the base face shuffles the base Prime");
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r1202_radiant_is_2_2_and_keeps_the_substitute() {
                crate::register_all();
                let def = crate::card_def(ID);
                assert_eq!(
                    def.radiant.text,
                    "Tribute 1, Windfury\nWhen this would attack, summon a Unit of your choice from your hand to make that attack instead.\nDeath: Shuffle a Radiant Windfurious Prime into your deck."
                );
                let all = registered_scripts();
                let flags = all[ID].radiant.static_flags.as_ref().expect("flags");
                assert_eq!(flags.attack_from_hand, Some(true));
                assert_eq!(flags.bounce_attacker, None);
                let mut s = scenario(json!({
                    "p1": { "hand": [UNIT, FILLER], "field": [{ "def": WINDFAST, "radiant": true }] },
                    "p2": { "hand": [FILLER] },
                }));
                s.attack(WINDFAST, "hero");
                s.expect_health(P2, HERO_HEALTH - 3);
                s.expect_in_zone(UNIT, "field");
            }

            #[test]
            fn radiant_death_shuffles_a_radiant_prime() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": WINDFAST, "radiant": true }] },
                    "p2": { "hand": [FILLER], "field": [GRUNT] },
                }));
                s.end_turn();
                s.attack(GRUNT, WINDFAST);
                let prime = s.state().players[P1]
                    .library
                    .iter()
                    .find(|card| card.def_id == PRIME)
                    .expect("the Prime in the deck");
                assert!(prime.radiant, "the Radiant face shuffles the Radiant Prime");
            }
        }
    }
}
