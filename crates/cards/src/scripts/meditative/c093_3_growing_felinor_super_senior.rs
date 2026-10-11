//! M #93.3 Growing Felinor Super Senior (SPEC §8.8 row 93.3, SPEC §7 token): (1) Unit, Felinor, Token
//! (printed Common), 4/4 → 8/8.
//!
//! Base:    "Can't be in Defense Position."
//! Radiant: "Divine Shield, Rush
//!           Can't be in Defense Position."
//! Engine: keywords and the flag only, the end of the Growing Felinor chain (R1180).
//! - **The flag:** `never_defense`, as #65.1 Spikey Pillow (`core-065-1`): `reduce.rs` refuses the switch
//!   and `legal_actions` leaves it out, and `combat.rs` checks a switch made as an effect (§6.1).
//! - **No Death:** a unit token vanishes when it leaves the field (R11), and nothing follows it.

use jackioh_engine::prelude::*;

pub const ID: &str = "meditative-093-3";

pub fn script() -> CardScripts {
    let base = Script {
        static_flags: Some(StaticFlags {
            never_defense: Some(true),
            ..StaticFlags::default()
        }),
        ..Script::default()
    };
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// M #93.3 Growing Felinor Super Senior — SPEC §8.8 row 93.3, BUILD M10 row M 93.3: "It can't switch
// to Defense; it summons nothing when it dies, ending the chain (MD-E19); radiant 8/8, Divine Shield,
// Rush".
#[cfg(test)]
mod tests {
    use super::{ID, script};
    use jackioh_engine::layers::keywords_of;
    use jackioh_engine::testkit::*;

    const P1: PlayerId = PlayerId::P1;

    const HIT_JOB: &str = "core-016"; // (3) Spell: Destroy a Unit.
    const VANILLA: &str = "core-008"; // Mr. Vanilla, 4/4, no text

    fn game(field: Value, hits: usize) -> Scenario {
        crate::scenario(json!({
            "p1": { "field": field, "hand": vec![HIT_JOB; hits], "mana": 30 },
        }))
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

    fn hit(s: &mut Scenario, id: &str) {
        s.play(
            HIT_JOB,
            json!({ "targets": [{ "pick": "instance", "instanceId": id }] }),
        );
    }

    mod m93_3_growing_felinor_super_senior {
        use super::*;

        #[test]
        fn is_a_1_cost_token_4_4_felinor_and_radiant_8_8_with_divine_shield_and_rush() {
            let def = crate::card_def(ID);
            assert_eq!(def.cost, CardCost::Fixed(1));
            assert!(def.token);
            assert_eq!(def.rarity, Rarity::Token);
            assert_eq!(def.printed_rarity, Some(PrintedRarity::Common));
            assert_eq!([def.base.attack, def.base.health], [Some(4), Some(4)]);
            assert_eq!([def.radiant.attack, def.radiant.health], [Some(8), Some(8)]);
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
            assert!(scripts.base.death.is_none());
            assert!(scripts.radiant.death.is_none());
        }

        mod base {
            use super::*;

            #[test]
            fn s4_1_it_can_never_switch_to_defense_position() {
                let mut s = game(json!([ID, VANILLA]), 0);
                let card = unit_at(&s, 1);
                s.expect_stats(&card, json!({ "attack": 4, "maxHealth": 4 }));
                assert!(keywords_of(s.state(), &card).is_empty());
                s.expect_refused_with(|s| s.switch_position(&card), "Defense Position");
                assert_eq!(s.card(&card).position.unwrap_or(Position::Atk), Position::Atk);
                // The flag is the card's alone: the Vanilla beside it still switches.
                let vanilla = unit_at(&s, 2);
                s.switch_position(&vanilla);
                assert_eq!(s.card(&vanilla).position, Some(Position::Def));
            }

            #[test]
            fn r1180_r11_it_vanishes_and_summons_nothing_ending_the_chain() {
                let mut s = game(json!([{ "def": ID, "lane": 3 }]), 1);
                let dying = unit_at(&s, 3);
                hit(&mut s, &dying.id);
                s.expect_in_zone(&dying.id, "gone");
                assert_eq!(row_defs(&s), vec![None, None, None, None, None]);
            }
        }

        mod radiant {
            use super::*;

            #[test]
            fn s4_1_the_radiant_face_cannot_switch_to_defense_either() {
                let mut s = game(json!([{ "def": ID, "radiant": true }]), 0);
                let card = unit_at(&s, 1);
                s.expect_stats(&card, json!({ "attack": 8, "maxHealth": 8 }));
                assert_eq!(
                    keywords_of(s.state(), &card),
                    vec![Keyword::DivineShield, Keyword::Rush]
                );
                s.expect_refused_with(|s| s.switch_position(&card), "Defense Position");
            }

            #[test]
            fn r1180_it_summons_nothing_either() {
                let mut s = game(json!([{ "def": ID, "radiant": true, "lane": 3 }]), 1);
                let dying = unit_at(&s, 3);
                hit(&mut s, &dying.id);
                s.expect_in_zone(&dying.id, "gone");
                assert_eq!(row_defs(&s), vec![None, None, None, None, None]);
            }
        }
    }
}
