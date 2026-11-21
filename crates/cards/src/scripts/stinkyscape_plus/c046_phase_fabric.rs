//! SPEC §8.12 S+ #46 Phase Fabric (§6.1 Immutable, Immune to Spells, Evasive; R81, R1420, R3535000,
//! R3543002, R3547000, R3547002, R4353001, R4353002): (1) Spell, Epic, Mindustrial, Mineral, a card of
//! the held Stinkyscape+ set.
//!   Base:    "Give a Unit Immutable, Immune to Spells, Divine Shield and Armor {armor}." — 2.
//!   Radiant: "Give a Unit Immutable, Immune to Spells, Divine Shield, Armor {armor} and Can't be
//!            attacked." — 4.
//!
//! Engine: a declared target, any Unit on either side (R81, R3547000), then `grant_keyword` Immutable
//! and Immune to Spells, and on the Radiant face Evasive, which is "Can't be attacked" as a keyword
//! (R3543002, the issue's reading): no attack may target the Unit, it implies Can't Defend without
//! adding it, and effects still reach it, but for a Spell's or a Field Spell's aim (R4353002). Immune to
//! Spells bars every Spell's pick of the Unit from then on, this card's own among them (E35; Immutable
//! bars no aim, R4353001); Immune to Spells is given last, so the Spell's own remaining grants are not
//! turned away by it (R3547002). Issue #3951 added Divine Shield to
//! both faces; through v0.6.4c neither gave it, which `script_v0_6_4c` keeps for those replays (R767000).
//! Issue #3996 added Armor `armor` to both faces (§6.1: a granted Armor N, stacking with every other
//! source), given with the rest before Immune to Spells; through v0.6.5b neither gave it, which
//! `script_v0_6_5b` keeps for those replays (R767000).

use jackioh_engine::prelude::*;

pub const ID: &str = "stinkyscapeplus-046";

/// R81, R3547000: any Unit on either side, picked with the play.
fn targets() -> Vec<TargetDecl> {
    vec![TargetDecl::target(1, 1, json!({ "side": "any", "of": ["unit"] }))]
}

/// The Radiant face adds Evasive to what the base face gives. Immune to Spells goes on last: once it
/// stands, "a Spell doesn't affect it" (E35) turns away every later effect of this same Spell, so the
/// keywords are given together as R3547002 reads them and the printed order is not observable.
/// `shield` is issue #3951's Divine Shield and `armor` issue #3996's Armor `armor`.
fn phase_fabric(radiant: bool, shield: bool, armor: bool) -> Script {
    Script {
        targets: targets(),
        cry: Some(hook(move |ctx| {
            let mut keywords = vec![json!({ "kind": "Immutable" })];
            if shield {
                keywords.push(json!({ "kind": "Divine Shield" }));
            }
            if armor {
                keywords.push(json!({ "kind": "Armor", "n": param(&*ctx, "armor") }));
            }
            if radiant {
                keywords.push(json!({ "kind": "Evasive" }));
            }
            keywords.push(json!({ "kind": "Immune to Spells" }));
            keywords
                .into_iter()
                .map(|keyword| grant_keyword(json_as(json!({ "target": { "of": "chosen" }, "keyword": keyword }))))
                .collect()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    CardScripts {
        base: phase_fabric(false, true, true),
        radiant: phase_fabric(true, true, true),
    }
}

/// R767000: the scripts through v0.6.5b, which gave no Armor (issue #3996). `KEPT_SCRIPTS` folds those
/// replays under it.
pub fn script_v0_6_5b() -> CardScripts {
    CardScripts {
        base: phase_fabric(false, true, false),
        radiant: phase_fabric(true, true, false),
    }
}

/// R767000: the scripts through v0.6.4c, which gave no Divine Shield (issue #3951). `KEPT_SCRIPTS` folds
/// those replays under it.
pub fn script_v0_6_4c() -> CardScripts {
    CardScripts {
        base: phase_fabric(false, false, false),
        radiant: phase_fabric(true, false, false),
    }
}

// S+ #46 Phase Fabric — SPEC §8.12 row S+ 46 (R81, R3543002, R3547000, R3547002, R4353001).
#[cfg(test)]
mod tests {
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const P1: PlayerId = PlayerId::P1;
    const P2: PlayerId = PlayerId::P2;

    const FABRIC: &str = "stinkyscapeplus-046";
    const CRYO: &str = "stinkyscapeplus-022"; // Cryofluid: a Spell that declares one Unit, for the "no Spell picks it" check.
    const VANILLA: &str = "stand-in (1) 4/4 R12/12";
    const WALL: &str = "stand-in (1) 1/30";
    const ARMORED: &str = "stand-in (4) 7/7 Armor 7";
    const BIG: &str = "stand-in (1) 9/9";
    const FILLER: &str = "core-005";

    fn board(radiant: bool, mine: Value, theirs: Value) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": FABRIC, "radiant": radiant }, CRYO, FILLER], "field": mine, "mana": 8 },
            "p2": { "hand": [FILLER], "field": theirs },
        }))
    }

    fn keyword_count(s: &Scenario, card: &CardInstance, kind: KeywordKind) -> usize {
        s.stats(card).keywords.iter().filter(|keyword| keyword.kind() == kind).count()
    }

    mod base {
        use super::*;

        #[test]
        fn r3547000_it_offers_any_unit_on_either_side_and_no_hero() {
            let s = board(false, json!([VANILLA]), json!([VANILLA]));
            let offered = s.play_targets(FABRIC);
            for unit in [s.unit_at(P1, 1), s.unit_at(P2, 1)] {
                assert!(offered.contains(&Selection::Instance { instance_id: unit.id }));
            }
            assert!(!offered.iter().any(|pick| matches!(pick, Selection::Hero { .. })));
        }

        #[test]
        fn immutable_immune_to_spells_divine_shield_and_armor_2_and_no_evasive() {
            let mut s = board(false, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(FABRIC, pick::targets(pick::instance(&unit)));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Immutable), 1);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::ImmuneToSpells), 1);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::DivineShield), 1);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Armor), 1);
            assert_eq!(s.stats(&unit).armor, 2);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Evasive), 0);
            s.expect_stats(&unit, json!({ "attack": 4, "health": 4 }));
        }

        /// §6.1: the granted Armor stacks with the Armor the Unit prints.
        #[test]
        fn its_armor_stacks_with_printed_armor() {
            let mut s = board(false, json!([ARMORED]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(FABRIC, pick::targets(pick::instance(&unit)));
            assert_eq!(s.stats(&unit).armor, 9);
        }

        /// The Armor takes the hit its Divine Shield no longer stops: a 9-attack hit after the shield
        /// pops deals 9 less 2.
        #[test]
        fn the_armor_softens_a_hit_once_the_divine_shield_is_gone() {
            let mut s = scenario(json!({
                "p1": { "hand": [FABRIC, FILLER], "field": [WALL], "library": crate::filler(4) },
                "p2": { "hand": [FILLER], "field": [BIG], "library": crate::filler(4) },
            }));
            let (mine, theirs) = (s.unit_at(P1, 1), s.unit_at(P2, 1));
            s.play(FABRIC, pick::targets(pick::instance(&mine)));
            s.end_turn();
            s.attack(&theirs, &mine);
            assert_eq!(s.card(&mine).damage, 0, "the Divine Shield took the first hit");
            s.end_turn();
            s.end_turn();
            s.attack(&theirs, &mine);
            assert_eq!(s.card(&mine).damage, 7, "Armor 2 took 2 of the 9");
        }

        #[test]
        fn its_armor_reads_through_param() {
            let mut s = board(false, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            let moved = crate::upgrade_number(&mut s, FABRIC, "armor");
            assert_eq!(moved, 3);
            s.play(FABRIC, pick::targets(pick::instance(&unit)));
            assert_eq!(s.stats(&unit).armor, 3);
        }

        #[test]
        fn no_spell_may_pick_the_unit_afterwards() {
            let mut s = board(false, json!([VANILLA, VANILLA]), json!([]));
            let (fabric, other) = (s.unit_at(P1, 1), s.unit_at(P1, 2));
            assert!(s.play_targets(CRYO).contains(&Selection::Instance { instance_id: fabric.id.clone() }));
            s.play(FABRIC, pick::targets(pick::instance(&fabric)));
            let offered = s.play_targets(CRYO);
            assert!(!offered.contains(&Selection::Instance { instance_id: fabric.id }));
            assert!(offered.contains(&Selection::Instance { instance_id: other.id }));
        }

        #[test]
        fn an_attack_still_reaches_the_unit() {
            let mut s = board(false, json!([VANILLA]), json!([WALL]));
            let (mine, theirs) = (s.unit_at(P1, 1), s.unit_at(P2, 1));
            s.play(FABRIC, pick::targets(pick::instance(&theirs)));
            s.attack(&mine, &theirs);
            assert_eq!(s.card(&theirs).damage, 0, "its Divine Shield took the hit");
            assert_eq!(keyword_count(&s, &theirs, KeywordKind::DivineShield), 0, "the attack reached it");
        }

        /// R767000: through v0.6.4c it gave no Divine Shield (issue #3951), nor any Armor.
        #[test]
        fn r767000_the_script_kept_for_v0_6_4c_gives_no_divine_shield() {
            crate::under_kept(FABRIC, super::super::script_v0_6_4c(), || {
                let mut s = board(false, json!([VANILLA]), json!([]));
                let unit = s.unit_at(P1, 1);
                s.play(FABRIC, pick::targets(pick::instance(&unit)));
                assert_eq!(keyword_count(&s, &unit, KeywordKind::Immutable), 1);
                assert_eq!(keyword_count(&s, &unit, KeywordKind::DivineShield), 0);
                assert_eq!(s.stats(&unit).armor, 0);
            });
        }

        /// R767000: through v0.6.5b it gave Divine Shield and no Armor (issue #3996), on both faces.
        #[test]
        fn r767000_the_script_kept_for_v0_6_5b_gives_divine_shield_and_no_armor() {
            crate::under_kept(FABRIC, super::super::script_v0_6_5b(), || {
                for radiant in [false, true] {
                    let mut s = board(radiant, json!([VANILLA]), json!([]));
                    let unit = s.unit_at(P1, 1);
                    s.play(FABRIC, pick::targets(pick::instance(&unit)));
                    assert_eq!(keyword_count(&s, &unit, KeywordKind::DivineShield), 1);
                    assert_eq!(keyword_count(&s, &unit, KeywordKind::ImmuneToSpells), 1);
                    assert_eq!(keyword_count(&s, &unit, KeywordKind::Armor), 0);
                    assert_eq!(s.stats(&unit).armor, 0);
                    assert_eq!(keyword_count(&s, &unit, KeywordKind::Evasive), usize::from(radiant));
                }
            });
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r3547002_immutable_immune_to_spells_divine_shield_armor_4_and_evasive_all_land() {
            let mut s = board(true, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(FABRIC, pick::targets(pick::instance(&unit)));
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Immutable), 1);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::ImmuneToSpells), 1);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::DivineShield), 1);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Armor), 1);
            assert_eq!(s.stats(&unit).armor, 4);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::Evasive), 1);
            assert_eq!(keyword_count(&s, &unit, KeywordKind::CantDefend), 0, "R3543002: implied, never added");
        }

        #[test]
        fn r3543002_no_attack_may_target_it_afterwards() {
            let mut s = board(true, json!([VANILLA]), json!([WALL]));
            let (mine, theirs) = (s.unit_at(P1, 1), s.unit_at(P2, 1));
            s.play(FABRIC, pick::targets(pick::instance(&theirs)));
            s.expect_refused_with(|s| s.attack(&mine, &theirs), "cannot be attacked");
            assert_eq!(s.card(&theirs).damage, 0);
        }

        #[test]
        fn no_spell_may_pick_the_unit_afterwards() {
            let mut s = board(true, json!([VANILLA]), json!([]));
            let unit = s.unit_at(P1, 1);
            s.play(FABRIC, pick::targets(pick::instance(&unit)));
            assert!(!s.play_targets(CRYO).contains(&Selection::Instance { instance_id: unit.id }));
        }
    }
}
