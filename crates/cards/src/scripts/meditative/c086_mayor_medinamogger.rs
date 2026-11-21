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

// M #86 Mayor Medinamogger — SPEC §8.8 row 86, BUILD M10 row M 86 (§10.5, R703).
//
// The system is proved in `crates/engine/tests/rules/mb22.rs` (R1200, R1201); these tests prove the
// card's own faces carry it.
#[cfg(test)]
mod tests {
    use super::*;

    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const MAYOR: &str = "meditative-086";
    const FILLER: &str = "core-005";
    const BURN: &str = "classic-036"; // Burn: deal 2 damage to any unit or hero
    const GRUNT: &str = "core-011"; // Tempo Timmy: A plain Unit for the board.

    mod m86_mayor_medinamogger {
        use super::*;

        mod base {
            use super::*;

            #[test]
            fn r1200_while_it_acts_plays_are_offered_without_targets_and_the_view_flags_it() {
                let s = scenario(json!({
                    "p1": { "hand": [BURN, FILLER], "field": [MAYOR] },
                    "p2": { "hand": [FILLER], "field": [GRUNT] },
                }));
                let plays: Vec<ActionBody> = s.plays_of(BURN);
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
            fn r1200_r4353001_the_drawn_target_may_be_an_immutable_unit_a_spell_may_target() {
                crate::register_all();
                let mut menace_drawn = 0;
                for n in 0..16 {
                    let mut s = scenario(json!({
                        "seed": format!("mayor-immutable-{n}"),
                        "p1": { "hand": ["core-016", FILLER], "field": [MAYOR], "mana": 10 },
                        "p2": { "hand": [FILLER], "field": [{ "def": "core-019", "radiant": true }] },
                    }));
                    let menace = s.card("core-019").id.clone();
                    let mayor = s.card(MAYOR).id.clone();
                    // Hit Job's target is drawn from the legal ones (R1200), and a Spell may target the
                    // Radiant Midrange Menace in play (R4353001): it is drawn from the two, and destroyed.
                    s.play("core-016", json!({}));
                    let menace_gone = s.card(&menace).zone.z() == ZoneName::Graveyard;
                    let mayor_gone = s.card(&mayor).zone.z() == ZoneName::Graveyard;
                    assert!(menace_gone != mayor_gone, "seed {n}: one of the two was drawn and destroyed");
                    menace_drawn += usize::from(menace_gone);
                }
                assert!(menace_drawn > 0, "the Immutable Menace is among the draws");
            }

            #[test]
            fn r1200_choices_return_once_the_mayor_leaves() {
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
                assert_eq!(s.plays_of(BURN).len(), 3, "burn at either hero and the grunt — the Mayor is gone");
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
                let s = scenario(json!({
                    "p1": { "hand": [FILLER], "field": [{ "def": MAYOR, "radiant": true }] },
                    "p2": { "hand": [FILLER] },
                }));
                assert_eq!(s.view(P1).random_targets, Some(true));
            }
        }
    }
}
