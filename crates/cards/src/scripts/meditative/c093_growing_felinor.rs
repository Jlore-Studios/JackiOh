//! M #93 Growing Felinor (SPEC §8.8 row 93): (1) Unit, Felinor, Common, 1/1 → 2/2.
//!
//! Base:    "Can't be in Defense Position.
//!           Death: Summon a Growing Felinor Sr."
//! Radiant: "Divine Shield, Rush
//!           Can't be in Defense Position.
//!           Death: Summon a Radiant Growing Felinor Sr."
//! Engine:
//! - **The flag:** `never_defense`, as #65.1 Spikey Pillow (`core-065-1`): `reduce.rs` refuses the switch
//!   and `legal_actions` leaves it out, and `combat.rs` checks a switch made as an effect (§6.1).
//! - **The Death:** summons the next size up, `meditative-093-1`, into the leftmost open unit zone
//!   (R64): a fresh card, on its Radiant face when this is the Radiant face (R1180). With no zone open it
//!   summons nothing. A named token, so no pool and no R387.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-093";

/// The next size up the chain, which this card's Death summons (R1180).
const NEXT: &str = "meditative-093-1";

fn growing(radiant: bool) -> Script {
    Script {
        static_flags: Some(StaticFlags {
            never_defense: Some(true),
            ..StaticFlags::default()
        }),
        death: Some(hook(move |_ctx| {
            vec![summon(json_as(
                json!({ "defId": NEXT, "player": "self", "radiant": radiant }),
            ))]
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: growing(false),
        radiant: growing(true),
    }
}

// M #93 Growing Felinor — SPEC §8.8 row 93, BUILD M10 row M 93: "It can't switch to Defense; its
// Death summons a base Growing Felinor Sr (M 93.1) into your leftmost open zone (MD-E19), none at
// a full board; radiant 2/2, Divine Shield, Rush, and the Sr is Radiant".
#[cfg(test)]
mod tests {
    use super::{ID, NEXT, script};
    use jackioh_engine::layers::keywords_of;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const VANILLA: &str = "core-008"; // Mr. Vanilla, 4/4, no text

    /// p1 holds `hits` Hit Jobs and the mana to cast them, with `field` on its side.
    fn game(field: Value, hits: usize) -> Scenario {
        crate::scenario(json!({
            "p1": { "field": field, "hand": vec![HIT_JOB; hits], "mana": 30 },
        }))
    }

    fn hit(s: &mut Scenario, id: &str) {
        s.play(
            HIT_JOB,
            json!({ "targets": [{ "pick": "instance", "instanceId": id }] }),
        );
    }

    fn unit_at(s: &Scenario, lane: i32) -> CardInstance {
        s.unit(P1, lane)
            .unwrap_or_else(|| panic!("expected a p1 unit in lane {lane}"))
    }

    fn row_defs(s: &Scenario) -> Vec<Option<String>> {
        (1..=5)
            .map(|lane| s.unit(P1, lane).map(|unit| unit.def_id))
            .collect()
    }

    mod m93_growing_felinor {
        use super::*;

        #[test]
        fn is_a_1_cost_1_1_felinor_and_radiant_a_2_2_with_divine_shield_and_rush() {
            let def = crate::card_def(ID);
            assert_eq!(def.cost, CardCost::Fixed(1));
            assert!(!def.token);
            assert_eq!(def.rarity, Rarity::Common);
            assert_eq!([def.base.attack, def.base.health], [Some(1), Some(1)]);
            assert_eq!([def.radiant.attack, def.radiant.health], [Some(2), Some(2)]);
            assert!(def.base.keywords.is_empty());
            assert_eq!(def.radiant.keywords, vec![Keyword::DivineShield, Keyword::Rush]);
            let scripts = script();
            assert_eq!(
                scripts.base.static_flags.and_then(|flags| flags.never_defense),
                Some(true)
            );
            assert_eq!(
                scripts.radiant.static_flags.and_then(|flags| flags.never_defense),
                Some(true)
            );
        }

        mod base {
            use super::*;

            #[test]
            fn s4_1_it_can_never_switch_to_defense_position() {
                let mut s = game(json!([ID, VANILLA]), 0);
                let card = unit_at(&s, 1);
                s.expect_stats(&card, json!({ "attack": 1, "maxHealth": 1 }));
                s.expect_refused_with(|s| s.switch_position(&card), "Defense Position");
                assert_eq!(s.card(&card).position.unwrap_or(Position::Atk), Position::Atk);
                // The flag is the card's alone: the Vanilla beside it still switches.
                let vanilla = unit_at(&s, 2);
                s.switch_position(&vanilla);
                assert_eq!(s.card(&vanilla).position, Some(Position::Def));
            }

            #[test]
            fn r1180_r64_its_death_summons_a_base_sr_into_the_leftmost_open_zone() {
                let mut s = game(json!([{ "def": ID, "lane": 3 }]), 1);
                let dying = unit_at(&s, 3);
                hit(&mut s, &dying.id);
                s.expect_in_zone(&dying.id, "graveyard");
                let next = unit_at(&s, 1);
                assert_eq!(next.def_id, NEXT);
                assert!(!next.radiant);
                s.expect_stats(&next, json!({ "attack": 2, "maxHealth": 2 }));
                assert!(keywords_of(s.state(), &next).is_empty());
                assert_eq!(row_defs(&s), vec![Some(NEXT.to_string()), None, None, None, None]);
            }

            #[test]
            fn r64_a_full_board_summons_nothing() {
                // The card tops a pile on lane 1; when it dies the Vanilla under it resumes there (§3.2).
                let mut s = game(
                    json!([VANILLA, { "def": ID, "stack": true }, VANILLA, VANILLA, VANILLA, VANILLA]),
                    1,
                );
                let dying = unit_at(&s, 1);
                assert_eq!(dying.def_id, ID);
                hit(&mut s, &dying.id);
                s.expect_in_zone(&dying.id, "graveyard");
                assert!(!row_defs(&s).contains(&Some(NEXT.to_string())));
                assert_eq!(unit_at(&s, 1).def_id, VANILLA);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn s4_1_the_radiant_face_cannot_switch_to_defense_either() {
                let mut s = game(json!([{ "def": ID, "radiant": true }]), 0);
                let card = unit_at(&s, 1);
                s.expect_stats(&card, json!({ "attack": 2, "maxHealth": 2 }));
                s.expect_refused_with(|s| s.switch_position(&card), "Defense Position");
            }

            #[test]
            fn r1180_radiant_its_death_summons_a_radiant_sr_with_divine_shield_and_rush() {
                let mut s = game(json!([{ "def": ID, "radiant": true, "lane": 3 }]), 1);
                let dying = unit_at(&s, 3);
                hit(&mut s, &dying.id);
                let next = unit_at(&s, 1);
                assert_eq!(next.def_id, NEXT);
                assert!(next.radiant);
                s.expect_stats(&next, json!({ "attack": 4, "maxHealth": 4 }));
                assert_eq!(
                    keywords_of(s.state(), &next),
                    vec![Keyword::DivineShield, Keyword::Rush]
                );
                assert_eq!(unit_at(&s, 1).id, next.id);
                assert!(s.unit(P1, 3).is_none());
            }
        }
        mod the_chain {
            use super::*;

            /// Hits the card in lane 1 and returns what stands there after.
            fn grow(s: &mut Scenario) -> Option<CardInstance> {
                let id = unit_at(s, 1).id;
                hit(s, &id);
                s.unit(P1, 1)
            }

            #[test]
            fn r1180_the_chain_grows_one_size_each_death_and_super_senior_ends_it() {
                let mut s = game(json!([ID]), 4);
                for (def, size) in [
                    ("meditative-093-1", 2),
                    ("meditative-093-2", 3),
                    ("meditative-093-3", 4),
                ] {
                    let next = grow(&mut s).unwrap_or_else(|| panic!("expected {def}"));
                    assert_eq!(next.def_id, def);
                    assert!(!next.radiant);
                    s.expect_stats(&next, json!({ "attack": size, "maxHealth": size }));
                }
                // Super Senior ends it: nothing stands and nothing is left to summon.
                assert!(grow(&mut s).is_none());
                assert_eq!(row_defs(&s), vec![None, None, None, None, None]);
            }

            #[test]
            fn r1180_the_radiant_chain_stays_radiant_up_to_super_senior() {
                let mut s = game(json!([{ "def": ID, "radiant": true }]), 4);
                for (def, size) in [
                    ("meditative-093-1", 4),
                    ("meditative-093-2", 6),
                    ("meditative-093-3", 8),
                ] {
                    let next = grow(&mut s).unwrap_or_else(|| panic!("expected {def}"));
                    assert_eq!(next.def_id, def);
                    assert!(next.radiant);
                    s.expect_stats(&next, json!({ "attack": size, "maxHealth": size }));
                    assert_eq!(
                        keywords_of(s.state(), &next),
                        vec![Keyword::DivineShield, Keyword::Rush]
                    );
                }
                assert!(grow(&mut s).is_none());
            }
        }
    }
}
