//! M #86 Mayor Medinamogger (SPEC §8.8 row 86): (2) Unit, Legendary, 5/4 → 10/8.
//!
//! Base:    "Aura: All targets are chosen at random."
//! Radiant: "Lucky 1\nAura: All targets are chosen at random."
//! Engine: Hearthstone's Mayor Noggenfogger for both players, while a Mayor acts on the field:
//! `random_targets` (ME-RANDOMTARGETS, R1200). The Radiant face's Lucky 1 rolls its controller's
//! random targets again (R1201). Keywords and the aura are catalog data; there is nothing to run.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-086";

pub fn script() -> CardScripts {
    // Both faces carry the aura; the Radiant face's Lucky 1 is catalog data beside it.
    let base = Script {
        static_flags: Some(StaticFlags {
            random_targets: Some(true),
            ..Default::default()
        }),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// M #86 Mayor Medinamogger — SPEC §8.8 row 86, BUILD M10 row M 86: "While it acts on either side,
// both players' declared targets are drawn by the engine at §10.5 step 1 (MD-E9), `legalActions`
// offering plays and attacks without targets, a target prompt answered at once at random, and an
// attack's target drawn from its legal targets (Taunt respected); Discover, modes, hand picks and
// Tributes stay chosen; the view flags it and the client asks for no target; a play with no legal
// target fizzles unless it needs one (R703); leaving the field restores the choices; radiant 10/8
// and Lucky 1: its controller's random targets roll twice, keeping the side its aim prefers, then
// a survivable or a lethal attack (MD-E10), the opponent's once".
//
// The system is proved in `crates/engine/tests/rules/mb22.rs` (R1200, R1201); these tests prove the
// card's own faces carry it.
#[cfg(test)]
mod tests {
    use super::*;

    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MAYOR: &str = "meditative-086";
    const FILLER: &str = "core-005";
    const BURN: &str = "classic-036"; // (0) Spell: deal 2 damage to any unit or hero.
    const GRUNT: &str = "core-011"; // A plain Unit for the board.

    mod m86_mayor_medinamogger {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn is_a_legendary_2_cost_5_4_unit_with_the_random_targets_aura() {
                crate::register_all();
                let def = crate::card_def(ID);
                assert_eq!(def.cost, CardCost::Fixed(2));
                assert_eq!(
                    [def.base.attack, def.base.health, def.radiant.attack, def.radiant.health],
                    [Some(5), Some(4), Some(10), Some(8)]
                );
                assert_eq!(
                    def.base.text,
                    "Aura: All targets are chosen at random."
                );
                assert!(
                    registered_scripts()[ID]
                        .base
                        .static_flags
                        .as_ref()
                        .and_then(|flags| flags.random_targets)
                        == Some(true)
                );
            }

            #[test]
            fn r1200_while_it_acts_plays_are_offered_without_targets_and_the_view_flags_it() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": { "hand": [BURN, FILLER], "field": [MAYOR] },
                    "p2": { "hand": [FILLER], "field": [GRUNT] },
                }));
                let burn = s.card(BURN).id.clone();
                let plays: Vec<ActionBody> =
                    jackioh_engine::reduce::legal_actions(s.state(), P1)
                        .into_iter()
                        .filter(|action| {
                            matches!(action, ActionBody::Play { instance_id, .. } if instance_id == &burn)
                        })
                        .collect();
                assert_eq!(plays.len(), 1, "one play, not one per target: {plays:?}");
                assert!(
                    matches!(&plays[0], ActionBody::Play { targets: None, .. }),
                    "no targets while the Mayor acts: {:?}",
                    plays[0]
                );
                assert_eq!(s.view(P1).random_targets, Some(true));
                assert_eq!(s.view(P2).random_targets, Some(true));
            }

            #[test]
            fn r1200_a_declared_target_is_drawn_when_the_play_resolves() {
                crate::register_all();
                let mut s = scenario(json!({
                    "seed": "mayor-burn",
                    "p1": { "hand": [BURN, FILLER], "field": [MAYOR] },
                    "p2": { "hand": [FILLER] },
                }));
                let burn = s.card(BURN).id.clone();
                s.play(BURN, json!({}));
                // One target drawn from the legal ones (either hero or the Mayor): the 2 land once.
                let hits: Vec<(String, i32)> = s
                    .last_events()
                    .iter()
                    .filter_map(|event| match event {
                        GameEvent::Damage { source_id: Some(source), target_id, amount, .. }
                            if source == &burn =>
                        {
                            Some((target_id.clone(), *amount))
                        }
                        _ => None,
                    })
                    .collect();
                assert_eq!(hits.len(), 1, "the drawn target took the hit: {hits:?}");
                assert_eq!(hits[0].1, 2, "the drawn target took the 2 damage");
                s.expect_in_zone(BURN, "graveyard");
            }

            #[test]
            fn r1200_choices_return_once_the_mayor_leaves() {
                crate::register_all();
                let mut s = scenario(json!({
                    "p1": { "hand": [BURN, FILLER], "field": [MAYOR] },
                    "p2": { "hand": [FILLER], "field": [GRUNT] },
                }));
                assert_eq!(s.view(P1).random_targets, Some(true));
                // The Mayor leaves the field: off its zone, into its owner's graveyard.
                let mayor = s.card(MAYOR).clone();
                assert!(jackioh_engine::zones::remove_from_field(s.state_mut(), &mayor, Default::default()));
                let mut gone = mayor;
                gone.zone = jackioh_engine::wire::Zone::Graveyard { player: P1 };
                s.state_mut().players[P1].graveyard.push(gone);
                assert_eq!(s.view(P1).random_targets, None);
                let burn = s.card(BURN).id.clone();
                assert_eq!(
                    jackioh_engine::reduce::legal_actions(s.state(), P1)
                        .into_iter()
                        .filter(|action| {
                            matches!(action, ActionBody::Play { instance_id, .. } if instance_id == &burn)
                        })
                        .count(),
                    3,
                    "burn at either hero and the grunt — the Mayor is gone"
                );
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn r1201_radiant_is_10_8_with_lucky_1_beside_the_aura() {
                crate::register_all();
                let def = crate::card_def(ID);
                assert_eq!(
                    def.radiant.text,
                    "Lucky 1\nAura: All targets are chosen at random."
                );
                assert!(def.radiant.keywords.contains(&Keyword::Lucky { n: 1 }));
                assert!(!def.base.keywords.contains(&Keyword::Lucky { n: 1 }));
                assert!(
                    registered_scripts()[ID]
                        .radiant
                        .static_flags
                        .as_ref()
                        .and_then(|flags| flags.random_targets)
                        == Some(true)
                );
            }

            #[test]
            fn the_radiant_mayor_flags_random_targets_too() {
                crate::register_all();
                let s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": MAYOR, "radiant": true }] },
                    "p2": { "hand": [FILLER] },
                }));
                assert_eq!(s.view(P1).random_targets, Some(true));
            }
        }
    }
}
