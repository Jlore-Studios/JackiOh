//! SPEC §8.14 FvH #44 Felinor Converter ((2) Unit, Felinor, Rare, 5/4 → 10/8), a card of the held Felinors vs
//! Humans set (R5235000).
//!   Base:    "Rush\nCry: Transform another friendly Unit into a copy of this."
//!   Radiant: "Rush\nCry: Summon a copy of this."
//!
//! Cry (R1). Base: one other friendly Unit declared with the play (R81), aimed as help, transformed into a
//! copy of this (`transform_into_copy`, R4592007, R57) in the same zone, keeping its place, running no Cry
//! (R691); with no other friendly Unit nothing is offered and the Cry does nothing (R90). Radiant:
//! `summon_copy` of itself into the leftmost open unit zone (R64, R57), running no Cry (R1). Rush is catalog
//! data. Declares no numbers. Built by S07 (#5257).

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-044";

pub fn script() -> CardScripts {
    let base = Script {
        targets: vec![TargetDecl {
            aim: Some(TargetAim::Help),
            ..TargetDecl::target(1, 1, json!({ "side": "ally", "of": ["unit"], "excludeSelf": true }))
        }],
        cry: Some(hook(|_ctx| {
            vec![transform_into_copy(TransformIntoCopyArgs {
                target: TargetSpec::Chosen { index: None },
                of: TargetSpec::SelfCard,
                radiant: None,
            })]
        })),
        ..Script::default()
    };
    let radiant = Script {
        cry: Some(hook(|_ctx| vec![summon_copy(json_as(json!({ "of": { "of": "self" } })))])),
        ..Script::default()
    };
    CardScripts { base, radiant }
}

// FvH #44 Felinor Converter — SPEC §8.14 row 44.
#[cfg(test)]
mod tests {
    use super::ID;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use crate::scenario;

    const ALLY: &str = "stand-in (1) 2/2";
    const FILLER: &str = "core-005"; // Stockpile: a card to hold

    fn board(radiant: bool, mine: &[&str], theirs: &[&str]) -> Scenario {
        scenario(json!({
            "p1": { "hand": [{ "def": ID, "radiant": radiant }, FILLER], "field": mine, "library": crate::filler(4) },
            "p2": { "hand": [FILLER], "field": theirs, "library": crate::filler(4) },
        }))
    }

    fn converters(s: &Scenario) -> Vec<CardInstance> {
        active_units_of(s.state(), P1)
            .into_iter()
            .filter(|unit| unit.def_id == ID)
            .cloned()
            .collect()
    }

    mod base {
        use super::*;

        #[test]
        fn r4592007_transforms_the_chosen_ally_into_a_copy_of_this_in_its_zone() {
            let mut s = board(false, &[ALLY, ALLY], &[ALLY]);
            let chosen = s.unit_at(P1, 1);
            let spared = s.unit_at(P1, 2);
            s.play(ID, json!({ "zone": 3, "targets": pick::instance(&chosen) }));
            let copy = s.unit_at(P1, 1);
            assert_eq!(copy.def_id, ID, "the ally's zone holds a Converter now");
            assert!(!copy.radiant);
            assert_eq!(s.count(GameEventType::Transformed), 1, "the copy runs no Cry of its own");
            assert_eq!(s.unit_at(P1, 2).id, spared.id, "the other ally is untouched");
            assert_eq!(s.unit_at(P1, 3).def_id, ID, "the Converter itself");
            assert_eq!(s.unit_at(P2, 1).def_id, ALLY, "the enemy is untouched");
            s.expect_stats(&copy, json!({ "attack": 5, "health": 4 }));
        }

        #[test]
        fn r81_only_another_friendly_unit_is_offered() {
            let s = board(false, &[ALLY], &[ALLY]);
            let ally = s.unit_at(P1, 1).id;
            let offered = pick::instance_ids(&s.play_targets(ID));
            let offered = nonempty(offered, "the Converter's offered targets");
            for id in &offered {
                assert_eq!(id, &ally, "only the friendly Unit");
            }
        }

        #[test]
        fn r90_with_no_other_friendly_unit_it_enters_and_the_cry_does_nothing() {
            let mut s = board(false, &[], &[ALLY]);
            s.play(ID, json!({}));
            assert_eq!(converters(&s).len(), 1);
            assert_eq!(s.count(GameEventType::Transformed), 0);
            assert_eq!(s.unit_at(P2, 1).def_id, ALLY);
        }

        #[test]
        fn rush_reaches_play() {
            let mut s = board(false, &[], &[ALLY]);
            s.play(ID, json!({}));
            let me = must(converters(&s).into_iter().next(), "the Converter");
            assert!(s.stats(&me).keywords.iter().any(|keyword| keyword.kind() == KeywordKind::Rush));
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r57_its_cry_summons_a_radiant_copy_of_this_that_runs_no_cry() {
            let mut s = board(true, &[], &[ALLY]);
            s.play(ID, json!({}));
            let copies = nonempty(converters(&s), "the Converters on p1's field");
            assert_eq!(copies.len(), 2, "itself and one copy, which summons no other");
            for copy in &copies {
                assert!(copy.radiant, "R57: the copy keeps the Radiant flag");
            }
        }

        #[test]
        fn it_declares_no_target_and_leaves_allies_alone() {
            let mut s = board(true, &[ALLY], &[]);
            assert!(s.play_targets(ID).is_empty());
            s.play(ID, json!({}));
            assert_eq!(s.unit_at(P1, 1).def_id, ALLY);
            assert_eq!(s.count(GameEventType::Transformed), 0);
        }
    }
}
